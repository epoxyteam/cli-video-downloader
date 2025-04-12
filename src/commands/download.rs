use std::path::PathBuf;
use crate::{Error, Result, Downloader};

/// Handles the download command execution
pub async fn download_command(
    url: String, 
    output: Option<PathBuf>, 
    quality: String, 
    format: String
) -> Result<()> {
    let downloader = Downloader::new();
    
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

    let output_path = output.unwrap_or_else(|| {
        let filename = format!(
            "{}-{}.{}",
            sanitize_filename::sanitize(&info.title),
            selected_format.quality.to_string().to_lowercase(),
            format.to_lowercase()
        );
        PathBuf::from(filename)
    });

    println!("Starting download of '{}' in {} {} (format ID: {})", 
        info.title, selected_format.quality, selected_format.format, selected_format.id
    );

    downloader.download(&url, &selected_format.id, output_path).await?;
    println!("\n✓ Download completed");
    Ok(())
}