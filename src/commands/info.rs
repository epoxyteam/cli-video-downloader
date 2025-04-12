use crate::{Result, Downloader, Config};

/// Handles the info command execution
pub async fn info_command(url: String) -> Result<()> {
    // Load configuration
    let config = Config::load();
    let downloader = Downloader::with_config(config);
    
    println!("Fetching video information...");
    let info = downloader.get_video_info(&url).await?;
    
    println!("\nTitle: {}", info.title);
    if let Some(desc) = &info.description {
        // Print only the first few lines of the description if it's long
        let desc_preview = desc.lines().take(5).collect::<Vec<_>>().join("\n");
        let is_truncated = desc.lines().count() > 5;
        
        println!("Description: {}{}", 
            desc_preview, 
            if is_truncated { "\n[... description truncated ...]" } else { "" }
        );
    }
    if let Some(duration) = info.duration {
        // Format duration in HH:MM:SS format
        let hours = duration / 3600;
        let minutes = (duration % 3600) / 60;
        let seconds = duration % 60;
        println!("Duration: {:02}:{:02}:{:02}", hours, minutes, seconds);
    }
    
    println!("\nAvailable formats:");
    
    // Group formats by quality for better display
    let mut quality_groups: std::collections::HashMap<String, Vec<&crate::VideoFormat>> = std::collections::HashMap::new();
    
    for format in &info.formats {
        let quality = format.quality.to_string();
        quality_groups.entry(quality).or_default().push(format);
    }
    
    // Display formats grouped by quality
    for (quality, formats) in quality_groups.iter() {
        println!("\n{} quality:", quality);
        for format in formats {
            println!(
                "  - Format ID: {}\n    Format: {}\n    Size: {}\n",
                format.id,
                format.format,
                format.file_size.map(|s| format!("{} bytes", s)).unwrap_or_else(|| "unknown".to_string())
            );
        }
    }
    
    println!("\nTo download this video:");
    println!("  video-dl download -u {} -f FORMAT_ID", url);
    
    Ok(())
}