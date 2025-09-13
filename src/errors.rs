use ctrlc;
use dialoguer;
use std::path::PathBuf;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::task::JoinError;

/// Type alias for Result with MergerError as the error type
pub type MergerResult<T> = Result<T, MergerError>;

/// Custom error types for the file merger application
#[derive(Error, Debug)]
pub enum MergerError {
    /// Standard IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error handling via anyhow
    #[error("Internal error: {0}")]
    Anyhow(#[from] anyhow::Error),

    /// Configuration related errors
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    /// System resource errors
    #[error("System error: {0}")]
    SysInfo(#[from] sys_info::Error),

    /// File processing errors
    #[error("Processing error: {0}")]
    Processing(String),

    /// Thread communication errors
    #[error("Channel error: {0}")]
    Channel(String),

    /// Input file validation errors
    #[error("Input validation error: {0}")]
    InputValidation(String),

    /// Progress tracking errors
    #[error("Progress tracking error: {0}")]
    Progress(String),

    /// Resume operation errors
    #[error("Resume error: {source}")]
    Resume {
        #[from]
        source: ResumeError,
    },

    /// Deduplication errors
    #[error("Deduplication error: {0}")]
    Deduplication(String),

    /// UTF-8 encoding errors
    #[error("Invalid UTF-8 in file {path}: {message}")]
    InvalidUtf8 { path: PathBuf, message: String },
}

/// Specific errors related to resume functionality
#[derive(Error, Debug)]
pub enum ResumeError {
    #[error("Progress file not found: {0}")]
    ProgressFileNotFound(PathBuf),

    #[error("Invalid progress file format")]
    InvalidProgressFormat,

    #[error("Progress file is corrupted")]
    CorruptedProgress,

    #[error("Cannot resume: input files have changed")]
    InputFilesChanged,
}

/// Specific errors related to configuration
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid thread count: {0}. Must be between 1 and 100")]
    InvalidThreadCount(usize),

    #[error("Input files path must be specified")]
    MissingInputFiles,

    #[error("Output files path must be specified")]
    MissingOutputFiles,

    #[error("Input file not found: {0}")]
    InputFileNotFound(PathBuf),

    #[error("Output directory is not writable: {0}")]
    OutputDirectoryNotWritable(PathBuf),

    #[error("Input and output paths cannot be the same")]
    InputOutputPathsEqual,

    #[error("Invalid configuration format: {0}")]
    InvalidFormat(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<dialoguer::Error> for MergerError {
    fn from(err: dialoguer::Error) -> Self {
        MergerError::Processing(err.to_string())
    }
}

impl From<JoinError> for MergerError {
    fn from(err: JoinError) -> Self {
        MergerError::Processing(format!("Task join error: {}", err))
    }
}

impl<T> From<SendError<T>> for MergerError {
    fn from(err: SendError<T>) -> Self {
        MergerError::Channel(err.to_string())
    }
}

impl From<serde_json::Error> for MergerError {
    fn from(err: serde_json::Error) -> Self {
        MergerError::Config(ConfigError::InvalidFormat(err.to_string()))
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for MergerError {
    fn from(err: std::sync::mpsc::SendError<T>) -> Self {
        MergerError::Channel(err.to_string())
    }
}

impl From<ctrlc::Error> for MergerError {
    fn from(err: ctrlc::Error) -> Self {
        MergerError::Processing(format!("Ctrl+C handler error: {}", err))
    }
}
