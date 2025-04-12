use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use video_dl::{
    Error, Result,
    platform::{create_platform_detector, normalize_url},
};

#[derive(Parser)]
#[command(name = "video-dl")]
#[command(author = "Phong H. <huyphongbn24@gmail.com>")]
#[command(version = "0.1.0")]
#[command(about = "A fast and user-friendly video downloader")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download a video
    Download {
        /// URL of the video to download
        #[arg(short, long)]
        url: String,

        /// Output path (optional)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Preferred video quality (e.g., "1080p", "720p", "best")
        #[arg(short, long, default_value = "best")]
        quality: String,

        /// Video format (e.g., "mp4", "webm")
        #[arg(short, long, default_value = "mp4")]
        format: String,
    },
    /// Show information about a video without downloading
    Info {
        /// URL of the video
        #[arg(short, long)]
        url: String,
    },
}

async fn handle_download(
    url: String,
    output: Option<PathBuf>,
    quality: String,
    format: String,
    detector: &video_dl::platform::PlatformDetector,
) -> Result<()> {
    println!("Fetching video information...");
    let url = normalize_url(&url)?;
    let platform = detector.detect(&url)?;
    
    // Get video information
    let info = platform.extract_info(&url).await?;
    
    // Select the best format matching the requested quality and format
    let selected_format = info.formats
        .iter()
        .find(|f| {
            f.format.to_string().eq_ignore_ascii_case(&format) &&
            f.quality.to_string().eq_ignore_ascii_case(&quality)
        })
        .ok_or_else(|| Error::InvalidFormat(
            format!("No format found for quality {} and format {}", quality, format)
        ))?;

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let filename = format!(
            "{}-{}.{}",
            sanitize_filename::sanitize(&info.title),
            selected_format.quality.to_string().to_lowercase(),
            format.to_lowercase()
        );
        PathBuf::from(filename)
    });

    println!("Starting download of '{}' in {} {}", 
        info.title, 
        selected_format.quality,
        selected_format.format
    );

    // Create progress channel
    let (progress_tx, _progress_rx) = tokio::sync::watch::channel(0.0);
    let progress_tx = Arc::new(progress_tx);

    // Set up progress bar
    let pb = Arc::new(indicatif::ProgressBar::new(100));
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{wide_bar:.cyan/blue}] {percent}% ({eta})")
            .unwrap()
            .progress_chars("#>-")
    );

    let progress_tx_clone = Arc::clone(&progress_tx);
    let pb_clone = Arc::clone(&pb);
    tokio::spawn(async move {
        let mut rx = progress_tx_clone.subscribe();
        while rx.changed().await.is_ok() {
            let progress = *rx.borrow();
            pb_clone.set_position((progress * 100.0) as u64);
        }
    });

    // Start download
    platform.download_video(&info, &selected_format.id, &output_path, progress_tx).await?;
    pb.finish_with_message("Download complete");

    println!("\n✓ Download completed: {}", output_path.display());
    Ok(())
}

async fn handle_info(url: String, detector: &video_dl::platform::PlatformDetector) -> Result<()> {
    let url = normalize_url(&url)?;
    let platform = detector.detect(&url)?;
    
    println!("Fetching video information...");
    let info = platform.extract_info(&url).await?;
    
    println!("\nTitle: {}", info.title);
    if let Some(desc) = &info.description {
        println!("Description: {}", desc);
    }
    if let Some(duration) = info.duration {
        println!("Duration: {} seconds", duration);
    }
    
    println!("\nAvailable formats:");
    for format in info.formats {
        println!(
            "- Format ID: {}\n  Quality: {}\n  Format: {}\n  Size: {}\n",
            format.id,
            format.quality,
            format.format,
            format.file_size.map(|s| format!("{} bytes", s)).unwrap_or_else(|| "unknown".to_string())
        );
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Create platform detector
    let detector = create_platform_detector();

    match cli.command {
        Commands::Download { url, output, quality, format } => {
            handle_download(url, output, quality, format, &detector).await
        }
        Commands::Info { url } => {
            handle_info(url, &detector).await
        }
    }
}
