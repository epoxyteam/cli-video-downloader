use std::sync::Arc;
use url::Url;
use crate::{Error, Result, platform::Platform};

/// PlatformDetector is responsible for determining which platform handler
/// should be used for a given URL
pub struct PlatformDetector {
    platforms: Vec<Arc<dyn Platform>>,
}

impl PlatformDetector {
    /// Creates a new empty platform detector
    pub fn new() -> Self {
        Self {
            platforms: Vec::new(),
        }
    }

    /// Registers a new platform handler
    pub fn register(&mut self, platform: Arc<dyn Platform>) {
        self.platforms.push(platform);
    }

    /// Detects the appropriate platform for the given URL
    pub fn detect(&self, url: &Url) -> Result<Arc<dyn Platform>> {
        for platform in &self.platforms {
            if platform.supports_url(url) {
                return Ok(Arc::clone(platform));
            }
        }

        Err(Error::UnsupportedPlatform(
            url.host_str()
                .unwrap_or("unknown")
                .to_string(),
        ))
    }

    /// Returns all registered platforms
    pub fn platforms(&self) -> &[Arc<dyn Platform>] {
        &self.platforms
    }
}

impl Default for PlatformDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::path::Path;
    use tokio::sync::watch;

    struct MockPlatform {
        name: &'static str,
        supported_domain: &'static str,
    }

    #[async_trait]
    impl Platform for MockPlatform {
        fn name(&self) -> &'static str {
            self.name
        }

        fn supports_url(&self, url: &Url) -> bool {
            url.host_str()
                .map(|host| host.ends_with(self.supported_domain))
                .unwrap_or(false)
        }

        async fn extract_info(&self, _url: &Url) -> Result<crate::VideoInfo> {
            unimplemented!("Mock platform does not implement extract_info")
        }

        async fn download_video(
            &self,
            _info: &crate::VideoInfo,
            _format_id: &str,
            _output_path: &Path,
            _progress_tx: Arc<watch::Sender<f64>>,
        ) -> Result<()> {
            unimplemented!("Mock platform does not implement download")
        }
    }

    #[test]
    fn test_platform_detection() {
        let mut detector = PlatformDetector::new();
        
        detector.register(Arc::new(MockPlatform {
            name: "YouTube",
            supported_domain: "youtube.com",
        }));
        
        detector.register(Arc::new(MockPlatform {
            name: "TikTok",
            supported_domain: "tiktok.com",
        }));

        // Test YouTube URL
        let youtube_url = Url::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        let platform = detector.detect(&youtube_url).unwrap();
        assert_eq!(platform.name(), "YouTube");

        // Test TikTok URL
        let tiktok_url = Url::parse("https://www.tiktok.com/@user/video/1234567890").unwrap();
        let platform = detector.detect(&tiktok_url).unwrap();
        assert_eq!(platform.name(), "TikTok");

        // Test unsupported URL
        let unsupported_url = Url::parse("https://example.com/video").unwrap();
        assert!(detector.detect(&unsupported_url).is_err());
    }
}