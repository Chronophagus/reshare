use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;

#[derive(Clone)]
pub struct ProgressTracker {
    multiprogress: Arc<MultiProgress>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            multiprogress: Arc::new(MultiProgress::new()),
        }
    }

    // Blocking call. Typically should be called in a separate thread
    pub fn show(&self) {
        let _ = self.multiprogress.join();
    }

    pub fn add(&self, file_name: &str, file_size: u64) -> ProgressBar {
        let progress_bar = self.multiprogress.add(ProgressBar::new(file_size));

        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] {prefix} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} | {bytes_per_sec} (finishes in {eta})",
                )
                .progress_chars("=>-"),
        );

        progress_bar.set_prefix(file_name);
        progress_bar
    }
}
