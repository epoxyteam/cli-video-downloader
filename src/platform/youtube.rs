use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use std::process::Stdio;
use tokio::sync::watch;
use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt};
use url::Url;
use regex::Regex;

use crate::{Error, Result};
use super::{Platform, VideoFormat, VideoInfo, Quality, Format};

#[derive(Default)]
pub struct YouTube {}

impl YouTube {
    async fn extract_video_id(&self, url: &Url) -> Result<String> {
        if url.host_str() == Some("youtu.be") {
            return url.path_segments()
                .and_then(|segments| segments.last())
                .map(|s| s.to_string())
                .ok_or_else(|| Error::InvalidUrl("Invalid YouTube short URL".into()));
        }

        url.query_pairs()
            .find(|(key, _)| key == "v")
            .map(|(_, value)| value.to_string())
            .ok_or_else(|| Error::InvalidUrl("No video ID found in URL".into()))
    }

    async fn get_format_info(&self, video_id: &str) -> Result<Vec<VideoFormat>> {
        // Run yt-dlp to get available formats
        let output = Command::new("yt-dlp")
            .args([
                "-F",
                "--format-sort", "hasvid+hasaud,res,fps,codec:h264",
                format!("https://www.youtube.com/watch?v={}", video_id).as_str()
            ])
            .output()
            .await
            .map_err(|e| Error::Platform(format!("Failed to execute yt-dlp: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut formats = Vec::new();
        
        // Find formats with video (possibly combined with audio)
        let mut found_any_format = false;
        
        // Process all lines of output
        for line in stdout.lines() {
            // Skip header lines and separator lines
            if line.starts_with("ID") || line.starts_with("--") || line.trim().is_empty() {
                continue;
            }
            
            // Parse format information - use multiple spaces as separator
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue; // Not enough parts to be a format line
            }
            
            // Extract basic fields (ID and extension are always in the same position)
            let id = parts[0].to_string();
            let ext = parts[1].to_string();
            
            // Check if this is an audio-only format
            if line.contains("audio only") {
                continue; // Skip audio-only formats
            }
            
            // Determine quality by looking for resolution patterns
            let quality = if line.contains("1920x1080") || line.contains("1080p") {
                Quality::HD1080
            } else if line.contains("1280x720") || line.contains("720p") {
                Quality::HD720
            } else if line.contains("854x480") || line.contains("480p") {
                Quality::Medium
            } else if line.contains("640x360") || line.contains("360p") {
                Quality::Low
            } else if line.contains("426x240") || line.contains("240p") {
                Quality::Low
            } else if line.contains("2160") {
                Quality::UHD2160
            } else {
                // Default to medium quality if we can't determine
                Quality::Medium
            };
            
            // Determine format based on extension
            let format = match ext.as_str() {
                "mp4" => Format::MP4,
                "webm" => Format::WebM,
                "mov" => Format::MOV,
                _ => Format::Other(ext.clone()),
            };
            
            // Only add if it's not audio-only (double check)
            if !line.contains("audio only") {
                formats.push(VideoFormat {
                    id,
                    quality,
                    format,
                    file_size: None,
                });
                found_any_format = true;
            }
        }
        
        // Always add a "best" format that lets yt-dlp choose
        formats.push(VideoFormat {
            id: "best".to_string(),
            quality: Quality::HD1080, // Assume high quality for "best"
            format: Format::MP4,      // Assume MP4 for "best"
            file_size: None,
        });
        
        println!("Found {} formats", formats.len());
        
        // Even if no specific formats were found, we added "best"
        Ok(formats)
    }
}

#[async_trait]
impl Platform for YouTube {
    fn name(&self) -> &'static str {
        "YouTube"
    }

    fn supports_url(&self, url: &Url) -> bool {
        url.host_str()
            .map(|host| host.ends_with("youtube.com") || host.ends_with("youtu.be"))
            .unwrap_or(false)
    }

    async fn extract_info(&self, url: &Url) -> Result<VideoInfo> {
        let video_id = self.extract_video_id(url).await?;
        
        let output = Command::new("yt-dlp")
            .args(["--dump-json", "--no-playlist"])
            .arg(format!("https://www.youtube.com/watch?v={}", video_id))
            .output()
            .await
            .map_err(|e| Error::Platform(format!("Failed to execute yt-dlp: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let info: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| Error::Platform(format!("Failed to parse video info: {}", e)))?;

        let title = info["title"].as_str()
            .unwrap_or("Unknown Title")
            .to_string();

        let description = info["description"].as_str()
            .map(|s| s.to_string());

        let duration = info["duration"].as_f64()
            .map(|d| d as u64);

        let formats = self.get_format_info(&video_id).await?;

        Ok(VideoInfo {
            url: url.clone(),
            title,
            description,
            duration,
            formats,
        })
    }

    async fn download_video(&self, info: &VideoInfo, format_id: &str, output_path: &Path, progress_tx: Arc<watch::Sender<f64>>) -> Result<()> {
        let output_str = output_path.to_str()
            .ok_or_else(|| Error::Platform("Invalid output path".to_string()))?;

        // Use format ID with best audio
        // format_id:format_id+bestaudio ensures video has audio
        let format_spec = format!("{}+bestaudio/best", format_id);

        let mut child = Command::new("yt-dlp")
            .args([
                "-f", &format_spec,
                "--merge-output-format", "mp4",  // Always merge to MP4
                "--newline",
                "--no-part",  // Don't create temporary .part files
                "-o", output_str,
            ])
            .arg(info.url.as_str())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Platform(format!("Failed to start yt-dlp: {}", e)))?;

        let progress_re = Regex::new(r"\[download\]\s+(\d+\.\d+)%").unwrap();
        
        if let Some(stderr) = child.stderr.take() {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(caps) = progress_re.captures(&line) {
                    if let Ok(progress) = caps[1].parse::<f64>() {
                        let _ = progress_tx.send(progress / 100.0);
                    }
                }
            }
        }

        let status = child.wait()
            .await
            .map_err(|e| Error::Platform(format!("Failed to complete download: {}", e)))?;

        if !status.success() {
            return Err(Error::Platform("Download failed".to_string()));
        }

        Ok(())
    }
}
