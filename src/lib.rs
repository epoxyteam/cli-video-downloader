mod error;
mod platform;
pub mod downloader;
pub mod commands;
pub mod config;
pub mod utils;

pub use error::{Error, Result};
pub use platform::{Platform, Quality, Format, VideoFormat, VideoInfo};
pub use platform::detector::PlatformDetector;
pub use downloader::Downloader;
pub use config::Config;