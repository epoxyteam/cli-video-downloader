use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use std::process::Stdio;
use tokio::sync::watch;
use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt};
use url::Url;
use regex::Regex;

use crate::{Error, Result, VideoFormat, VideoInfo, Quality, Format};
use super::Platform;

pub struct YouTube {
    client: reqwest::Client,
}

impl YouTube {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn extract_video_id(&self, url: &Url) -> Result<String> {
        // Handle youtu.be URLs
        if url.host_str() == Some("youtu.be") {
            return url.path_segments()
                .and_then(|segments| segments.last())
                .map(|s| s.to_string())
                .ok_or_else(|| Error::InvalidUrl("Invalid YouTube short URL".into()));
        }

        // Handle youtube.com URLs
        url.query_pairs()
            .find(|(key, _)| key == "v")
            .map(|(_, value)| value.to_string())
            .ok_or_else(|| Error::InvalidUrl("No video ID found in URL".into()))
    }

    async fn get_format_info(&self, video_id: &str) -> Result<Vec<VideoFormat>> {
        // Get video formats using yt-dlp
        let output = Command::new("yt-dlp")
            .arg("-F")
            .arg(format!("https://www.youtube.com/watch?v={}", video_id))
            .output()
            .await
            .map_err(|e| Error::Platform(format!("Failed to execute yt-dlp: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut formats = Vec::new();

        // Parse yt-dlp format list output
        // Format pattern: ID EXT RESOLUTION FPS ...
        let format_re = Regex::new(r"(\d+)\s+(\w+)\s+(\d+x\d+|audio only)").unwrap();
        
        for line in stdout.lines() {
            if let Some(caps) = format_re.captures(line) {
                let id = caps[1].to_string();
                let ext = &caps[2];
                let resolution = &caps[3];
                
                let format = match ext {
                    "mp4" => Format::MP4,
                    "webm" => Format::WebM,
                    "mov" => Format::MOV,
                    _ => Format::Other(ext.to_string()),
                };

                let quality = if resolution == "audio only" {
                    Quality::Low
                } else if resolution.contains('x') {
                    let height = resolution.split('x')
                        .nth(1)
                        .and_then(|h| h.parse::<u32>().ok())
                        .unwrap_or(0);
                    match height {
                        h if h <= 360 => Quality::Low,
                        h if h <= 480 => Quality::Medium,
                        h if h <= 720 => Quality::HD720,
                        h if h <= 1080 => Quality::HD1080,
                        h if h <= 2160 => Quality::UHD2160,
                        _ => Quality::Custom(format!("{}p", height)),
                    }
                } else {
                    Quality::Custom(resolution.to_string())
                };

                formats.push(VideoFormat {
                    id,
                    quality,
                    format,
                    file_size: None,
                });
            }
        }

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
            .map(|host| {
                host.ends_with("youtube.com") || 
                host.ends_with("youtu.be")
            })
            .unwrap_or(false)
    }

    async fn extract_info(&self, url: &Url) -> Result<VideoInfo> {
        let video_id = self.extract_video_id(url).await?;
        
        // Get video metadata using yt-dlp
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

    async fn download_video(
        &self,
        info: &VideoInfo,
        format_id: &str,
        output_path: &Path,
        progress_tx: Arc<watch::Sender<f64>>,
    ) -> Result<()> {
        let output_str = output_path.to_str()
            .ok_or_else(|| Error::Platform("Invalid output path".to_string()))?;

        let mut child = Command::new("yt-dlp")
            .arg("-f")
            .arg(format_id)
            .arg("--newline")
            .arg("-o")
            .arg(output_str)
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

impl Default for YouTube {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_url() {
        let youtube = YouTube::new();
        let youtube_url = Url::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        let youtu_be_url = Url::parse("https://youtu.be/dQw4w9WgXcQ").unwrap();
        let other_url = Url::parse("https://example.com/video").unwrap();
        
        assert!(youtube.supports_url(&youtube_url));
        assert!(youtube.supports_url(&youtu_be_url));
        assert!(!youtube.supports_url(&other_url));
    }
}
