mod download;
mod info;
mod config;
mod batch; // Add new batch module

pub use download::download_command;
pub use info::info_command;
pub use config::config_command;
pub use batch::batch_download_command; // Export the batch download command

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "video-dl")]
#[command(author = "Phong H. <huyphongbn24@gmail.com>")]
#[command(version = "0.2.0")]
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
    Batch {
        #[arg(short, long, help = "Video URLs to download")]
        url: Vec<String>,
        
        #[arg(short = 'F', long, help = "File with URLs, one per line")]
        file: Option<PathBuf>,
        
        #[arg(short = 'd', long, help = "Output directory")]
        output_dir: Option<PathBuf>,
        
        #[arg(short = 'q', long, default_value = "best", help = "Video quality")]
        quality: String,
        
        #[arg(short = 'f', long, default_value = "mp4", help = "Video format")]
        format: String,
        
        #[arg(short = 'p', long, help = "Download videos in parallel")]
        parallel: bool,
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