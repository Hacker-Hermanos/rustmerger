// Import required dependencies
use anyhow::Result; // For error handling
use log::{info, warn}; // For logging
use std::path::PathBuf; // For file path operations
use std::sync::Arc; // For thread-safe reference counting

// Import local modules
use crate::{
    app_state::AppState, // Application state management
    cli::{Cli, GenerateConfigArgs, GuidedSetupArgs, MergeArgs, ResumeArgs}, // CLI arguments
    config::Config,      // Configuration handling
    core::ProcessingCore, // Core processing logic
    signal_handler::SignalHandler, // Add this with other imports
};

// Command handler for processing CLI commands
pub struct CommandHandler;

impl CommandHandler {
    // Handle the merge command - combines wordlists and rules
    pub async fn handle_merge(cli: &Cli, args: MergeArgs) -> Result<()> {
        info!("Starting merge operation");

        // Load existing config or create default template
        let config = if let Some(config_path) = args.config {
            Config::load(&config_path).await?
        } else {
            Config::default()
        };

        // Determine input and output files (handle both wordlists and rules)
        let input_file = args
            .wordlists_file
            .or(args.rules_file)
            .or(config.input_files)
            .ok_or_else(|| {
                anyhow::anyhow!("No input file specified (use --wordlists-file or --rules-file)")
            })?;

        let output_file = args
            .output_wordlist
            .or(args.output_rules)
            .or(config.output_files)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No output file specified (use --output-wordlist or --output-rules)"
                )
            })?;

        // Create thread-safe application state
        let app_state = Arc::new(
            AppState::new(
                input_file,
                output_file,
                if let Some(threads) = config.threads {
                    threads
                } else {
                    10 // Default to 10 threads if not specified
                },
            )
            .await?,
        );

        // Fix debug and verbose settings
        let debug_enabled = args.debug || config.debug; // Enable debug if specified in args or config
        let verbose_enabled = cli.verbose_count() > 0 || config.verbose; // Enable verbose if specified in CLI or config

        // Set up signal handler
        let signal_handler = SignalHandler::new(app_state.clone())?;
        signal_handler.setup_handlers()?;

        // Create processing core and start processing
        let mut core =
            ProcessingCore::new(app_state.clone(), debug_enabled, verbose_enabled).await?;

        if let Err(e) = core.process().await {
            warn!("Error during processing: {}", e);
        }

        info!("Merge operation completed");
        Ok(())
    }

    // Handle configuration file generation
    pub async fn handle_generate_config(args: GenerateConfigArgs) -> Result<()> {
        info!("Generating configuration file");

        // Create default template config
        let config = if args.template {
            Config::template()
        } else {
            Config::template()
        };

        // Save configuration to specified path
        config.save(&args.output).await?;

        info!("Configuration file generated at: {:?}", args.output);
        Ok(())
    }

    // Handle interactive setup process
    pub async fn handle_guided_setup(args: GuidedSetupArgs) -> Result<()> {
        info!("Starting guided setup");

        // Run interactive configuration
        let config = Config::guided_setup().await?;
        config.save(&args.output).await?;

        info!("Configuration saved to: {:?}", args.output);
        Ok(())
    }

    // Handle resuming from a previous state
    #[allow(dead_code)]
    pub async fn handle_resume(args: ResumeArgs) -> Result<()> {
        info!("Resuming from progress file: {:?}", args.progress_file);

        // Create application state with default values
        let app_state = Arc::new(
            AppState::new(
                args.progress_file.clone(),
                PathBuf::from("/tmp/output.txt"), // Default output path
                10,                               // Default threads
            )
            .await?,
        );

        // Initialize processing core with minimal logging
        let mut core = ProcessingCore::new(
            app_state.clone(),
            false, // Debug disabled
            false, // Verbose disabled
        )
        .await?;

        // Resume processing and handle errors
        if let Err(e) = core.process().await {
            warn!("Error during processing: {}", e);
        }

        info!("Resume operation completed");
        Ok(())
    }
}
