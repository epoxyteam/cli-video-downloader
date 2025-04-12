use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{Result, platform::normalize_url};
use crate::platform::detector::PlatformDetector;

pub struct Downloader {
    detector: PlatformDetector,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            detector: PlatformDetector::new(),
        }
    }

    pub async fn get_video_info(&self, url: &str) -> Result<crate::platform::VideoInfo> {
        let url = normalize_url(url)?;
        let platform = self.detector.detect(&url)?;
        platform.extract_info(&url).await
    }

    pub async fn download(&self, url: &str, format_id: &str, output: PathBuf) -> Result<()> {
        let url = normalize_url(url)?;
        let platform = self.detector.detect(&url)?;
        let info = platform.extract_info(&url).await?;

        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {percent}% ({eta})")
                .unwrap()
                .progress_chars("=>-")
        );

        let (progress_tx, mut progress_rx) = watch::channel(0.0);
        
        let pb_handle = tokio::spawn({
            let pb = pb.clone();
            async move {
                while progress_rx.changed().await.is_ok() {
                    let progress = *progress_rx.borrow();
                    pb.set_position((progress * 100.0) as u64);
                }
            }
        });

        let result = platform
            .download_video(&info, format_id, &output, Arc::new(progress_tx))
            .await;

        pb_handle.abort();
        pb.finish_and_clear();

        result
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}