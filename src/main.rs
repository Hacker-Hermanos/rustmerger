use anyhow::Result; // Import the Result type from the anyhow crate for error handling
use clap::Parser; // Import the Parser trait from the clap crate for command-line argument parsing
use log::{info, error}; // Import the info macro and error macro from the log crate for logging
use std::sync::Arc; // Import the Arc type from the std::sync crate for shared ownership
use ctrlc; // Import the ctrlc crate for handling Ctrl+C signals

// Declare the modules used in the application
mod cli; // Module for command-line interface definitions
mod commands; // Module for handling different commands
mod config; // Module for configuration management
mod core; // Module for core processing logic
mod app_state; // Module for application state management
mod progress; // Module for progress tracking
mod signal_handler; // Module for signal handling
mod errors; // Add this line

// Import specific items from the cli and commands modules
use cli::{Cli, Commands}; // Import the Cli struct and Commands enum from the cli module
use commands::CommandHandler; // Import the CommandHandler struct from the commands module
use crate::core::ProcessingCore;
use crate::app_state::AppState;
use crate::errors::{MergerError, MergerResult};

// Main asynchronous function
#[tokio::main] // Macro to set up the Tokio runtime
async fn main() -> MergerResult<()> {
    // Parse command-line arguments into the Cli struct
    let cli = Cli::parse();
    
    // Initialize the logger with the log level specified in the command-line arguments
    env_logger::builder().filter_level(cli.log_level()).init();

    // Match on the command provided in the command-line arguments
    match cli.command {
        // Handle the "merge" command
        Commands::Merge(ref args) => {
            CommandHandler::handle_merge(&cli, args.clone()).await?;
        }
        // Handle the "generate-config" command
        Commands::GenerateConfig(args) => {
            CommandHandler::handle_generate_config(args).await?;
        }
        // Handle the "guided-setup" command
        Commands::GuidedSetup(args) => {
            CommandHandler::handle_guided_setup(args).await?;
        }
        // Handle the "resume" command
        Commands::Resume(args) => {
            let state: AppState = AppState::from_resume(args.progress_file).await?;
            let state = Arc::new(state);
            
            // Set up Ctrl+C handler
            let state_clone = Arc::clone(&state);
            ctrlc::set_handler(move || {
                let state = state_clone.clone();
                tokio::spawn(async move {
                    info!("Received Ctrl+C, saving progress...");
                    if let Err(e) = state.save_progress().await {
                        error!("Failed to save progress: {}", e);
                    }
                    state.request_shutdown().await;
                });
            })?;

            // Resume merger
            let mut core = ProcessingCore::new(state.clone(), true, true).await?;
            core.process().await?;
        }
    }

    Ok(())
}
