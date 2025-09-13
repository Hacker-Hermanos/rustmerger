// Declare the display module, which handles displaying information to the user
pub mod display;

// Declare the core module, which contains the core processing logic of the application
pub mod core;

// Declare the app_state module, which manages the state of the application
pub mod app_state;

// Declare the progress module, which tracks and displays progress information
pub mod progress;

// Declare the config module, which handles configuration management
pub mod config;

// Declare the file_utils module, which provides utility functions for file operations
pub mod file_utils;

// Declare the logging module, which handles logging of messages and errors
pub mod logging;

// Declare the processing module, which contains the main processing logic
pub mod processing;

// Declare the signal_handler module, which handles OS signals and manages application state
pub mod signal_handler;

// Declare the errors module, which contains custom error types
pub mod errors;

// Declare the encoding module, which handles file encoding detection and conversion
// Added for Issue #1: https://github.com/Hacker-Hermanos/rustmerger/issues/1
pub mod encoding;
