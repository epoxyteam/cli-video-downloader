use std::sync::Arc;
use tokio::sync::watch;
use indicatif::{ProgressBar, ProgressStyle};

/// Progress tracker for downloads with visual progress bar
pub struct ProgressTracker {
    progress_tx: Arc<watch::Sender<f64>>,
    progress_bar: ProgressBar,
    _handle: Option<tokio::task::JoinHandle<()>>,
}

impl ProgressTracker {
    /// Creates a new progress tracker with a styled progress bar
    pub fn new() -> Self {
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {percent}% ({eta})")
                .unwrap()
                .progress_chars("=>-")
        );

        let (progress_tx, mut progress_rx) = watch::channel(0.0);
        let progress_tx = Arc::new(progress_tx);
        
        let pb_handle = tokio::spawn({
            let pb = pb.clone();
            async move {
                while progress_rx.changed().await.is_ok() {
                    let progress = *progress_rx.borrow();
                    pb.set_position((progress * 100.0) as u64);
                }
            }
        });

        Self {
            progress_tx: progress_tx.clone(),
            progress_bar: pb,
            _handle: Some(pb_handle),
        }
    }

    /// Get the progress sender that can be passed to platform implementations
    pub fn get_sender(&self) -> Arc<watch::Sender<f64>> {
        self.progress_tx.clone()
    }

    /// Completes and clears the progress bar
    pub fn finish(mut self) {
        if let Some(handle) = self._handle.take() {
            handle.abort();
        }
        self.progress_bar.finish_and_clear();
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        if let Some(handle) = self._handle.take() {
            handle.abort();
        }
    }
}