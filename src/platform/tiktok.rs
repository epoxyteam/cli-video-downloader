use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use std::process::Stdio;
use tokio::sync::watch;
use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt};
use url::Url;
use regex::Regex;
use serde::Deserialize;

use crate::{Error, Result};
use super::{Platform, VideoFormat, VideoInfo, Quality, Format};

/// TikTok-specific video metadata
#[derive(Debug, Deserialize)]
struct TikTokMetadata {
    title: String,
    description: Option<String>,
    duration: Option<f64>,
    #[serde(rename = "id")]
    video_id: String,
}

#[derive(Default)]
pub struct TikTok {}

impl TikTok {
    /// Extract TikTok video ID from URL
    async fn extract_video_id(&self, url: &Url) -> Result<String> {
        // Handle VM.TikTok.com links
        if url.host_str() == Some("vm.tiktok.com") {
            return url.path_segments()
                .and_then(|segments| segments.last())
                .map(|s| s.to_string())
                .ok_or_else(|| Error::InvalidUrl("Invalid TikTok short URL".into()));
        }

        // Handle standard tiktok.com links
        if let Some(host) = url.host_str() {
            if host.ends_with("tiktok.com") {
                if let Some(segments) = url.path_segments() {
                    let segments: Vec<_> = segments.collect();
                    // Common TikTok URL format: /username/video/videoId
                    if segments.len() >= 3 && segments[1] == "video" {
                        return Ok(segments[2].to_string());
                    }
                }
            }
        }

        Err(Error::InvalidUrl("No video ID found in URL".into()))
    }

    /// Get available video formats using yt-dlp
    async fn get_format_info_internal(&self, url: &Url) -> Result<Vec<VideoFormat>> {
        // Run yt-dlp to get available formats
        // Use the original URL instead of reconstructing it
        let output = Command::new("yt-dlp")
            .args([
                "-F",
                "--format-sort", "res,fps,codec:h264",
                url.as_str()
            ])
            .output()
            .await
            .map_err(|e| Error::CommandExecution {
                command: "yt-dlp".to_string(),
                reason: e.to_string()
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::CommandExecution {
                command: "yt-dlp".to_string(),
                reason: stderr.to_string()
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut formats = Vec::new();
        
        // Process all lines of output
        for line in stdout.lines() {
            // Skip header lines and separator lines
            if line.starts_with("ID") || line.starts_with("--") || line.trim().is_empty() {
                continue;
            }
            
            // Parse format information
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue; // Not enough parts to be a format line
            }
            
            // Extract basic fields
            let id = parts[0].to_string();
            let ext = parts[1].to_string();
            
            // Skip audio-only formats
            if line.contains("audio only") {
                continue;
            }
            
            // Determine quality from resolution
            let quality = self.determine_quality(line);
            
            // Determine format based on extension
            let format = match ext.as_str() {
                "mp4" => Format::MP4,
                "webm" => Format::WebM,
                "mov" => Format::MOV,
                _ => Format::Other(ext.clone()),
            };
            
            formats.push(VideoFormat {
                id,
                quality,
                format,
                file_size: None,
            });
        }
        
        // Always add a "best" format
        formats.push(VideoFormat {
            id: "best".to_string(),
            quality: Quality::HD1080,
            format: Format::MP4,
            file_size: None,
        });
        
        if formats.len() <= 1 {
            return Err(Error::NoSuitableFormats);
        }
        
        Ok(formats)
    }
    
    /// Helper to determine video quality from format description
    fn determine_quality(&self, format_line: &str) -> Quality {
        if format_line.contains("1920x1080") || format_line.contains("1080p") {
            Quality::HD1080
        } else if format_line.contains("1280x720") || format_line.contains("720p") {
            Quality::HD720
        } else if format_line.contains("854x480") || format_line.contains("480p") {
            Quality::Medium
        } else if format_line.contains("640x360") || format_line.contains("360p") {
            Quality::Low
        } else {
            // Default to medium quality if we can't determine
            Quality::Medium
        }
    }
    
    /// Get video metadata using yt-dlp
    async fn fetch_metadata(&self, url: &str) -> Result<TikTokMetadata> {
        let output = Command::new("yt-dlp")
            .args(["--dump-json", "--no-playlist"])
            .arg(&url)
            .output()
            .await
            .map_err(|e| Error::CommandExecution {
                command: "yt-dlp".to_string(),
                reason: e.to_string()
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::CommandExecution {
                command: "yt-dlp".to_string(),
                reason: stderr.to_string()
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let metadata: TikTokMetadata = serde_json::from_str(&stdout)
            .map_err(|e| Error::OutputParsing(format!("Failed to parse video metadata: {}", e)))?;
            
        Ok(metadata)
    }
}

#[async_trait]
impl Platform for TikTok {
    fn name(&self) -> &'static str {
        "TikTok"
    }

    fn supports_url(&self, url: &Url) -> bool {
        url.host_str()
            .map(|host| host.ends_with("tiktok.com") || host == "vm.tiktok.com")
            .unwrap_or(false)
    }

    async fn extract_info(&self, url: &Url) -> Result<VideoInfo> {
        let metadata = self.fetch_metadata(url.as_str()).await?;
        let formats = self.get_format_info_internal(url).await?;

        Ok(VideoInfo {
            url: url.clone(),
            title: metadata.title,
            description: metadata.description,
            duration: metadata.duration.map(|d| d as u64),
            formats,
        })
    }

    async fn download_video(&self, info: &VideoInfo, format_id: &str, output_path: &Path, progress_tx: Arc<watch::Sender<f64>>) -> Result<()> {
        let output_str = output_path.to_str()
            .ok_or_else(|| Error::InvalidOutputPath(output_path.to_path_buf()))?;

        // Check for ffmpeg availability first
        let ffmpeg_available = Command::new("ffmpeg")
            .arg("-version")
            .output()
            .await
            .is_ok();

        // If ffmpeg is not available, display a warning
        if !ffmpeg_available {
            eprintln!("Warning: ffmpeg is not installed. Will try to download a format with both video and audio.");
            eprintln!("For best results, please install ffmpeg and add it to your PATH.");
        }

        // Determine the format specification based on ffmpeg availability and requested format
        let format_spec = if ffmpeg_available {
            // If ffmpeg is available, we can download video and audio separately and merge
            if format_id == "best" {
                "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best".to_string()
            } else {
                format!("{}+bestaudio/best", format_id)
            }
        } else {
            // If ffmpeg is not available, we must use formats that already include both video and audio
            if format_id == "best" {
                // Get best available format with both video and audio
                "best[ext=mp4]/best".to_string()
            } else {
                // For specific format IDs, we need to find a suitable alternative with audio included
                match info.formats.iter().find(|f| f.id == format_id) {
                    Some(format) => {
                        let quality_label = match format.quality {
                            Quality::UHD2160 => "2160",
                            Quality::HD1080 => "1080",
                            Quality::HD720 => "720",
                            Quality::High => "720",  // Treat 'High' as equivalent to 720p
                            Quality::Medium => "480",
                            Quality::Low => "360",
                            Quality::Custom(ref s) => s,
                        };
                        // Look for formats with audio included that match the quality
                        format!("best[height<={}][ext=mp4]/best", quality_label)
                    },
                    None => "best[ext=mp4]/best".to_string()
                }
            }
        };

        // Store error output for better diagnostics
        let mut error_output = String::new();

        // Prepare the yt-dlp command
        let mut cmd = Command::new("yt-dlp");
        
        if ffmpeg_available {
            cmd.args(["-f", &format_spec, "--merge-output-format", "mp4"]);
        } else {
            cmd.args(["-f", &format_spec]);
        }

        // Add additional options for consistent progress reporting
        cmd.args([
            "--newline",
            "--no-part",
            "--no-colors",
            "--quiet",
            "--progress",
            "--progress-template", "[download] %(progress._percent_str)s"
        ]);
        
        if ffmpeg_available {
            cmd.args(["--merge-output-format", "mp4"]);
        } else {
            cmd.arg("--no-check-formats");
        }
        
        // Add output path and URL
        cmd.args(["-o", output_str, info.url.as_str()]);
        
        // Execute the command
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::CommandExecution {
                command: "yt-dlp".to_string(),
                reason: e.to_string()
            })?;

        let progress_re = Regex::new(r"\[download\]\s+(\d+\.\d+)%.*").unwrap();
        
        // Process stderr for progress and error information
        if let Some(stderr) = child.stderr.take() {
            let mut reader = BufReader::new(stderr).lines();
            
            while let Ok(Some(line)) = reader.next_line().await {
                // Print debug information
                if line.contains("Downloading") || line.contains("Merging") {
                    eprintln!("{}", line);
                }
                
                // Save error output for diagnostics but filter out common warnings
                if !line.contains("WARNING:") && !line.contains("[debug]") {
                    error_output.push_str(&line);
                    error_output.push('\n');
                }
                
                // Extract progress information
                if let Some(caps) = progress_re.captures(&line) {
                    if let Ok(progress) = caps[1].parse::<f64>() {
                        let _ = progress_tx.send(progress / 100.0);
                    }
                }
            }
        }

        let status = child.wait()
            .await
            .map_err(|e| Error::CommandExecution {
                command: "yt-dlp".to_string(),
                reason: e.to_string()
            })?;

        // Check if download produced any output file
        if !output_path.exists() {
            return Err(Error::DownloadFailed {
                reason: format!("Download failed: Output file not created. {}",
                    if !ffmpeg_available {
                        "ffmpeg is required for merging video and audio. Please install ffmpeg."
                    } else {
                        &error_output
                    })
            });
        }

        // Even if exit code is non-zero, if we have an output file, consider it a success
        if !status.success() {
            eprintln!("Warning: TikTok downloader process exited with non-zero status.");
            if !ffmpeg_available {
                eprintln!("If the video has no audio, please install ffmpeg and try again.");
            }
        }

        Ok(())
    }
}