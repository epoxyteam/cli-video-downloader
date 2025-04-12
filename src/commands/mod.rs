mod download;
mod info;
mod config;

pub use download::download_command;
pub use info::info_command;
pub use config::config_command;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "video-dl")]
#[command(author = "Phong H. <huyphongbn24@gmail.com>")]
#[command(version = "0.1.1")]
#[command(about = "A fast and user-friendly video downloader")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Download {
        #[arg(short, long)]
        url: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(short, long, default_value = "best")]
        quality: String,
        #[arg(short, long, default_value = "mp4")]
        format: String,
    },
    Info {
        #[arg(short, long)]
        url: String,
    },
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    Set {
        #[arg(short, long)]
        key: String,
        #[arg(short, long)]
        value: String,
    },
    Get {
        #[arg(short, long)]
        key: Option<String>,
    },
    Reset,
}