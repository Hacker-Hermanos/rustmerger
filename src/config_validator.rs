use anyhow::{Context, Result}; // Importing Context and Result from the anyhow crate for error handling
use std::path::Path; // Importing Path from the standard library for file path handling
use crate::Config; // Importing the Config struct from the current crate

// Define a struct for configuration validation
pub struct ConfigValidator;

impl ConfigValidator {
    // Function to validate the entire configuration
    pub fn validate_config(config: &Config) -> Result<()> {
        // Validate input files path
        Self::validate_input_file(&config.input_files)
            .context("Invalid input files configuration")?;

        // Validate output files path
        if let Some(parent) = config.output_files.parent() {
            Self::validate_directory(parent)
                .context("Invalid output directory")?;
        }

        // Validate thread count
        if config.threads == 0 {
            return Err(anyhow::anyhow!("Thread count must be greater than 0"));
        }

        Ok(())
    }

    // Function to validate an input file path
    fn validate_input_file(path: &Path) -> Result<()> {
        // Check if the file exists
        if !path.exists() {
            return Err(anyhow::anyhow!("File does not exist: {:?}", path));
        }
        // Check if the path is a file
        if !path.is_file() {
            return Err(anyhow::anyhow!("Path is not a file: {:?}", path));
        }
        Ok(())
    }

    // Function to validate a directory path
    fn validate_directory(path: &Path) -> Result<()> {
        // Check if the path exists and is a directory
        if path.exists() && !path.is_dir() {
            return Err(anyhow::anyhow!("Path exists but is not a directory: {:?}", path));
        }
        Ok(())
    }
}