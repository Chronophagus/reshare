use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use tokio::{sync::mpsc, task::JoinHandle};

const CHANNEL_SIZE: usize = 40000;

pub struct ProgressUpdate {
    pub file_name: String,
    pub bytes_transmitted: u64,
}

pub type ProgressReporter = mpsc::Sender<ProgressUpdate>;

pub struct ProgressTracker {
    multiprogress: MultiProgress,
    bar_map: HashMap<String, ProgressBar>,
    reporter: mpsc::Sender<ProgressUpdate>,
    monitor: mpsc::Receiver<ProgressUpdate>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
        Self {
            multiprogress: MultiProgress::new(),
            bar_map: HashMap::new(),
            reporter: tx,
            monitor: rx,
        }
    }

    pub fn get_reporter(&self) -> ProgressReporter {
        self.reporter.clone()
    }

    /// Shows progress bars and starts monitoring for progress updates
    /// This function must be called from the context of Tokio runtime
    pub fn spawn(self) -> JoinHandle<()> {
        let ProgressTracker {
            multiprogress,
            bar_map,
            reporter,
            mut monitor,
        } = self;

        drop(reporter);

        let join_handle = tokio::task::spawn_blocking(move || {
            let _ = multiprogress.join();
        });

        tokio::spawn(async move {
            while let Some(progress_update) = monitor.recv().await {
                if let Some(progress_bar) = bar_map.get(&progress_update.file_name) {
                    progress_bar.inc(progress_update.bytes_transmitted);
                }
            }

            for (_, bar) in bar_map {
                bar.abandon();
            }
        });

        join_handle
    }

    pub fn add_bar(&mut self, file_name: String, file_size: u64) {
        let progress_bar = self.multiprogress.add(ProgressBar::new(file_size));

        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] {prefix} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} | {bytes_per_sec} (finishes in {eta})",
                )
                .progress_chars("=>-"),
        );

        progress_bar.set_prefix(&file_name);
        self.bar_map.insert(file_name, progress_bar);
    }
}
