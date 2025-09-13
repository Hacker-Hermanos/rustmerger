use crate::app_state::AppState;
use anyhow::Result; // Importing Result type from anyhow for error handling
use log::{error, info}; // Importing logging macros for info and error messages
use std::sync::Arc; // Importing Arc for thread-safe reference counting
use tokio::sync::broadcast; // Importing broadcast channel from tokio for sending shutdown signals // Importing the AppState struct from the app_state module

// Struct to handle OS signals and manage application state
pub struct SignalHandler {
    app_state: Arc<AppState>,           // Shared and mutable application state
    shutdown_tx: broadcast::Sender<()>, // Broadcast channel sender for shutdown signals
}

impl SignalHandler {
    // Function to create a new instance of SignalHandler
    pub fn new(app_state: Arc<AppState>) -> Result<Self> {
        // Create a new broadcast channel with a buffer size of 1
        let (shutdown_tx, _) = broadcast::channel(1);

        // Return a new SignalHandler instance with the provided app_state and broadcast channel
        Ok(Self {
            app_state,
            shutdown_tx,
        })
    }

    // Function to subscribe to the shutdown broadcast channel
    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        // Return a new receiver for the broadcast channel
        self.shutdown_tx.subscribe()
    }

    // Function to set up signal handlers
    pub fn setup_handlers(&self) -> Result<()> {
        // Clone the broadcast channel sender for use in the signal handler
        let shutdown_tx = self.shutdown_tx.clone();
        // Clone the app_state for use in the signal handler
        let app_state = self.app_state.clone();

        // Set up a handler for the Ctrl+C signal
        ctrlc::set_handler(move || {
            // Log that an interrupt signal was received
            info!("Received interrupt signal, initiating graceful shutdown");

            // Clone app_state and shutdown_tx again before moving into async block
            let app_state = app_state.clone();
            let shutdown_tx = shutdown_tx.clone();

            tokio::spawn(async move {
                // Attempt to save the progress
                if let Err(e) = app_state.save_progress().await {
                    error!("Failed to save progress: {}", e);
                }

                // Attempt to send the shutdown signal
                if let Err(e) = shutdown_tx.send(()) {
                    error!("Failed to send shutdown signal: {}", e);
                }
            });
        })?;

        Ok(())
    }
}
