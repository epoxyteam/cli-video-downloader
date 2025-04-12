mod download;
mod info;

pub use download::download_command;
pub use info::info_command;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "video-dl")]
#[command(author = "Your Name <your.email@example.com>")]
#[command(version = "0.1.0")]
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
}