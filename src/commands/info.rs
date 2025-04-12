use crate::{Result, Downloader};

/// Handles the info command execution
pub async fn info_command(url: String) -> Result<()> {
    let downloader = Downloader::new();
    
    println!("Fetching video information...");
    let info = downloader.get_video_info(&url).await?;
    
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