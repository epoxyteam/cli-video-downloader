use crate::{Result, Config};
use std::str::FromStr;
use std::path::PathBuf;
use super::ConfigAction;

/// Handles the config command execution
pub async fn config_command(action: Option<ConfigAction>) -> Result<()> {
    // Load current configuration
    let mut config = Config::load();
    
    match action {
        Some(ConfigAction::Get { key }) => {
            if let Some(key) = key {
                // Get a specific config value
                match key.as_str() {
                    "download_dir" => println!("download_dir: {:?}", config.download_dir),
                    "default_quality" => println!("default_quality: {}", config.default_quality),
                    "default_format" => println!("default_format: {}", config.default_format),
                    "show_progress" => println!("show_progress: {}", config.show_progress),
                    "overwrite_files" => println!("overwrite_files: {}", config.overwrite_files),
                    "ytdlp_path" => println!("ytdlp_path: {:?}", config.ytdlp_path),
                    _ => println!("Unknown configuration key: {}", key),
                }
            } else {
                // Show all config values
                println!("Current configuration:");
                println!("  download_dir: {:?}", config.download_dir);
                println!("  default_quality: {}", config.default_quality);
                println!("  default_format: {}", config.default_format);
                println!("  show_progress: {}", config.show_progress);
                println!("  overwrite_files: {}", config.overwrite_files);
                if let Some(path) = &config.ytdlp_path {
                    println!("  ytdlp_path: {:?}", path);
                } else {
                    println!("  ytdlp_path: Using system PATH");
                }
            }
        },
        Some(ConfigAction::Set { key, value }) => {
            // Set a specific config value
            match key.as_str() {
                "download_dir" => {
                    let path = PathBuf::from(&value);
                    config.download_dir = path;
                    println!("Updated download_dir to {:?}", config.download_dir);
                },
                "default_quality" => {
                    config.default_quality = value;
                    println!("Updated default_quality to {}", config.default_quality);
                },
                "default_format" => {
                    config.default_format = value;
                    println!("Updated default_format to {}", config.default_format);
                },
                "show_progress" => {
                    if let Ok(val) = bool::from_str(&value) {
                        config.show_progress = val;
                        println!("Updated show_progress to {}", config.show_progress);
                    } else {
                        println!("Invalid value for show_progress. Use 'true' or 'false'");
                    }
                },
                "overwrite_files" => {
                    if let Ok(val) = bool::from_str(&value) {
                        config.overwrite_files = val;
                        println!("Updated overwrite_files to {}", config.overwrite_files);
                    } else {
                        println!("Invalid value for overwrite_files. Use 'true' or 'false'");
                    }
                },
                "ytdlp_path" => {
                    if value.to_lowercase() == "none" {
                        config.ytdlp_path = None;
                        println!("Cleared ytdlp_path, will use system PATH");
                    } else {
                        let path = PathBuf::from(&value);
                        config.ytdlp_path = Some(path);
                        println!("Updated ytdlp_path to {:?}", config.ytdlp_path);
                    }
                },
                _ => {
                    println!("Unknown configuration key: {}", key);
                    println!("Available keys: download_dir, default_quality, default_format, show_progress, overwrite_files, ytdlp_path");
                    return Ok(());
                }
            }
            
            // Save updated configuration
            if let Err(e) = config.save() {
                println!("Failed to save configuration: {}", e);
            } else {
                println!("Configuration saved successfully");
            }
        },
        Some(ConfigAction::Reset) => {
            // Reset to default configuration
            config = Config::default();
            println!("Configuration reset to defaults");
            
            // Save reset configuration
            if let Err(e) = config.save() {
                println!("Failed to save configuration: {}", e);
            } else {
                println!("Configuration saved successfully");
            }
        },
        None => {
            // Show usage information
            println!("Config command usage:");
            println!("  video-dl config get [--key KEY]  - Show all config or a specific value");
            println!("  video-dl config set --key KEY --value VALUE  - Set a config value");
            println!("  video-dl config reset  - Reset configuration to defaults");
            println!("\nAvailable config keys:");
            println!("  download_dir      - Directory where videos are downloaded");
            println!("  default_quality   - Default video quality (best, low, medium, high, 720p, 1080p, etc.)");
            println!("  default_format    - Default video format (mp4, webm, etc.)");
            println!("  show_progress     - Whether to show progress bars (true/false)");
            println!("  overwrite_files   - Whether to overwrite existing files (true/false)");
            println!("  ytdlp_path        - Path to yt-dlp executable, or 'none' to use system PATH");
        }
    }
    
    Ok(())
}