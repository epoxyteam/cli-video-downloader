use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader};
use futures::future::join_all;
use tokio::task;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::{Result, Error, Config, Downloader};

/// Helper function to handle downloading a single video within the batch
async fn download_single_video(
    downloader: &Downloader,
    url: &str,
    output_dir: Option<PathBuf>,
    quality: &str,
    format: &str
) -> Result<PathBuf> {
    // Get video info
    let info = downloader.get_video_info(url).await?;
    
    // Format selection logic
    let mut selected_format = info.formats.iter()
        .find(|f| {
            f.format.to_string().eq_ignore_ascii_case(format) &&
            f.quality.to_string().eq_ignore_ascii_case(quality)
        });
            
    // If exact match not found, try to find a format with the requested quality
    if selected_format.is_none() {
        selected_format = info.formats.iter()
            .find(|f| f.quality.to_string().eq_ignore_ascii_case(quality));
    }
    
    // If still not found, find closest quality with the requested format
    if selected_format.is_none() && !quality.eq_ignore_ascii_case("best") {
        selected_format = info.formats.iter()
            .find(|f| f.format.to_string().eq_ignore_ascii_case(format));
    }
    
    // If still not found, just pick the first format (best quality)
    let selected_format = selected_format
        .or_else(|| info.formats.first())
        .ok_or_else(|| Error::NoSuitableFormats)?;

    // Prepare output path if directory is specified
    let output = match output_dir {
        Some(dir) => {
            let filename = format!("{}.mp4", sanitize_filename::sanitize(&info.title));
            Some(dir.join(filename))
        },
        None => None
    };
    
    // Download the video and return the output path
    downloader.download(url, &selected_format.id, output).await
}

/// Handles batch download command execution
pub async fn batch_download_command(
    urls: Vec<String>,
    file_path: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    quality: String,
    format: String,
    parallel: bool
) -> Result<()> {
    // Load configuration
    let config = Config::load();
    
    // Use provided quality/format or fall back to config defaults
    let quality = if quality == "best" { config.default_quality.clone() } else { quality };
    let format = if format == "mp4" { config.default_format.clone() } else { format };
    
    // Create downloader
    let downloader = Downloader::with_config(config.clone());
    
    // Collect all URLs to process
    let mut all_urls = Vec::new();
    
    // Add URLs from command line arguments
    all_urls.extend(urls);
    
    // Add URLs from file if provided
    if let Some(path) = file_path {
        let file = File::open(path)
            .map_err(|e| Error::IoError(format!("Failed to open URL file: {}", e)))?;
        
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let url = line.map_err(|e| Error::IoError(format!("Failed to read URL file: {}", e)))?;
            let trimmed = url.trim();
            if !trimmed.is_empty() {
                all_urls.push(trimmed.to_string());
            }
        }
    }
    
    if all_urls.is_empty() {
        return Err(Error::InvalidArgument("No URLs provided for download".into()));
    }
    
    println!("Starting batch download of {} videos", all_urls.len());
    
    if parallel {
        // Process URLs in parallel
        let multi_progress = MultiProgress::new();
        
        let download_tasks = all_urls.into_iter().map(|url| {
            let downloader_clone = downloader.clone();
            let quality = quality.clone();
            let format = format.clone();
            let output_dir = output_dir.clone();
            let progress_bar = multi_progress.add(ProgressBar::new(100));
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40.cyan/blue} {percent}% {msg}")
                    .expect("Valid progress bar template")
            );
            progress_bar.set_message(format!("Downloading: {}", url));
            
            task::spawn(async move {
                let result = match downloader_clone.get_video_info(&url).await {
                    Ok(info) => {
                        progress_bar.set_message(format!("Downloading: {}", info.title));
                        download_single_video(&downloader_clone, &url, output_dir, &quality, &format).await
                    },
                    Err(e) => Err(e)
                };
                
                if result.is_ok() {
                    progress_bar.finish_with_message(format!("Completed: {}", url));
                } else {
                    progress_bar.abandon_with_message(format!("Failed: {}", url));
                }
                
                (url, result)
            })
        }).collect::<Vec<_>>();
        
        // Wait for all downloads to complete
        let results = join_all(download_tasks).await;
        
        // Summarize results
        let mut success_count = 0;
        let mut failure_count = 0;
        
        println!("\nBatch download summary:");
        
        for result in results {
            match result {
                Ok((url, Ok(path))) => {
                    println!("✓ Successfully downloaded: {} -> {:?}", url, path);
                    success_count += 1;
                },
                Ok((url, Err(e))) => {
                    println!("✗ Failed to download {}: {}", url, e);
                    failure_count += 1;
                },
                Err(e) => {
                    println!("✗ Task failed: {}", e);
                    failure_count += 1;
                }
            }
        }
        
        println!("\nBatch download complete: {} successful, {} failed", success_count, failure_count);
    } else {
        // Process URLs sequentially
        let mut success_count = 0;
        let mut failure_count = 0;
        
        for url in all_urls {
            println!("Downloading {}... ", url);
            match download_single_video(&downloader, &url, output_dir.clone(), &quality, &format).await {
                Ok(path) => {
                    println!("✓ Success: {:?}", path);
                    success_count += 1;
                },
                Err(e) => {
                    println!("✗ Failed: {}", e);
                    failure_count += 1;
                }
            }
        }
        
        println!("\nBatch download complete: {} successful, {} failed", success_count, failure_count);
    }
    
    Ok(())
}