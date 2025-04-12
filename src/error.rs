use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported platform")]
    UnsupportedPlatform,
    
    #[error("Failed to execute external command: {command} - {reason}")]
    CommandExecution {
        command: String,
        reason: String,
    },
    
    #[error("Failed to parse command output: {0}")]
    OutputParsing(String),
    
    #[error("Download failed: {reason}")]
    DownloadFailed {
        reason: String,
    },
    
    #[error("Invalid output path: {0}")]
    InvalidOutputPath(PathBuf),
    
    #[error("No suitable formats found")]
    NoSuitableFormats,
}

pub type Result<T> = std::result::Result<T, Error>;

// Helper function to create context-rich platform errors
pub fn platform_err<S: Into<String>>(msg: S) -> Error {
    Error::Platform(msg.into())
}

// Helper function to create context-rich command execution errors
pub fn command_err<S1: Into<String>, S2: Into<String>>(command: S1, reason: S2) -> Error {
    Error::CommandExecution {
        command: command.into(),
        reason: reason.into(),
    }
}