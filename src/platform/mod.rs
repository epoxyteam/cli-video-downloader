use async_trait::async_trait;
use url::Url;
use std::sync::Arc;

use crate::{Error, Result, VideoInfo};

/// Core platform trait that can be used as a trait object
#[async_trait]
pub trait Platform: Send + Sync {
    /// Returns the name of the platform (e.g., "YouTube", "TikTok")
    fn name(&self) -> &'static str;

    /// Checks if the given URL is supported by this platform
    fn supports_url(&self, url: &Url) -> bool;

    /// Extracts video information without downloading
    async fn extract_info(&self, url: &Url) -> Result<VideoInfo>;

    /// Downloads the video with the specified format
    async fn download_video(
        &self,
        info: &VideoInfo,
        format_id: &str,
        output_path: &std::path::Path,
        progress_tx: Arc<tokio::sync::watch::Sender<f64>>,
    ) -> Result<()>;
}

pub mod detector;
pub mod youtube;
// We'll implement these later
// pub mod tiktok;
// pub mod reddit;
// pub mod instagram;

// Re-export platform implementations
pub use detector::PlatformDetector;
pub use youtube::YouTube;

// Factory function to create a new platform detector with all supported platforms
pub fn create_platform_detector() -> PlatformDetector {
    let mut detector = PlatformDetector::new();
    
    // Register platform implementations
    detector.register(Arc::new(YouTube::new()));
    
    detector
}

// Helper function to normalize URLs (remove tracking parameters, etc.)
pub fn normalize_url(url: &str) -> Result<Url> {
    let url = Url::parse(url).map_err(|e| Error::InvalidUrl(e.to_string()))?;
    
    // Create a new URL with only the essential components
    let mut normalized = Url::parse(&format!(
        "{}://{}{}",
        url.scheme(),
        url.host_str().ok_or_else(|| Error::InvalidUrl("No host found".into()))?,
        url.path()
    )).map_err(|e| Error::InvalidUrl(e.to_string()))?;
    
    // Preserve only essential query parameters (platform specific)
    if let Some(query) = url.query() {
        let essential_params: Vec<(String, String)> = url
            .query_pairs()
            .filter(|(key, _)| {
                matches!(key.as_ref(), 
                    "v" | // YouTube video ID
                    "t" | // Timestamp
                    "id" // Generic video ID
                )
            })
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
            
        if !essential_params.is_empty() {
            normalized.set_query(Some(
                &essential_params
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("&")
            ));
        }
    }
    
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        let test_cases = vec![
            (
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ&feature=shared",
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
            ),
            (
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=30s&feature=shared",
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=30s"
            ),
        ];

        for (input, expected) in test_cases {
            let normalized = normalize_url(input).unwrap().to_string();
            assert_eq!(normalized, expected);
        }
    }
}