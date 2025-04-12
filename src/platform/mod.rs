use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use std::fmt;
use tokio::sync::watch;
use url::Url;

use crate::Result;

pub mod detector;
pub mod youtube;

#[derive(Debug, Clone, PartialEq)]
pub enum Quality {
    Low,
    Medium,
    High,
    HD720,
    HD1080,
    UHD2160,
    Custom(String),
}

impl fmt::Display for Quality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Quality::Low => write!(f, "low"),
            Quality::Medium => write!(f, "medium"),
            Quality::High => write!(f, "high"),
            Quality::HD720 => write!(f, "720p"),
            Quality::HD1080 => write!(f, "1080p"),
            Quality::UHD2160 => write!(f, "2160p"),
            Quality::Custom(q) => write!(f, "{}", q),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Format {
    MP4,
    WebM,
    MOV,
    Other(String),
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Format::MP4 => write!(f, "mp4"),
            Format::WebM => write!(f, "webm"),
            Format::MOV => write!(f, "mov"),
            Format::Other(fmt) => write!(f, "{}", fmt),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoFormat {
    pub id: String,
    pub quality: Quality,
    pub format: Format,
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub url: Url,
    pub title: String,
    pub description: Option<String>,
    pub duration: Option<u64>,
    pub formats: Vec<VideoFormat>,
}

#[async_trait]
pub trait Platform: Send + Sync {
    fn name(&self) -> &'static str;
    fn supports_url(&self, url: &Url) -> bool;
    async fn extract_info(&self, url: &Url) -> Result<VideoInfo>;
    async fn download_video(
        &self,
        info: &VideoInfo,
        format_id: &str,
        output_path: &Path,
        progress_tx: Arc<watch::Sender<f64>>,
    ) -> Result<()>;
}

pub fn normalize_url(url: &str) -> Result<Url> {
    Url::parse(url).map_err(|_| crate::Error::InvalidUrl(url.to_string()))
}