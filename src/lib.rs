use thiserror::Error;
use url::Url;
use std::fmt;

pub mod platform;
pub mod downloader;
pub mod error;

/// Re-export common types
pub use error::Error;
pub use platform::{Platform, PlatformDetector};
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub url: Url,
    pub title: String,
    pub description: Option<String>,
    pub duration: Option<u64>,
    pub formats: Vec<VideoFormat>,
}

#[derive(Debug, Clone)]
pub struct VideoFormat {
    pub id: String,
    pub quality: Quality,
    pub format: Format,
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
            Quality::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
            Format::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::str::FromStr for Quality {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Quality::Low),
            "medium" => Ok(Quality::Medium),
            "high" => Ok(Quality::High),
            "720p" | "hd720" => Ok(Quality::HD720),
            "1080p" | "hd1080" => Ok(Quality::HD1080),
            "2160p" | "4k" | "uhd" => Ok(Quality::UHD2160),
            _ => Ok(Quality::Custom(s.to_string())),
        }
    }
}

impl std::str::FromStr for Format {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "mp4" => Ok(Format::MP4),
            "webm" => Ok(Format::WebM),
            "mov" => Ok(Format::MOV),
            _ => Ok(Format::Other(s.to_string())),
        }
    }
}

impl VideoFormat {
    /// Returns true if this format matches the specified quality and format strings
    pub fn matches(&self, quality: &str, format: &str) -> bool {
        self.quality.to_string().eq_ignore_ascii_case(quality) &&
        self.format.to_string().eq_ignore_ascii_case(format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_display() {
        assert_eq!(Quality::Low.to_string(), "low");
        assert_eq!(Quality::HD1080.to_string(), "1080p");
        assert_eq!(Quality::Custom("1440p".to_string()).to_string(), "1440p");
    }

    #[test]
    fn test_format_display() {
        assert_eq!(Format::MP4.to_string(), "mp4");
        assert_eq!(Format::WebM.to_string(), "webm");
        assert_eq!(Format::Other("mkv".to_string()).to_string(), "mkv");
    }

    #[test]
    fn test_quality_fromstr() {
        assert_eq!("low".parse::<Quality>().unwrap(), Quality::Low);
        assert_eq!("1080p".parse::<Quality>().unwrap(), Quality::HD1080);
        assert_eq!("1440p".parse::<Quality>().unwrap(), Quality::Custom("1440p".to_string()));
    }

    #[test]
    fn test_format_fromstr() {
        assert_eq!("mp4".parse::<Format>().unwrap(), Format::MP4);
        assert_eq!("webm".parse::<Format>().unwrap(), Format::WebM);
        assert_eq!("mkv".parse::<Format>().unwrap(), Format::Other("mkv".to_string()));
    }
}