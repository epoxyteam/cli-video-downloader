use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use tokio::process::Command;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{Result, Error, Config};
use crate::utils::dependency_check;

/// Handles merging multiple videos into one
pub async fn merge_command(
    files: Vec<PathBuf>,
    file_list: Option<PathBuf>,
    output: PathBuf
) -> Result<()> {
    // Check if ffmpeg is available - required for merging
    let config = Config::load();
    let status = dependency_check::check_dependencies(&config).await;
    
    if !status.ffmpeg_available {
        return Err(Error::CommandExecution {
            command: "ffmpeg".to_string(),
            reason: "ffmpeg is required for merging videos. Please install it first.".to_string()
        });
    }
    
    // Collect all files to process
    let mut all_files: Vec<PathBuf> = Vec::new();
    
    // Add files from command line arguments
    all_files.extend(files);
    
    // Add files from list file if provided
    if let Some(list_path) = file_list {
        let file = File::open(&list_path)
            .map_err(|e| Error::IoError(format!("Failed to open file list: {}", e)))?;
        
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let path_str = line.map_err(|e| Error::IoError(format!("Failed to read file list: {}", e)))?;
            let path = PathBuf::from(path_str.trim());
            if path.exists() {
                all_files.push(path);
            } else {
                eprintln!("Warning: File not found: {}", path.display());
            }
        }
    }
    
    // Check if we have enough files
    if all_files.len() < 2 {
        return Err(Error::InvalidArgument("At least two input files are required for merging".into()));
    }
    
    println!("Merging {} videos into: {}", all_files.len(), output.display());
    
    // Create a temporary file for the concat list
    let temp_file = PathBuf::from(format!("concat_list_{}.txt", uuid::Uuid::new_v4()));
    let mut file = File::create(&temp_file)
        .map_err(|e| Error::IoError(format!("Failed to create temporary file: {}", e)))?;
    
    // Write the file list in ffmpeg's concat format
    for input_file in &all_files {
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
    
    // Create a progress bar
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
       .arg(&temp_file)
       .args(["-c", "copy"]) // Copy streams without re-encoding
       .arg(&output);
       
    let output_result = cmd.output().await
        .map_err(|e| Error::CommandExecution {
            command: "ffmpeg".to_string(),
            reason: e.to_string()
        })?;
    
    // Clean up the temporary file
    if temp_file.exists() {
        let _ = std::fs::remove_file(&temp_file);
    }
    
    // Check if the merge was successful
    if !output_result.status.success() {
        let error = String::from_utf8_lossy(&output_result.stderr);
        pb.abandon_with_message("Merge failed");
        return Err(Error::CommandExecution {
            command: "ffmpeg".to_string(),
            reason: error.to_string()
        });
    }
    
    pb.finish_with_message(format!("Successfully merged {} videos into {}", all_files.len(), output.display()));
    
    Ok(())
}