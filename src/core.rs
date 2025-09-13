use anyhow::Result; // Import Result type from anyhow crate for error handling
                    // Progress bar utilities are imported in progress.rs module
use crate::app_state::AppState;
use crate::encoding::EncodingHandler;
use crate::errors::MergerResult;
use crate::progress::ProgressTracker;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf}; // Import Path and PathBuf for file path handling
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc; // Import Arc for thread-safe reference counting
use sys_info;
use tokio::fs::File;
use tokio::fs::OpenOptions;
use tokio::io::SeekFrom;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufWriter};
use tokio::sync::mpsc; // Import encoding support for Issue #1 fix

const CHUNK_SIZE: usize = 1024 * 1024 * 10; // 10MB chunks
const BUFFER_SIZE: usize = 1024 * 1024 * 32; // 32MB buffer
const CHANNEL_SIZE: usize = 1000; // Number of chunks to keep in memory
const PARALLEL_FILES: usize = 4; // Number of files to process in parallel
const LINE_BUFFER_CAPACITY: usize = 1024 * 64; // 64KB initial line buffer
const OUTPUT_BUFFER_SIZE: usize = 1024 * 1024 * 16; // 16MB output buffer

// Define a struct to manage the core processing logic
#[allow(dead_code)]
pub struct ProcessingCore {
    app_state: Arc<AppState>, // Shared application state
    tracker: ProgressTracker, // Replace progress: MultiProgress with tracker
    verbose: bool,            // Flag to enable verbose logging
    debug: bool,              // Flag to enable debug mode
}

// Implement methods for ProcessingCore
impl ProcessingCore {
    // Asynchronous constructor for ProcessingCore
    pub async fn new(app_state: Arc<AppState>, verbose: bool, debug: bool) -> MergerResult<Self> {
        // Estimate total files and lines
        let input_file = &app_state.input_file;
        let content = tokio::fs::read_to_string(input_file).await?;
        let total_files = content.lines().count();

        // Rough estimation of lines (can be adjusted based on your needs)
        let estimated_lines = total_files * 1000; // Assuming average 1000 lines per file

        Ok(Self {
            app_state,
            tracker: ProgressTracker::new(total_files, estimated_lines),
            verbose,
            debug,
        })
    }

    // Main processing function
    pub async fn process(&mut self) -> MergerResult<()> {
        if self.verbose {
            println!("Starting the processing of files...");
        }

        let input_path = self.app_state.input_file.clone();
        let files = match Self::read_input_files(&input_path).await {
            Ok(f) => f,
            Err(e) => {
                self.log_error(&format!("Failed to read input files: {}", e))
                    .await?;
                return Ok(());
            }
        };

        let mut files_processed = 0;
        let app_state = Arc::clone(&self.app_state);

        for file in files {
            if app_state.should_shutdown().await {
                self.tracker.finish();
                return Ok(());
            }

            let file_path = file.clone();
            let result = self
                .process_single_file(file_path.clone(), &app_state)
                .await;
            if let Err(e) = result {
                let error_msg = format!("Error processing file {:?}: {}", file_path, e);
                self.log_error(&error_msg).await?;
                continue;
            }

            files_processed += 1;
            self.tracker.update_overall_progress(files_processed);
        }

        println!("Starting merge and deduplication process...");
        self.merge_and_deduplicate().await?;

        self.tracker.finish();
        println!("Processing completed successfully");

        Ok(())
    }

    // Function to merge files and remove duplicates
    async fn merge_and_deduplicate(&mut self) -> MergerResult<()> {
        let files = self
            .validate_and_collect_metadata(&self.app_state.progress.read().await.processed_files)
            .await?;
        let optimized_files = optimize_processing_order(files).await;

        // Calculate optimal batch size based on available system memory
        let mem_info = sys_info::mem_info()?;
        let available_memory = (mem_info.avail as usize * 1024) / 2;
        let batch_size = (available_memory / std::mem::size_of::<String>()).min(CHUNK_SIZE);

        let (tx, mut rx) = mpsc::channel::<HashSet<String>>(CHANNEL_SIZE);
        let unique_count = Arc::new(AtomicUsize::new(0));

        // Spawn writer task with optimized batching
        let writer_task = tokio::spawn({
            let unique_count = unique_count.clone();
            async move {
                let mut final_set = HashSet::with_capacity(batch_size);

                while let Some(mut chunk_set) = rx.recv().await {
                    final_set.extend(chunk_set.drain());
                    unique_count.store(final_set.len(), Ordering::Relaxed);
                }
                final_set
            }
        });

        // Process files in parallel with optimized ordering
        let mut total_lines_processed = 0;

        // Process files in chunks
        for chunk in optimized_files.chunks(PARALLEL_FILES) {
            let tx = tx.clone();
            let chunk_files = chunk.to_vec();

            for file in chunk_files {
                if let Ok(lines_count) =
                    Self::process_large_file(&file, tx.clone(), batch_size).await
                {
                    total_lines_processed += lines_count;
                    let current_unique = unique_count.load(Ordering::Relaxed);
                    self.tracker
                        .update_dedup_progress(current_unique, total_lines_processed);
                }
            }
        }

        drop(tx); // Close the channel

        // Get the final set and write results
        let unique_lines = writer_task.await?;
        let file = File::create(&self.app_state.output_file).await?;
        let mut writer = BufWriter::with_capacity(BUFFER_SIZE, file);
        let total_unique = unique_lines.len();

        println!("Writing {} unique lines to output file", total_unique);

        let mut buffer = String::with_capacity(CHUNK_SIZE);
        for line in unique_lines {
            buffer.push_str(&line);
            buffer.push('\n');

            if buffer.len() >= CHUNK_SIZE {
                writer.write_all(buffer.as_bytes()).await?;
                buffer.clear();
            }
        }

        if !buffer.is_empty() {
            writer.write_all(buffer.as_bytes()).await?;
        }

        writer.flush().await?;
        self.tracker
            .update_dedup_progress(total_unique, total_lines_processed);

        Ok(())
    }

    // Move process_large_file into the impl block and make it an associated function
    async fn process_large_file(
        path: &PathBuf,
        tx: mpsc::Sender<HashSet<String>>,
        chunk_size: usize,
    ) -> MergerResult<usize> {
        // ====================================================================
        // ENCODING-AWARE FILE PROCESSING (Issue #1 Fix)
        // ====================================================================
        // This function now properly handles non-UTF-8 encoded wordlists
        // by using the encoding module to detect and convert character encodings

        // Create encoding handler for this file
        let mut encoding_handler = EncodingHandler::new(true); // verbose mode
        let detected_encoding = encoding_handler.detect_or_default(path).await?;

        // Use encoding-aware reader instead of raw file reader
        let reader = crate::encoding::converter::EncodingConverter::create_converting_reader(
            path,
            detected_encoding,
        )
        .await?;
        let mut reader = reader;

        let mut buffer = Vec::with_capacity(LINE_BUFFER_CAPACITY);
        let mut current_set = HashSet::with_capacity(chunk_size);
        let mut bytes_processed = 0;
        let mut total_lines = 0;

        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer).await? {
                0 => break,
                n => {
                    bytes_processed += n;
                    if !buffer.is_empty() {
                        // The encoding converter already converted to UTF-8,
                        // so this should never fail for properly converted content
                        if let Ok(line) = String::from_utf8(buffer[..n - 1].to_vec()) {
                            let trimmed_line = line.trim();
                            if !trimmed_line.is_empty() {
                                current_set.insert(trimmed_line.to_string());
                                total_lines += 1;
                            }
                        } else {
                            // This should rarely happen with proper encoding conversion
                            // but we handle it gracefully and continue processing
                            log::warn!("Failed to parse converted line in {}", path.display());
                        }
                    }
                }
            }

            if bytes_processed >= CHUNK_SIZE || current_set.len() >= chunk_size {
                tx.send(current_set).await?;
                current_set = HashSet::with_capacity(chunk_size);
                bytes_processed = 0;
            }
        }

        if !current_set.is_empty() {
            tx.send(current_set).await?;
        }

        // Print encoding statistics
        encoding_handler.print_summary();

        Ok(total_lines)
    }

    // Function to read input files from the provided path
    async fn read_input_files(input_file: &Path) -> Result<Vec<PathBuf>> {
        let content = tokio::fs::read_to_string(input_file).await?;
        Ok(content.lines().map(PathBuf::from).collect())
    }

    // Function to process a single file
    async fn process_single_file(
        &mut self,
        file: PathBuf,
        app_state: &Arc<AppState>,
    ) -> Result<()> {
        if app_state.should_shutdown().await {
            return Err(anyhow::anyhow!("Processing interrupted by shutdown signal"));
            // Return an error if shutdown is requested
        }

        // ====================================================================
        // ENCODING-AWARE FILE VALIDATION (Issue #1 Fix)
        // ====================================================================
        // Use encoding detection to validate files instead of assuming UTF-8

        let mut encoding_handler = EncodingHandler::new(self.verbose);
        let detected_encoding = match encoding_handler.detect_or_default(&file).await {
            Ok(encoding) => encoding,
            Err(e) => {
                self.log_error(&format!(
                    "Error detecting encoding for {}: {}",
                    file.display(),
                    e
                ))
                .await?;
                return Ok(());
            }
        };

        // Try to read the file using the detected encoding
        let line_count = match Self::count_lines_with_encoding(&file, detected_encoding).await {
            Ok(count) => count,
            Err(e) => {
                self.log_error(&format!(
                    "Error reading file with detected encoding {}: {}",
                    file.display(),
                    e
                ))
                .await?;
                return Ok(());
            }
        };

        // Process the content here
        let mut progress = app_state.progress.write().await; // Acquire a write lock on the progress state
        progress.processed_files.push(file.clone()); // Add the file to the list of processed files
        progress.current_position += line_count; // Update the current position
        progress.save().await?; // Save the progress state

        if self.verbose {
            log::debug!(
                "Processed file: {} ({} lines, encoding: {})",
                file.display(),
                line_count,
                detected_encoding.name()
            ); // Log the processed file if verbose is enabled
        }

        Ok(())
    }

    // Helper function to count lines using encoding-aware reading
    async fn count_lines_with_encoding(
        path: &PathBuf,
        encoding: &'static encoding_rs::Encoding,
    ) -> Result<usize> {
        let reader =
            crate::encoding::converter::EncodingConverter::create_converting_reader(path, encoding)
                .await?;
        let mut reader = reader;
        let mut line_count = 0;
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer).await? {
                0 => break,
                _ => {
                    if !buffer.is_empty() {
                        line_count += 1;
                    }
                }
            }
        }

        Ok(line_count)
    }

    // Function to validate the input files
    async fn validate_files(&mut self, files: &[PathBuf]) -> Result<()> {
        for (i, file) in files.iter().enumerate() {
            if !file.exists() {
                self.log_error(&format!("File not found: {}", file.display()))
                    .await?;
                continue;
            }
            self.tracker.update_overall_progress(i + 1);
        }
        Ok(())
    }

    // Function to log errors to a file
    async fn log_error(&self, message: &str) -> Result<()> {
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("error.log")
            .await?;

        let error_message = format!(
            "[{}] {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), // Get the current timestamp
            message
        );

        file.write_all(error_message.as_bytes()).await?; // Write the error message to the file
        file.sync_all().await?; // Sync the file to ensure all data is written
        Ok(())
    }

    async fn validate_and_collect_metadata(
        &self,
        files: &[PathBuf],
    ) -> Result<Vec<(PathBuf, u64)>> {
        let mut valid_files = Vec::with_capacity(files.len());

        // Process files in parallel batches
        let batch_size = 50; // Validate 50 files at a time
        for chunk in files.chunks(batch_size) {
            let futures: FuturesUnordered<_> = chunk
                .iter()
                .map(|path| async move {
                    match tokio::fs::metadata(path).await {
                        Ok(meta) => Some((path.clone(), meta.len())),
                        Err(e) => {
                            eprintln!("Error accessing file {}: {}", path.display(), e);
                            None
                        }
                    }
                })
                .collect();

            // Collect results from this batch
            let batch_results: Vec<_> = futures
                .filter_map(|result| async move { result })
                .collect()
                .await;

            // Extend valid_files with batch results
            valid_files.extend(batch_results);
        }

        Ok(valid_files)
    }
}

// Enum to represent different processing stages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStage {
    Initializing,    // Initializing stage
    ValidatingFiles, // Validating files stage
    ProcessingFiles, // Processing files stage
    Merging,         // Merging stage
    Completed,       // Completed stage
    Failed,          // Failed stage
}

async fn write_chunk(lines: Vec<String>, file: &Path, offset: u64) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(file)
        .await?;
    file.seek(SeekFrom::Start(offset)).await?;
    let mut writer = BufWriter::with_capacity(OUTPUT_BUFFER_SIZE, file);

    for line in lines {
        writer.write_all(line.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }
    writer.flush().await?;
    Ok(())
}

async fn optimize_processing_order(files: Vec<(PathBuf, u64)>) -> Vec<PathBuf> {
    // Sort files by size in descending order for better memory utilization
    let mut sorted_files = files;
    sorted_files.sort_by(|a, b| b.1.cmp(&a.1));

    // Group files by size ranges to process similar-sized files together
    let mut optimized = Vec::with_capacity(sorted_files.len());
    let mut small = Vec::new();
    let mut medium = Vec::new();
    let mut large = Vec::new();

    for (path, size) in sorted_files {
        match size {
            s if s < 1024 * 1024 * 100 => small.push(path), // < 100MB
            s if s < 1024 * 1024 * 1000 => medium.push(path), // < 1GB
            _ => large.push(path),                          // >= 1GB
        }
    }

    // Process largest files first when memory is fresh
    optimized.extend(large);
    optimized.extend(medium);
    optimized.extend(small);
    optimized
}
