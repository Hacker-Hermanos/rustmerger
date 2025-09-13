use crate::progress::Progress;
use anyhow::Result; // Importing Result type from anyhow crate for error handling
use std::path::PathBuf; // Importing PathBuf to handle file paths
use std::sync::Arc; // Importing Arc for atomic reference counting
use tokio::sync::RwLock; // Importing RwLock from tokio for async read-write lock // Importing Progress struct from the local crate

#[allow(dead_code)]
// AppState struct holds the state of the application
pub struct AppState {
    pub input_file: PathBuf,                   // Path to the input file
    pub output_file: PathBuf,                  // Path to the output file
    pub threads: usize,                        // Number of threads to use for processing
    pub progress: Arc<RwLock<Progress>>, // Progress tracking wrapped in an async read-write lock and atomic reference counter
    pub shutdown_requested: Arc<RwLock<bool>>, // Flag to indicate if shutdown is requested, wrapped in an async read-write lock and atomic reference counter
}

impl AppState {
    // Asynchronous function to create a new AppState instance
    pub async fn new(input_file: PathBuf, output_file: PathBuf, threads: usize) -> Result<Self> {
        Ok(Self {
            input_file,                                           // Set input file path
            output_file,                                          // Set output file path
            threads,                                              // Set number of threads
            progress: Arc::new(RwLock::new(Progress::default())), // Initialize progress with default value, wrapped in Arc and RwLock
            shutdown_requested: Arc::new(RwLock::new(false)), // Initialize shutdown_requested to false, wrapped in Arc and RwLock
        })
    }

    // Asynchronous function to create an AppState instance from a resume file
    pub async fn from_resume(resume_file: PathBuf) -> Result<Self> {
        let progress = Progress::load(&resume_file).await?; // Load progress from the resume file
        Ok(Self {
            input_file: progress.input_file.clone(), // Set input file path from progress
            output_file: progress.output_file.clone(), // Set output file path from progress
            threads: progress.threads,               // Set number of threads from progress
            progress: Arc::new(RwLock::new(progress)), // Wrap loaded progress in Arc and RwLock
            shutdown_requested: Arc::new(RwLock::new(false)), // Initialize shutdown_requested to false, wrapped in Arc and RwLock
        })
    }

    // Asynchronous function to save the current progress
    pub async fn save_progress(&self) -> Result<()> {
        let progress = self.progress.read().await; // Acquire read lock on progress
        progress.save().await // Save the progress
    }

    // Asynchronous function to request shutdown
    pub async fn request_shutdown(&self) {
        *self.shutdown_requested.write().await = true; // Acquire write lock and set shutdown_requested to true
    }

    // Asynchronous function to check if shutdown is requested
    pub async fn should_shutdown(&self) -> bool {
        *self.shutdown_requested.read().await // Acquire read lock and return the value of shutdown_requested
    }
}
