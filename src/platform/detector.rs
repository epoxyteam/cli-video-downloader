use crate::platform::youtube::YouTube;
use crate::platform::Platform;
use crate::Result;
use crate::Error;
use url::Url;

#[derive(Default)]
pub struct PlatformDetector {
    youtube: YouTube,
}

impl PlatformDetector {
    pub fn new() -> Self {
        Self {
            youtube: YouTube::default(),
        }
    }

    pub fn detect(&self, url: &Url) -> Result<&dyn Platform> {
        if self.youtube.supports_url(url) {
            Ok(&self.youtube)
        } else {
            Err(Error::UnsupportedPlatform)
        }
    }
}