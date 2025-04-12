use std::path::Path;
use tokio::process::Command;
use crate::Config;

/// Dependency check result
pub struct DependencyStatus {
    pub yt_dlp_available: bool,
    pub yt_dlp_version: Option<String>,
    pub ffmpeg_available: bool,
    pub ffmpeg_version: Option<String>,
}

/// Check if required dependencies are available
pub async fn check_dependencies(config: &Config) -> DependencyStatus {
    let yt_dlp_cmd = match &config.ytdlp_path {
        Some(path) => path.to_str().unwrap_or("yt-dlp").to_string(),
        None => "yt-dlp".to_string(),
    };

    // Check for yt-dlp
    let yt_dlp_result = Command::new(&yt_dlp_cmd)
        .arg("--version")
        .output()
        .await;
    
    let yt_dlp_available = yt_dlp_result.is_ok();
    let yt_dlp_version = match yt_dlp_result {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_string()
                .into()
        }
        _ => None
    };

    // Check for ffmpeg (needed for merging formats)
    let ffmpeg_result = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .await;
    
    let ffmpeg_available = ffmpeg_result.is_ok();
    let ffmpeg_version = match ffmpeg_result {
        Ok(output) if output.status.success() => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Extract just the version number from the output
            let version_line = output_str.lines().next().unwrap_or("");
            if let Some(version_start) = version_line.find("version ") {
                let version_str = &version_line[(version_start + 8)..];
                if let Some(end) = version_str.find(' ') {
                    Some(version_str[..end].to_string())
                } else {
                    Some(version_str.to_string())
                }
            } else {
                Some("Unknown version".to_string())
            }
        }
        _ => None
    };

    DependencyStatus {
        yt_dlp_available,
        yt_dlp_version,
        ffmpeg_available,
        ffmpeg_version,
    }
}

/// Print dependency check results to console
pub fn print_dependency_status(status: &DependencyStatus) {
    println!("Dependency check:");
    
    println!("  yt-dlp: {}", if status.yt_dlp_available {
        format!("✓ Available (v{})", status.yt_dlp_version.as_deref().unwrap_or("unknown"))
    } else {
        "✗ Not found. Please install yt-dlp: https://github.com/yt-dlp/yt-dlp#installation".to_string()
    });
    
    println!("  ffmpeg: {}", if status.ffmpeg_available {
        format!("✓ Available (v{})", status.ffmpeg_version.as_deref().unwrap_or("unknown"))
    } else {
        "✗ Not found. Some formats may not be available without ffmpeg.".to_string()
    });
}

/// Check if all required dependencies are available
pub fn all_dependencies_available(status: &DependencyStatus) -> bool {
    status.yt_dlp_available && status.ffmpeg_available
}

/// Check if minimum required dependencies are available (just yt-dlp)
pub fn minimum_dependencies_available(status: &DependencyStatus) -> bool {
    status.yt_dlp_available
}