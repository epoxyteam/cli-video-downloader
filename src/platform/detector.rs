use std::sync::Arc;
use crate::platform::PlatformFactory;
use crate::platform::Platform;
use crate::Result;
use crate::Error;
use url::Url;

#[derive(Clone)]
pub struct PlatformDetector {
    platforms: Vec<Arc<dyn Platform>>,
}

impl PlatformDetector {
    pub fn new() -> Self {
        let mut detector = Self {
            platforms: Vec::new(),
        };
        
        // Use the factory to register all supported platforms
        PlatformFactory::register_platforms(&mut detector);
        
        detector
    }
    
    /// Register a new platform implementation
    pub fn register(&mut self, platform: Arc<dyn Platform>) {
        self.platforms.push(platform);
    }

    /// Detect the appropriate platform implementation for a URL
    pub fn detect(&self, url: &Url) -> Result<Arc<dyn Platform>> {
        for platform in &self.platforms {
            if platform.supports_url(url) {
                return Ok(platform.clone());
            }
        }
        
        Err(Error::UnsupportedPlatform)
    }
    
    /// List all supported platforms
    pub fn supported_platforms(&self) -> Vec<&str> {
        self.platforms.iter()
            .map(|p| p.name())
            .collect()
    }
}

impl Default for PlatformDetector {
    fn default() -> Self {
        Self::new()
    }
}