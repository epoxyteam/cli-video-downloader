use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Invalid quality format: {0}")]
    InvalidQuality(String),

    #[error("Video not found: {0}")]
    VideoNotFound(Url),

    #[error("Invalid video format: {0}")]
    InvalidFormat(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub trait ErrorExt {
    fn context<C>(self, context: C) -> Self
    where
        C: std::fmt::Display + Send + Sync + 'static;
}

impl ErrorExt for Error {
    fn context<C>(self, context: C) -> Self
    where
        C: std::fmt::Display + Send + Sync + 'static,
    {
        match self {
            Error::Other(err) => Error::Other(err.context(context)),
            err => Error::Other(anyhow::Error::new(err).context(context)),
        }
    }
}