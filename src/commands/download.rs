use std::path::PathBuf;
use crate::{Error, Result, Downloader, Config};

/// Handles the download command execution
pub async fn download_command(
    url: String, 
    output: Option<PathBuf>, 
    quality: String, 
    format: String
) -> Result<()> {
    // Load configuration
    let config = Config::load();
    
    // Use provided quality/format or fall back to config defaults
    let quality = if quality == "best" { config.default_quality.clone() } else { quality };
    let format = if format == "mp4" { config.default_format.clone() } else { format };
    
    // Create downloader with config
    let downloader = Downloader::with_config(config);
    
    println!("Fetching video information...");
    let info = downloader.get_video_info(&url).await?;
    
    // Format selection logic
    let mut selected_format = info.formats.iter()
        .find(|f| {
            f.format.to_string().eq_ignore_ascii_case(&format) &&
            f.quality.to_string().eq_ignore_ascii_case(&quality)
        });
            
    // If exact match not found, try to find a format with the requested quality
    if selected_format.is_none() {
        selected_format = info.formats.iter()
            .find(|f| f.quality.to_string().eq_ignore_ascii_case(&quality));
    }
    
    // If still not found, find closest quality with the requested format
    if selected_format.is_none() && !quality.eq_ignore_ascii_case("best") {
        selected_format = info.formats.iter()
            .find(|f| f.format.to_string().eq_ignore_ascii_case(&format));
    }
    
    // If still not found, just pick the first format (best quality)
    let selected_format = selected_format
        .or_else(|| info.formats.first())
        .ok_or_else(|| Error::NoSuitableFormats)?;

    println!("Starting download of '{}' in {} {} (format ID: {})", 
        info.title, selected_format.quality, selected_format.format, selected_format.id
    );

    // Download returns the path where the file was saved
    let output_path = downloader.download(&url, &selected_format.id, output).await?;
    
    println!("\n✓ Download completed: {:?}", output_path);
    Ok(())
}