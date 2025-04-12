use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use futures::StreamExt;
use futures::Stream;
use futures::TryStreamExt;
use crate::{Error, Result};

pub struct Downloader {
    client: Client,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Downloads a file from a URL with progress indication
    pub async fn download_file(
        &self,
        url: &str,
        output_path: &Path,
        progress_callback: impl Fn(f64) + Send + 'static,
    ) -> Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Start the download
        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Network(e))?;

        let total_size = response.content_length().unwrap_or(0);

        // Create progress bar
        let progress_bar = if total_size > 0 {
            let pb = ProgressBar::new(total_size);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"));
            Some(pb)
        } else {
            None
        };

        // Open the output file
        let mut file = File::create(output_path).await?;
        let mut downloaded = 0u64;

        // Download chunks and write to file
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.try_next().await? {
            file.write_all(&chunk).await?;
            
            downloaded += chunk.len() as u64;
            if let Some(pb) = &progress_bar {
                pb.set_position(downloaded);
            }

            if total_size > 0 {
                progress_callback(downloaded as f64 / total_size as f64);
            }
        }

        if let Some(pb) = progress_bar {
            pb.finish_with_message("Download complete");
        }

        Ok(())
    }

    /// Creates a new temporary file with a random name in the system's temp directory
    pub async fn create_temp_file() -> Result<(File, std::path::PathBuf)> {
        let temp_dir = std::env::temp_dir();
        let random_filename = format!("video-dl-{}", uuid::Uuid::new_v4());
        let temp_path = temp_dir.join(random_filename);
        
        let file = File::create(&temp_path).await?;
        Ok((file, temp_path))
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_create_temp_file() {
        let result = Downloader::create_temp_file().await;
        assert!(result.is_ok());
        
        let (_, path) = result.unwrap();
        assert!(path.exists());
        
        // Cleanup
        let _ = tokio::fs::remove_file(path).await;
    }
}