use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::{sleep, Duration};
use url::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use reqwest::{Client, header, redirect};
use futures_util::StreamExt;
use regex::Regex;
use log::{debug, info};
use std::sync::OnceLock;
use base64::Engine;

use crate::{Error, Result};
use super::{Platform, VideoFormat, VideoInfo, Quality, Format};

static USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
static CLIENT: OnceLock<Client> = OnceLock::new();
const DELAY_MS: u64 = 200;

fn extract_json_from_html(html: &str) -> Option<Value> {
    let patterns = [
        r#"<script id="SIGI_STATE" type="application/json">(.+?)</script>"#,
        r#"<script id="__NEXT_DATA__" type="application/json">(.+?)</script>"#,
        r#"window\._SSR_DATA\s*=\s*({.+?})</script>"#,
        r#"<script id="__UNIVERSAL_DATA_FOR_REHYDRATION__" type="application/json">(.+?)</script>"#,
        r#"<script id="RENDER_DATA" type="application/json">(.+?)</script>"#,
        r#"window\.__INIT_PROPS__\s*=\s*({.+?});\s*</script>"#,
    ];

    for pattern in &patterns {
        if let Some(re) = Regex::new(pattern).ok() {
            if let Some(caps) = re.captures(html) {
                if let Some(json_str) = caps.get(1) {
                    let json_text = json_str.as_str();
                    let decoded_text = if json_text.chars().all(|c| c.is_ascii() && (c.is_alphanumeric() || "+/=".contains(c))) {
                        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(json_text) {
                            if let Ok(text) = String::from_utf8(decoded) {
                                text
                            } else {
                                json_text.to_string()
                            }
                        } else {
                            json_text.to_string()
                        }
                    } else {
                        json_text.to_string()
                    };
                    
                    if let Ok(json) = serde_json::from_str(&decoded_text) {
                        return Some(json);
                    }
                }
            }
        }
    }

    let item_patterns = [
        r#""videoData"\s*:\s*({.+?}),\s*"#,
        r#""ItemModule"\s*:\s*({.+?}),\s*"#,
        r#""itemInfo"\s*:\s*({.+?}),\s*"#,
    ];

    for pattern in &item_patterns {
        if let Some(re) = Regex::new(pattern).ok() {
            if let Some(caps) = re.captures(html) {
                if let Some(json_str) = caps.get(1) {
                    if let Ok(json) = serde_json::from_str(json_str.as_str()) {
                        return Some(json);
                    }
                }
            }
        }
    }

    None
}

#[derive(Debug, Deserialize, Serialize)]
struct TikTokVideo {
    desc: String,
    #[serde(default)]
    video: TikTokVideoDetails,
    author: TikTokAuthor,
    #[serde(rename = "stats")]
    statistics: TikTokStats,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct TikTokVideoDetails {
    #[serde(default)]
    height: u32,
    #[serde(default)]
    duration: u64,
    #[serde(rename = "playAddr")]
    play_url: String,
    #[serde(rename = "downloadAddr")]
    download_url: String,
    #[serde(default)]
    format: String,
    #[serde(rename = "bitrate", default)]
    bit_rate: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct TikTokAuthor {
    nickname: String,
    #[serde(rename = "uniqueId")]
    unique_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TikTokStats {
    #[serde(rename = "playCount", default)]
    play_count: u64,
    #[serde(rename = "likeCount", default)]
    like_count: u64,
}

pub struct TikTok {
    client: Client,
}

impl Default for TikTok {
    fn default() -> Self {
        Self::new()
    }
}

impl TikTok {
    pub fn new() -> Self {
        Self {
            client: CLIENT.get_or_init(|| {
                let mut headers = header::HeaderMap::new();
                headers.insert(header::USER_AGENT, header::HeaderValue::from_static(USER_AGENT));
                headers.insert(header::ACCEPT, header::HeaderValue::from_static("application/json, text/plain, */*"));
                headers.insert(header::ACCEPT_LANGUAGE, header::HeaderValue::from_static("en-US,en;q=0.9"));
                headers.insert(header::ACCEPT_ENCODING, header::HeaderValue::from_static("gzip, deflate, br"));
                headers.insert("tt-web-region", header::HeaderValue::from_static("US"));
                headers.insert("sec-ch-ua", header::HeaderValue::from_static("\"Chromium\";v=\"122\", \"Google Chrome\";v=\"122\""));
                headers.insert("sec-ch-ua-mobile", header::HeaderValue::from_static("?0"));
                headers.insert("sec-ch-ua-platform", header::HeaderValue::from_static("\"Windows\""));
                headers.insert("sec-fetch-dest", header::HeaderValue::from_static("empty"));
                headers.insert("sec-fetch-mode", header::HeaderValue::from_static("cors"));
                headers.insert("sec-fetch-site", header::HeaderValue::from_static("same-origin"));

                Client::builder()
                    .default_headers(headers)
                    .user_agent(USER_AGENT)
                    .cookie_store(true)
                    .redirect(redirect::Policy::custom(|attempt| {
                        if attempt.previous().len() < 5 {
                            attempt.follow()
                        } else {
                            attempt.stop()
                        }
                    }))
                    .build()
                    .unwrap_or_default()
            }).clone()
        }
    }

    fn normalize_url(url: &str) -> String {
        let url = url.replace("vm.tiktok.com", "www.tiktok.com");
        let url = if !url.contains("www.tiktok.com") {
            url.replace("tiktok.com", "www.tiktok.com")
        } else {
            url
        };
        if !url.starts_with("https://") {
            format!("https://{}", url)
        } else {
            url
        }
    }

    async fn extract_video_url_from_html(&self, html: &str, video_id: &str) -> Result<TikTokVideo> {
        debug!("Attempting to extract video URL directly from HTML");
        
        let video_url_patterns = [
            r#"<video[^>]*\ssrc="([^"]+)"[^>]*>#is"#,
            r#"<video[^>]*\sdata-src="([^"]+)"[^>]*>#is"#,
            r#"playAddr":"([^"]+)"#,
            r#"playAddr\\\":\\\"([^\\]+)\\\"#,
            r#"playAddr=([^&]+)"#,
        ];
        
        let mut video_url = String::new();
        
        for pattern in &video_url_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(html) {
                    if caps.len() > 1 {
                        if let Some(url_match) = caps.get(1) {
                            let url = url_match.as_str();
                            let clean_url = url.replace("\\u002F", "/")
                                              .replace("\\u0026", "&")
                                              .replace("\\", "");
                                              
                            video_url = clean_url;
                            debug!("Found video URL: {}", video_url);
                            break;
                        }
                    }
                }
            }
        }
        
        if video_url.is_empty() {
            return Err(Error::Platform("Could not extract video URL from HTML".into()));
        }
        
        let mut author_name = "Unknown".to_string();
        let mut author_id = "unknown".to_string();
        
        let author_patterns = [
            r#"nickname":"([^"]+)"#,
            r#"nickname\\\":\\\"([^\\]+)\\\"#,
            r#"<h3[^>]*>([^<]+)</h3>#is"#,
        ];
        
        for pattern in &author_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(html) {
                    if caps.len() > 1 {
                        if let Some(name_match) = caps.get(1) {
                            author_name = name_match.as_str().to_string();
                            break;
                        }
                    }
                }
            }
        }
        
        let id_patterns = [
            r#"uniqueId":"([^"]+)"#,
            r#"uniqueId\\\":\\\"([^\\]+)\\\"#,
            r#"@([a-zA-Z0-9_.]+)"#,
        ];
        
        for pattern in &id_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(html) {
                    if caps.len() > 1 {
                        if let Some(id_match) = caps.get(1) {
                            author_id = id_match.as_str().to_string();
                            break;
                        }
                    }
                }
            }
        }
        
        let mut video_title = format!("TikTok Video {}", video_id);
        let desc_patterns = [
            r#"desc":"([^"]+)"#,
            r#"desc\\\":\\\"([^\\]+)\\\"#,
            r#"<meta name="description" content="([^"]+)"#,
        ];
        
        for pattern in &desc_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(html) {
                    if caps.len() > 1 {
                        if let Some(desc_match) = caps.get(1) {
                            let desc = desc_match.as_str();
                            if !desc.is_empty() {
                                video_title = desc.to_string();
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        let video_details = TikTokVideoDetails {
            height: 720,
            duration: 30,
            play_url: video_url.clone(),
            download_url: video_url,
            format: "mp4".to_string(),
            bit_rate: 1_000_000,
        };
        
        let author = TikTokAuthor {
            nickname: author_name,
            unique_id: author_id,
        };
        
        let stats = TikTokStats {
            play_count: 0,
            like_count: 0,
        };
        
        Ok(TikTokVideo {
            desc: video_title,
            video: video_details,
            author,
            statistics: stats,
        })
    }

    async fn fetch_video_info(&self, url: &str) -> Result<TikTokVideo> {
        let canonical_url = Self::normalize_url(url);
        info!("Fetching URL: {}", canonical_url);

        sleep(Duration::from_millis(DELAY_MS)).await;

        let html_response = self.client
            .get(&canonical_url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Upgrade-Insecure-Requests", "1")
            .send()
            .await
            .map_err(|e| Error::Network(e))?
            .text()
            .await
            .map_err(|e| Error::Network(e))?;

        // Support more URL patterns: standard format, short format, and mobile format
        let patterns = [
            "@([^/]+)/video/(\\d+)",          // Standard format: @username/video/1234567890
            "tiktok\\.com/v/(\\d+)",          // Short format: v/1234567890 (username not available)
            "/video/(\\d+)",                  // Mobile or alternate format
        ];

        let mut username = String::new();
        let mut video_id = String::new();

        for pattern in &patterns {
            let re = Regex::new(pattern).unwrap();
            if let Some(caps) = re.captures(&canonical_url) {
                if pattern == &"@([^/]+)/video/(\\d+)" {
                    if caps.len() > 2 {
                        username = caps.get(1).unwrap().as_str().to_string();
                        video_id = caps.get(2).unwrap().as_str().to_string();
                    }
                } else if pattern == &"tiktok\\.com/v/(\\d+)" || pattern == &"/video/(\\d+)" {
                    if caps.len() > 1 {
                        // For formats without username, we'll use empty string
                        video_id = caps.get(1).unwrap().as_str().to_string();
                    }
                }
                
                if !video_id.is_empty() {
                    break;
                }
            }
        }

        if video_id.is_empty() {
            return Err(Error::Platform("Invalid TikTok URL format".into()));
        }

        debug!("Username: {}, Video ID: {}", username, video_id);

        if let Some(json_data) = extract_json_from_html(&html_response) {
            if let Ok(video_info) = serde_json::from_value::<TikTokVideo>(json_data) {
                return Ok(video_info);
            }
        }

        let api_url = format!(
            "https://api16-normal-c-useast1a.tiktokv.com/aweme/v1/feed/?aweme_id={}",
            video_id
        );

        debug!("Trying API endpoint: {}", api_url);

        let api_response = self.client
            .get(&api_url)
            .header("Referer", &canonical_url)
            .send()
            .await;

        match api_response {
            Ok(response) => {
                if response.status().is_success() {
                    let text = response.text().await.map_err(|e| Error::Network(e))?;
                    
                    if let Ok(api_data) = serde_json::from_str::<Value>(&text) {
                        if let Some(video_data) = api_data.get("aweme_list").and_then(|list| list.get(0)) {
                            if let Ok(video_info) = serde_json::from_value::<TikTokVideo>(video_data.clone()) {
                                return Ok(video_info);
                            }
                        }
                    }
                }
            },
            Err(e) => {
                debug!("API request failed: {}", e);
            }
        }

        self.extract_video_url_from_html(&html_response, &video_id).await
    }

    fn determine_quality(&self, height: u32) -> Quality {
        if height >= 1080 {
            Quality::HD1080
        } else if height >= 720 {
            Quality::HD720
        } else if height >= 480 {
            Quality::Medium
        } else {
            Quality::Low
        }
    }

    async fn download_video_file(
        &self,
        url: &str,
        output_path: &Path,
        progress_tx: Arc<watch::Sender<f64>>,
    ) -> Result<()> {
        let response = self.client
            .get(url)
            .header("Range", "bytes=0-")
            .header("Referer", "https://www.tiktok.com/")
            .send()
            .await
            .map_err(|e| Error::Network(e))?;

        let total_size = response
            .content_length()
            .ok_or_else(|| Error::DownloadFailed {
                reason: "Content length not available".to_string(),
            })?;

        let mut file = File::create(output_path)
            .await
            .map_err(|e| Error::IO(e))?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|e| Error::Network(e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| Error::IO(e))?;

            downloaded += chunk.len() as u64;
            let progress = (downloaded as f64) / (total_size as f64);
            let _ = progress_tx.send(progress);
        }

        Ok(())
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
        let video_info = self.fetch_video_info(url.as_str()).await?;
        let quality = self.determine_quality(video_info.video.height);

        let mut formats = Vec::new();
        formats.push(VideoFormat {
            id: "default".to_string(),
            quality,
            format: Format::MP4,
            file_size: Some((video_info.video.bit_rate / 8) * video_info.video.duration),
        });

        let desc = format!(
            "By {} (@{}) - {} plays, {} likes",
            video_info.author.nickname,
            video_info.author.unique_id,
            video_info.statistics.play_count,
            video_info.statistics.like_count
        );

        Ok(VideoInfo {
            url: url.clone(),
            title: video_info.desc,
            description: Some(desc),
            duration: Some(video_info.video.duration),
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
        if format_id != "default" {
            return Err(Error::InvalidFormat(format_id.to_string()));
        }

        let video_info = self.fetch_video_info(info.url.as_str()).await?;
        
        // First try the download URL
        let download_result = self.download_video_file(&video_info.video.download_url, output_path, progress_tx.clone()).await;
        
        if download_result.is_ok() {
            return Ok(());
        }
        
        // If download URL failed, try with play URL
        debug!("Download URL failed, trying play URL");
        let play_result = self.download_video_file(&video_info.video.play_url, output_path, progress_tx.clone()).await;
        
        if play_result.is_ok() {
            return Ok(());
        }

        // If both methods failed, try to extract a new URL from the webpage again
        // This helps when URLs expire quickly
        debug!("Both download URLs failed, trying to refresh video info");
        let refreshed_info = self.fetch_video_info(info.url.as_str()).await?;
        
        // Add a small delay before trying again
        sleep(Duration::from_millis(500)).await;
        
        // Try both URLs from the refreshed info
        let refresh_download_result = self.download_video_file(&refreshed_info.video.download_url, output_path, progress_tx.clone()).await;
        
        if refresh_download_result.is_ok() {
            return Ok(());
        }
        
        let refresh_play_result = self.download_video_file(&refreshed_info.video.play_url, output_path, progress_tx).await;
        
        if refresh_play_result.is_ok() {
            return Ok(());
        }
        
        // If all attempts failed, return the original error
        Err(Error::DownloadFailed {
            reason: "Failed to download TikTok video after multiple attempts".into()
        })
    }
}
