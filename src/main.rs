// ============================================================================
// rustmerger - High-Performance Wordlist & Rule Merger
// Main Entry Point
//
// This is the primary entry point for the rustmerger application, a Rust-based
// tool designed for merging and deduplicating wordlists and hashcat rule files.
// The tool is optimized for password cracking workflows and security testing.
//
// Key Features:
// - Parallel processing with configurable thread counts
// - Memory-efficient processing of large files (multi-GB support)
// - Graceful error handling and progress tracking
// - Resume capability for interrupted operations
// - HashSet-based deduplication for optimal performance
//
// Author: Robert Pimentel (@pr0b3r7)
// Repository: https://github.com/Hacker-Hermanos/rustmerger
// ============================================================================

use clap::Parser; // Command-line argument parsing with derive macros
use ctrlc;
use log::{error, info}; // Structured logging for debugging and monitoring
use std::sync::Arc; // Thread-safe reference counting for shared state // Cross-platform Ctrl+C signal handling

// Application modules - organized by functionality
mod app_state; // Application state management and persistence
mod cli; // Command-line interface definitions and argument parsing
mod commands; // Command handlers for different operations (merge, config, etc.)
mod config; // Configuration file management and validation
mod core; // Core processing logic for file merging and deduplication
mod encoding;
mod errors; // Custom error types and error handling utilities
mod progress; // Progress tracking and checkpoint functionality
mod signal_handler; // OS signal handling for graceful shutdown // Encoding detection and conversion for Issue #1 fix

// Import application components
use crate::app_state::AppState; // Application state and persistence
use crate::core::ProcessingCore; // Core file processing engine
use crate::errors::MergerResult; // Custom result type
use cli::{Cli, Commands}; // CLI structure and command enumeration
use commands::CommandHandler; // Command processing and orchestration

// ============================================================================
// APPLICATION ENTRY POINT
// ============================================================================

/// Main application entry point
///
/// This async function orchestrates the entire application lifecycle:
/// 1. Parses command-line arguments
/// 2. Initializes logging subsystem
/// 3. Dispatches to appropriate command handlers
/// 4. Manages graceful shutdown and error handling
///
/// The application supports four main commands:
/// - merge: Core functionality for merging and deduplicating files
/// - generate-config: Creates configuration file templates
/// - guided-setup: Interactive configuration creation
/// - resume: Resumes interrupted operations from checkpoints
#[tokio::main] // Tokio async runtime initialization
async fn main() -> MergerResult<()> {
    // ========================================================================
    // INITIALIZATION PHASE
    // ========================================================================

    // Parse command-line arguments using clap derive macros
    // This validates all input parameters and generates help text
    let cli = Cli::parse();

    // Initialize the structured logging system
    // Log level is configurable via CLI arguments (--log-level)
    // Supports: error, warn, info, debug, trace
    env_logger::builder().filter_level(cli.log_level()).init();

    info!("rustmerger starting up");

    // ========================================================================
    // COMMAND DISPATCH PHASE
    // ========================================================================

    // Route execution to appropriate command handler based on CLI input
    // Each command is handled by a specialized function in CommandHandler
    match cli.command {
        // MERGE COMMAND - Primary functionality
        // Merges and deduplicates wordlists/rules with parallel processing
        Commands::Merge(ref args) => {
            info!("Executing merge command");
            CommandHandler::handle_merge(&cli, args.clone()).await?;
        }

        // GENERATE-CONFIG COMMAND - Configuration management
        // Creates JSON configuration file templates for reusable settings
        Commands::GenerateConfig(args) => {
            info!("Executing generate-config command");
            CommandHandler::handle_generate_config(args).await?;
        }

        // GUIDED-SETUP COMMAND - Interactive configuration
        // Provides step-by-step configuration creation with prompts
        Commands::GuidedSetup(args) => {
            info!("Executing guided-setup command");
            CommandHandler::handle_guided_setup(args).await?;
        }

        // RESUME COMMAND - Operation recovery
        // Resumes interrupted operations from checkpoint files
        Commands::Resume(args) => {
            info!(
                "Executing resume command from checkpoint: {:?}",
                args.progress_file
            );

            // Reconstruct application state from checkpoint file
            // This includes processed files, current position, and configuration
            let state: AppState = AppState::from_resume(args.progress_file).await?;
            let state = Arc::new(state); // Thread-safe shared ownership

            // ================================================================
            // SIGNAL HANDLER SETUP FOR RESUME OPERATIONS
            // ================================================================

            // Set up graceful Ctrl+C handling to preserve progress
            // Critical for long-running operations that may be interrupted
            let state_clone = Arc::clone(&state);
            ctrlc::set_handler(move || {
                let state = state_clone.clone();

                // Spawn async task to handle shutdown sequence
                tokio::spawn(async move {
                    info!("Received Ctrl+C during resume, saving progress...");

                    // Attempt to save current progress before termination
                    if let Err(e) = state.save_progress().await {
                        error!("Failed to save progress during shutdown: {}", e);
                    }

                    // Signal all workers to shut down gracefully
                    state.request_shutdown().await;
                });
            })?;

            // ================================================================
            // RESUME PROCESSING EXECUTION
            // ================================================================

            // Initialize processing core with resume state
            // Enable both debug and verbose modes for resume operations
            let mut core = ProcessingCore::new(
                state.clone(),
                true, // debug mode - detailed logging
                true, // verbose mode - progress information
            )
            .await?;

            // Execute the resumed processing operation
            // This will continue from the last checkpoint
            core.process().await?;

            info!("Resume operation completed successfully");
        }
    }

    info!("rustmerger operation completed");
    Ok(())
}
