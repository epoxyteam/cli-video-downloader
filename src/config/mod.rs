use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to yt-dlp executable, if not in PATH
    pub ytdlp_path: Option<PathBuf>,
    
    /// Default download directory
    pub download_dir: PathBuf,
    
    /// Default video quality
    pub default_quality: String,
    
    /// Default video format
    pub default_format: String,
    
    /// Whether to show progress bars
    pub show_progress: bool,
    
    /// Whether to overwrite existing files
    pub overwrite_files: bool,
}

impl Config {
    /// Load config from file or create default if not exists
    pub fn load() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("video-dl");
        
        let config_path = config_dir.join("config.toml");
        
        if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(content) => {
                    match toml::from_str(&content) {
                        Ok(config) => return config,
                        Err(_) => eprintln!("Failed to parse config file, using defaults"),
                    }
                },
                Err(_) => eprintln!("Failed to read config file, using defaults"),
            }
        }
        
        // Default configuration
        let config = Self::default();
        
        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            let _ = fs::create_dir_all(&config_dir);
        }
        
        // Save default config
        if let Ok(content) = toml::to_string_pretty(&config) {
            let _ = fs::write(&config_path, content);
        }
        
        config
    }
    
    /// Save config to file
    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("video-dl");
            
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        
        let config_path = config_dir.join("config.toml");
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            
        fs::write(config_path, content)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ytdlp_path: None,
            download_dir: dirs::download_dir().unwrap_or_else(|| PathBuf::from(".")),
            default_quality: "best".to_string(),
            default_format: "mp4".to_string(),
            show_progress: true,
            overwrite_files: false,
        }
    }
}