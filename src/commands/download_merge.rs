use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use tokio::process::Command;
use indicatif::{ProgressBar, ProgressStyle};
use futures_util::future::join_all;
use tokio::task;
use uuid::Uuid;
use sanitize_filename;

use crate::{Result, Error, Config, Downloader};
use crate::utils::dependency_check;
use crate::commands::batch::download_single_video;

/// Downloads multiple videos and automatically merges them into a single output file
pub async fn download_merge_command(
    urls: Vec<String>,
    file_path: Option<PathBuf>,
    output: PathBuf,
    quality: String,
    format: String,
    parallel: bool
) -> Result<()> {
    // Check dependencies
    let config = Config::load();
    let status = dependency_check::check_dependencies(&config).await;
    
    if !status.ffmpeg_available {
        return Err(Error::CommandExecution {
            command: "ffmpeg".to_string(),
            reason: "ffmpeg is required for merging videos. Please install it first.".to_string()
        });
    }

    // Create temporary directory for downloaded videos
    let temp_dir = std::env::temp_dir().join(format!("video_dl_merge_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| Error::IoError(format!("Failed to create temporary directory: {}", e)))?;
    
    println!("Using temporary directory: {}", temp_dir.display());
    
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
    
    if all_urls.len() < 2 {
        return Err(Error::InvalidArgument("At least two URLs are required for download-merge".into()));
    }
    
    println!("Step 1/2: Downloading {} videos", all_urls.len());
    
    // Track downloaded files to merge later
    let mut downloaded_files = Vec::new();
    
    if parallel {
        // Process URLs in parallel
        let multi_progress = indicatif::MultiProgress::new();
        
        let download_tasks = all_urls.into_iter().enumerate().map(|(index, url)| {
            let downloader_clone = downloader.clone();
            let quality = quality.clone();
            let format = format.clone();
            let temp_dir = temp_dir.clone();
            let progress_bar = multi_progress.add(ProgressBar::new(100));
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40.cyan/blue} {percent}% {msg}")
                    .expect("Valid progress bar template")
            );
            progress_bar.set_message(format!("Downloading: {}", url));
            
            task::spawn(async move {
                let output_dir = Some(temp_dir);
                let result = download_single_video(&downloader_clone, &url, output_dir, &quality, &format).await;
                
                if let Ok(path) = &result {
                    progress_bar.finish_with_message(format!("Downloaded: {}", path.file_name().unwrap_or_default().to_string_lossy()));
                } else {
                    progress_bar.abandon_with_message(format!("Failed: {}", url));
                }
                
                // Return tuple with index to maintain order
                (index, result)
            })
        }).collect::<Vec<_>>();
        
        // Wait for all downloads to complete
        let results = join_all(download_tasks).await;
        
        // Process results while maintaining order
        let mut downloaded_files_with_index: Vec<(usize, PathBuf)> = Vec::new();
        
        for result in results {
            if let Ok((index, download_result)) = result {
                if let Ok(path) = download_result {
                    downloaded_files_with_index.push((index, path));
                }
            }
        }
        
        // Sort by the original index to maintain the order
        downloaded_files_with_index.sort_by_key(|(index, _)| *index);
        
        // Add successful downloads to the list in original order
        downloaded_files = downloaded_files_with_index.into_iter()
            .map(|(_, path)| path)
            .collect();
    } else {
        // Process URLs sequentially
        for url in all_urls {
            println!("Downloading {}...", url);
            match download_single_video(&downloader, &url, Some(temp_dir.clone()), &quality, &format).await {
                Ok(path) => {
                    println!("✓ Success: {}", path.file_name().unwrap_or_default().to_string_lossy());
                    downloaded_files.push(path);
                },
                Err(e) => {
                    println!("✗ Failed: {}", e);
                }
            }
        }
    }
    
    // Check if we have files to merge
    if downloaded_files.len() < 2 {
        return Err(Error::DownloadFailed {
            reason: format!("Not enough videos were successfully downloaded. Only got {} out of minimum 2 required.", 
                downloaded_files.len())
        });
    }
    
    // Sort files by name to ensure consistent order (usually this will match the input order)
    downloaded_files.sort_by(|a, b| {
        a.file_name().unwrap_or_default().cmp(&b.file_name().unwrap_or_default())
    });
    
    println!("Step 2/2: Merging {} downloaded videos into one file", downloaded_files.len());
    
    // Create a temporary file for the concat list
    let concat_file = temp_dir.join(format!("concat_list_{}.txt", Uuid::new_v4()));
    let mut file = File::create(&concat_file)
        .map_err(|e| Error::IoError(format!("Failed to create temporary file: {}", e)))?;
    
    // Write the file list in ffmpeg's concat format
    for input_file in &downloaded_files {
        if let Some(path_str) = input_file.to_str() {
            // Escape single quotes in filenames
            let escaped_path = path_str.replace("'", "'\\''");
            writeln!(file, "file '{}'", escaped_path)
                .map_err(|e| Error::IoError(format!("Failed to write to temporary file: {}", e)))?;
        } else {
            return Err(Error::InvalidOutputPath(input_file.clone()));
        }
    }
    
    file.flush().map_err(|e| Error::IoError(format!("Failed to flush temporary file: {}", e)))?;
    
    // Create a progress bar for merging
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {percent}% {msg}")
            .expect("Valid progress bar template")
    );
    pb.set_message("Merging videos...");
    
    // Run ffmpeg to concatenate the videos
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-f", "concat", "-safe", "0", "-i"])
       .arg(&concat_file)
       .args(["-c", "copy"]) // Copy streams without re-encoding
       .arg(&output);
       
    let output_result = cmd.output().await
        .map_err(|e| Error::CommandExecution {
            command: "ffmpeg".to_string(),
            reason: e.to_string()
        })?;
    
    // Check if the merge was successful
    if !output_result.status.success() {
        let error = String::from_utf8_lossy(&output_result.stderr);
        pb.abandon_with_message("Merge failed");
        
        // Clean up temporary directory
        let _ = std::fs::remove_dir_all(&temp_dir);
        
        return Err(Error::CommandExecution {
            command: "ffmpeg".to_string(),
            reason: error.to_string()
        });
    }
    
    pb.finish_with_message(format!("Successfully merged {} videos into {}", downloaded_files.len(), output.display()));
    
    // Clean up temporary directory
    println!("Cleaning up temporary files...");
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    Ok(())
}