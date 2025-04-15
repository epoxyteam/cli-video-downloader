use async_trait::async_trait;
use std::sync::Arc;
use std::path::Path;
use tokio::sync::watch;
use url::Url;
use futures_util::StreamExt;

use super::{Format, Platform, Quality, VideoFormat, VideoInfo};
use crate::{Result, Error};

#[derive(Default)]
pub struct RedditPlatform;

impl RedditPlatform {
    pub fn new() -> Arc<dyn Platform> {
        Arc::new(Self)
    }

    fn is_reddit_url(url: &Url) -> bool {
        let host = url.host_str().unwrap_or("");
        host == "reddit.com" || host == "www.reddit.com" || host.ends_with(".reddit.com")
    }
}

#[async_trait]
impl Platform for RedditPlatform {
    fn name(&self) -> &'static str {
        "reddit"
    }

    fn supports_url(&self, url: &Url) -> bool {
        Self::is_reddit_url(url)
    }

    async fn extract_info(&self, url: &Url) -> Result<VideoInfo> {
        // First get the JSON API endpoint for the post
        let api_url = url.as_str().replace("www.reddit.com", "api.reddit.com");
        
        // Fetch post data from Reddit API
        let client = reqwest::Client::new();
        let response = client
            .get(&api_url)
            .header("User-Agent", "cli-video-downloader")
            .send()
            .await?;

        let data = response.json::<serde_json::Value>().await?;

        // Extract video information from the JSON response
        let post = &data[0]["data"]["children"][0]["data"];
        
        // Check if it's a video post
        if !post["is_video"].as_bool().unwrap_or(false) {
            return Err(Error::Platform("Not a video post".into()));
        }

        // Get video details
        let video_url = post["media"]["reddit_video"]["fallback_url"]
            .as_str()
            .ok_or_else(|| Error::Platform("Could not find video URL".into()))?;
        
        let duration = post["media"]["reddit_video"]["duration"]
            .as_u64()
            .unwrap_or(0);

        // Create available formats list
        let formats = vec![
            VideoFormat {
                id: "high".to_string(),
                quality: Quality::High,
                format: Format::MP4,
                file_size: None,
            }
        ];

        Ok(VideoInfo {
            url: url.clone(),
            title: post["title"].as_str().unwrap_or("Untitled").to_string(),
            description: Some(post["selftext"].as_str().unwrap_or("").to_string()),
            duration: Some(duration),
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
        // Get the video URL from the Reddit API again to ensure fresh URL
        let api_url = info.url.as_str().replace("www.reddit.com", "api.reddit.com");
        
        let client = reqwest::Client::new();
        let mut response = client
            .get(&api_url)
            .header("User-Agent", "cli-video-downloader")
            .send()
            .await?;

        let data = response.json::<serde_json::Value>().await?;
        let post = &data[0]["data"]["children"][0]["data"];
        
        let video_url = post["media"]["reddit_video"]["fallback_url"]
            .as_str()
            .ok_or_else(|| Error::Platform("Could not find video URL".into()))?;

        // Download the video
        let response = client
            .get(video_url)
            .header("User-Agent", "cli-video-downloader")
            .send()
            .await?;

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut file = tokio::fs::File::create(output_path).await?;

        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            downloaded += chunk.len() as u64;
            
            if total_size > 0 {
                let progress = (downloaded as f64 / total_size as f64) * 100.0;
                if let Err(e) = progress_tx.send(progress) {
                    return Err(Error::Platform(format!("Failed to send progress update: {}", e)));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reddit_url_detection() {
        let valid_urls = [
            "https://reddit.com/r/videos/comments/abc123",
            "https://www.reddit.com/r/funny/comments/def456",
            "https://old.reddit.com/r/gifs/comments/ghi789",
        ];

        let invalid_urls = [
            "https://youtube.com/watch?v=abc123",
            "https://vimeo.com/123456",
            "https://notreddit.com/video",
        ];

        for url in valid_urls.iter() {
            let parsed_url = Url::parse(url).unwrap();
            assert!(RedditPlatform::is_reddit_url(&parsed_url));
        }

        for url in invalid_urls.iter() {
            let parsed_url = Url::parse(url).unwrap();
            assert!(!RedditPlatform::is_reddit_url(&parsed_url));
        }
    }
}