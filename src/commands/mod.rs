mod download;
mod info;
mod config;
mod batch;
mod merge;
mod download_merge;

pub use download::download_command;
pub use info::info_command;
pub use config::config_command;
pub use batch::batch_download_command;
pub use merge::merge_command;
pub use download_merge::download_merge_command; 

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "video-dl")]
#[command(author = "Phong H. <huyphongbn24@gmail.com>")]
#[command(version = "0.3.0")]
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
    Merge {
        #[arg(short, long, help = "Video files to merge (can specify multiple)")]
        files: Vec<PathBuf>,
        
        #[arg(short = 'F', long, help = "File containing list of video paths, one per line")]
        file_list: Option<PathBuf>,
        
        #[arg(short, long, required = true, help = "Output file path")]
        output: PathBuf,
    },
    // Thêm lệnh mới DownloadMerge
    DownloadMerge {
        #[arg(short, long, help = "Video URLs to download and merge")]
        url: Vec<String>,
        
        #[arg(short = 'F', long, help = "File with URLs, one per line")]
        file: Option<PathBuf>,
        
        #[arg(short, long, required = true, help = "Output file path")]
        output: PathBuf,
        
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