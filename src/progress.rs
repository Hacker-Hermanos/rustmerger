// Import required dependencies
use anyhow::Result; // For error handling
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize}; // For JSON serialization/deserialization
use std::path::PathBuf; // For file path handling
use std::time::{Duration, Instant};
use tokio::fs; // For async file operations

// Metrics tracking structures
pub struct ProcessingMetrics {
    start_time: Instant,
    files_processed: usize,
    lines_processed: usize,
    errors_count: usize,
}

impl ProcessingMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            files_processed: 0,
            lines_processed: 0,
            errors_count: 0,
        }
    }

    pub fn increment_files(&mut self) {
        self.files_processed += 1;
    }

    pub fn add_lines(&mut self, count: usize) {
        self.lines_processed += count;
    }

    pub fn get_summary(&self) -> ProcessingSummary {
        ProcessingSummary {
            elapsed_time: self.start_time.elapsed(),
            files_processed: self.files_processed,
            lines_processed: self.lines_processed,
            errors_count: self.errors_count,
            memory_usage: 0,
        }
    }
}

pub struct ProcessingSummary {
    pub elapsed_time: Duration,
    pub files_processed: usize,
    pub lines_processed: usize,
    pub errors_count: usize,
    pub memory_usage: usize,
}

// Progress tracking structure that can be serialized to/from JSON
#[derive(Debug, Serialize, Deserialize)]
pub struct Progress {
    pub input_file: PathBuf,  // Source file containing list of files to process
    pub output_file: PathBuf, // Destination file for merged content
    pub threads: usize,       // Number of parallel processing threads
    pub processed_files: Vec<PathBuf>, // List of successfully processed files
    pub current_position: usize, // Current processing position for resume capability
    pub save_path: Option<PathBuf>, // Path where progress state is saved
}

// Implement Default trait for Progress
impl Default for Progress {
    fn default() -> Self {
        Self {
            input_file: PathBuf::new(),
            output_file: PathBuf::new(),
            threads: 10, // Default to 10 threads
            processed_files: Vec::new(),
            current_position: 0,
            save_path: None,
        }
    }
}

impl Progress {
    // Save current progress state to JSON file
    pub async fn save(&self) -> Result<()> {
        if let Some(path) = &self.save_path {
            // Convert progress state to pretty-printed JSON
            let content = serde_json::to_string_pretty(&self)?;
            // Write to file asynchronously
            fs::write(path, content).await?;
        }
        Ok(())
    }

    // Load progress state from a JSON file
    pub async fn load(path: &PathBuf) -> Result<Self> {
        // Read file content asynchronously
        let content = fs::read_to_string(path).await?;
        // Parse JSON into Progress struct
        let mut progress: Progress = serde_json::from_str(&content)?;
        // Store save path for future updates
        progress.save_path = Some(path.clone());
        Ok(progress)
    }

    // Add a processed file to the progress tracking
    #[allow(dead_code)] // Suppress unused function warning
    pub async fn add_processed_file(&mut self, file: PathBuf) -> Result<()> {
        // Add file to processed list
        self.processed_files.push(file);
        // Increment position counter
        self.current_position += 1;
        // Save updated progress state
        self.save().await
    }
}

pub struct ProgressTracker {
    multi_progress: MultiProgress,
    overall_progress: ProgressBar,
    dedup_progress: ProgressBar,
    metrics: ProcessingMetrics,
    refresh_rate: Duration,
}

impl ProgressTracker {
    pub fn new(total_files: usize, estimated_lines: usize) -> Self {
        let multi = MultiProgress::new();

        // Overall progress bar style
        let overall_style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({percent}%) | {msg}")
            .unwrap()
            .progress_chars("#>-");

        // Deduplication progress bar style
        let dedup_style = ProgressStyle::default_bar()
            .template("{spinner:.yellow} [{elapsed_precise}] [{bar:40.yellow/blue}] {pos}/{len} lines | {msg}")
            .unwrap()
            .progress_chars("#>-");

        let overall_pb = multi.add(ProgressBar::new(total_files as u64));
        overall_pb.set_style(overall_style);

        let dedup_pb = multi.add(ProgressBar::new(estimated_lines as u64));
        dedup_pb.set_style(dedup_style);

        Self {
            multi_progress: multi,
            overall_progress: overall_pb,
            dedup_progress: dedup_pb,
            metrics: ProcessingMetrics::new(),
            refresh_rate: Duration::from_millis(100),
        }
    }

    pub fn update_overall_progress(&mut self, files_processed: usize) {
        self.metrics.increment_files();
        let summary = self.metrics.get_summary();

        self.overall_progress.set_position(files_processed as u64);
        self.overall_progress.set_message(format!(
            "Speed: {:.2} files/s | Memory: {:.2} MB | Errors: {}",
            files_processed as f64 / summary.elapsed_time.as_secs_f64(),
            summary.memory_usage as f64 / 1_048_576.0, // Convert bytes to MB
            summary.errors_count
        ));
    }

    pub fn update_dedup_progress(&mut self, lines_processed: usize, total_lines: usize) {
        self.metrics.add_lines(lines_processed);
        let summary = self.metrics.get_summary();

        self.dedup_progress.set_length(total_lines as u64);
        self.dedup_progress.set_position(lines_processed as u64);
        self.dedup_progress.set_message(format!(
            "Speed: {:.2} lines/s | Unique lines: {}",
            summary.lines_processed as f64 / summary.elapsed_time.as_secs_f64(),
            lines_processed
        ));
    }

    pub fn finish(&self) {
        let summary = self.metrics.get_summary();
        self.overall_progress.finish_with_message(format!(
            "Completed in {}s | Files: {} | Lines: {} | Errors: {}",
            summary.elapsed_time.as_secs(),
            summary.files_processed,
            summary.lines_processed,
            summary.errors_count
        ));
        self.dedup_progress.finish();
    }

    pub fn get_metrics(&self) -> &ProcessingMetrics {
        &self.metrics
    }
}
