use std::path::PathBuf;
use std::sync::Arc;

use crate::{Result, platform::normalize_url, Config};
use crate::platform::detector::PlatformDetector;
use crate::utils::progress::ProgressTracker;

pub struct Downloader {
    detector: PlatformDetector,
    config: Config,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            detector: PlatformDetector::new(),
            config: Config::load(),
        }
    }
    
    /// Create a new downloader with a custom configuration
    pub fn with_config(config: Config) -> Self {
        Self {
            detector: PlatformDetector::new(),
            config,
        }
    }
    
    /// Get the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
    
    /// Get a mutable reference to the configuration
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    pub async fn get_video_info(&self, url: &str) -> Result<crate::platform::VideoInfo> {
        let url = normalize_url(url)?;
        let platform = self.detector.detect(&url)?;
        platform.extract_info(&url).await
    }

    pub async fn download(&self, url: &str, format_id: &str, output: Option<PathBuf>) -> Result<PathBuf> {
        let url = normalize_url(url)?;
        let platform = self.detector.detect(&url)?;
        let info = platform.extract_info(&url).await?;

        // Use configured output directory if none is specified
        let output_path = match output {
            Some(path) => path,
            None => {
                // Use configuration download directory
                let filename = format!(
                    "{}.mp4",
                    sanitize_filename::sanitize(&info.title),
                );
                self.config.download_dir.join(filename)
            }
        };
        
        // Create progress tracker if configured to show progress
        let progress = if self.config.show_progress {
            Some(ProgressTracker::new())
        } else {
            None
        };
        
        // Download the video
        let result = match &progress {
            Some(tracker) => {
                platform
                    .download_video(&info, format_id, &output_path, tracker.get_sender())
                    .await
            }
            None => {
                // Create a dummy progress sender
                let (tx, _) = tokio::sync::watch::channel(0.0);
                platform
                    .download_video(&info, format_id, &output_path, Arc::new(tx))
                    .await
            }
        };
        
        // Finish the progress tracker if used
        if let Some(tracker) = progress {
            tracker.finish();
        }

        // Return the output path on success
        result.map(|_| output_path)
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}