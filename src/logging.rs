use anyhow::Result; // Importing Result type from anyhow for error handling
use chrono::Local; // Importing Local from chrono to get the current date and time
use crossterm::style::Stylize; // Importing Stylize from crossterm to style log levels
use log::{Level, LevelFilter, Metadata, Record}; // Importing logging types from the log crate
use std::{
    fs::{File, OpenOptions}, // Importing File and OpenOptions for file operations
    io::Write,               // Importing Write trait for writing to files
    path::PathBuf,           // Importing PathBuf to handle file paths
    sync::Mutex,             // Importing Mutex for thread-safe access to files
};

// Define a struct for the Logger
pub struct Logger {
    log_file: Option<Mutex<File>>, // Optional log file wrapped in a Mutex for thread-safe access
    error_file: Option<Mutex<File>>, // Optional error file wrapped in a Mutex for thread-safe access
    level: LevelFilter,              // Log level filter to control which log messages are recorded
}

impl Logger {
    // Initialize the logger with optional log and error file paths and a log level
    pub fn init(
        log_path: Option<PathBuf>,   // Optional path for the log file
        error_path: Option<PathBuf>, // Optional path for the error file
        level: LevelFilter,          // Log level filter
    ) -> Result<()> {
        // Create the log file if a path is provided
        let log_file = log_path.map(|path| {
            Mutex::new(
                OpenOptions::new()
                    .create(true) // Create the file if it doesn't exist
                    .append(true) // Append to the file if it exists
                    .open(path) // Open the file at the given path
                    .unwrap(), // Unwrap the result, panicking if there's an error
            )
        });

        // Create the error file if a path is provided
        let error_file = error_path.map(|path| {
            Mutex::new(
                OpenOptions::new()
                    .create(true) // Create the file if it doesn't exist
                    .append(true) // Append to the file if it exists
                    .open(path) // Open the file at the given path
                    .unwrap(), // Unwrap the result, panicking if there's an error
            )
        });

        // Create a new Logger instance
        let logger = Logger {
            log_file,
            error_file,
            level,
        };

        // Set the global logger to the newly created logger
        log::set_boxed_logger(Box::new(logger))?;
        // Set the maximum log level
        log::set_max_level(level);

        Ok(())
    }

    // Format a log record into a string
    fn format_log(&self, record: &Record) -> String {
        // Style the log level based on its severity
        let level_str = match record.level() {
            Level::Error => record.level().to_string().red(), // Red for errors
            Level::Warn => record.level().to_string().yellow(), // Yellow for warnings
            Level::Info => record.level().to_string().green(), // Green for info
            Level::Debug => record.level().to_string().blue(), // Blue for debug
            Level::Trace => record.level().to_string().magenta(), // Magenta for trace
        };

        // Format the log message with the current time, log level, target, and message
        format!(
            "[{}] {} - {}: {}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S"), // Current date and time
            level_str,                                // Styled log level
            record.target(),                          // Target of the log message
            record.args()                             // Log message
        )
    }
}

// Implement the Log trait for the Logger struct
impl log::Log for Logger {
    // Check if a log message should be logged based on its metadata
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level // Only log messages at or below the set log level
    }

    // Log a message
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Check if the log message should be logged
            let formatted = self.format_log(record); // Format the log message

            // Print the log message to the console
            print!("{}", formatted);

            // Write the log message to the log file if it exists
            if let Some(log_file) = &self.log_file {
                if let Ok(mut file) = log_file.lock() {
                    // Lock the file for thread-safe access
                    let _ = file.write_all(formatted.as_bytes()); // Write the log message to the file
                }
            }

            // Write error messages to the error file if it exists
            if record.level() == Level::Error {
                if let Some(error_file) = &self.error_file {
                    if let Ok(mut file) = error_file.lock() {
                        // Lock the file for thread-safe access
                        let _ = file.write_all(formatted.as_bytes()); // Write the error message to the file
                    }
                }
            }
        }
    }

    // Flush the log files
    fn flush(&self) {
        // Flush the log file if it exists
        if let Some(log_file) = &self.log_file {
            if let Ok(mut file) = log_file.lock() {
                // Lock the file for thread-safe access
                let _ = file.flush(); // Flush the file
            }
        }
        // Flush the error file if it exists
        if let Some(error_file) = &self.error_file {
            if let Ok(mut file) = error_file.lock() {
                // Lock the file for thread-safe access
                let _ = file.flush(); // Flush the file
            }
        }
    }
}
