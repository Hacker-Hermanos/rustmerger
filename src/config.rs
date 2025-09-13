// Import required dependencies
use crate::errors::{ConfigError, MergerError, MergerResult};
use anyhow::Result; // For error handling
use dialoguer::{Confirm, Input}; // For interactive CLI prompts
use serde::{Deserialize, Serialize}; // For JSON serialization/deserialization
use std::path::PathBuf; // For file path handling
use tokio::fs; // For async file operations

// Configuration structure that can be serialized to/from JSON
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub input_files: Option<PathBuf>, // Path to file containing list of input files
    pub output_files: Option<PathBuf>, // Path where merged output will be written
    pub threads: Option<usize>,       // Number of parallel processing threads
    pub verbose: bool,                // Enable detailed logging
    pub debug: bool,                  // Enable debug mode
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_files: None,
            output_files: None,
            threads: Some(10),
            verbose: true,
            debug: true,
        }
    }
}

impl Config {
    // Load configuration from a JSON file
    pub async fn load(path: &PathBuf) -> MergerResult<Self> {
        let content = fs::read_to_string(path).await.map_err(MergerError::Io)?;
        serde_json::from_str(&content)
            .map_err(|e| MergerError::Config(ConfigError::InvalidFormat(e.to_string())))
    }

    // Save configuration to a JSON file
    pub async fn save(&self, path: &PathBuf) -> MergerResult<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| MergerError::Config(ConfigError::SerializationError(e.to_string())))?;
        fs::write(path, content).await.map_err(MergerError::Io)
    }

    // Create a default configuration template
    pub fn template() -> Self {
        Self {
            input_files: None,
            output_files: None,
            threads: Some(10),
            verbose: true,
            debug: true,
        }
    }

    // Interactive configuration setup using command-line prompts
    pub async fn guided_setup() -> MergerResult<Self> {
        // Prompt for input files path with default value
        let input_files: String = Input::new()
            .with_prompt("Enter path to input files list")
            .default("/tmp/wordlists_to_merge.txt".into())
            .interact()?;

        // Prompt for output file path with default value
        let output_files: String = Input::new()
            .with_prompt("Enter path for output file")
            .default("/tmp/merged_wordlist.txt".into())
            .interact()?;

        // Prompt for number of processing threads
        let threads: String = Input::new()
            .with_prompt("Enter number of threads")
            .default("50".into())
            .interact()?;

        // Confirm whether to enable verbose logging
        let verbose = Confirm::new()
            .with_prompt("Enable verbose logging?")
            .default(true)
            .interact()?;

        // Confirm whether to enable debug mode
        let debug = Confirm::new()
            .with_prompt("Enable debug logging?")
            .default(false)
            .interact()?;

        // Parse threads with proper error handling
        let threads = threads
            .parse::<usize>()
            .map_err(|_| MergerError::Config(ConfigError::InvalidThreadCount(0)))?;

        if threads == 0 || threads > 100 {
            return Err(MergerError::Config(ConfigError::InvalidThreadCount(
                threads,
            )));
        }

        // Create and return configuration with user-provided values
        Ok(Self {
            input_files: Some(PathBuf::from(input_files)),
            output_files: Some(PathBuf::from(output_files)),
            threads: Some(threads),
            verbose,
            debug,
        })
    }

    // Replace the existing validate method with this implementation
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate thread count
        if let Some(threads) = self.threads {
            if threads == 0 || threads > 100 {
                return Err(ConfigError::InvalidThreadCount(threads));
            }
        }

        // Validate input files path exists
        let input_path = self
            .input_files
            .as_ref()
            .ok_or(ConfigError::MissingInputFiles)?;

        if !input_path.exists() {
            return Err(ConfigError::InputFileNotFound(input_path.clone()));
        }

        // Validate output files path
        let output_path = self
            .output_files
            .as_ref()
            .ok_or(ConfigError::MissingOutputFiles)?;

        // Check if input and output paths are the same
        if input_path == output_path {
            return Err(ConfigError::InputOutputPathsEqual);
        }

        // Validate output directory exists and is writable
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                return Err(ConfigError::OutputDirectoryNotWritable(
                    parent.to_path_buf(),
                ));
            }

            // Check if directory is writable by attempting to create a temporary file
            if let Ok(temp_path) = tempfile::Builder::new()
                .prefix(".test-write-")
                .tempfile_in(parent)
            {
                // Clean up temporary file
                let _ = temp_path.close();
            } else {
                return Err(ConfigError::OutputDirectoryNotWritable(
                    parent.to_path_buf(),
                ));
            }
        }

        Ok(())
    }
}
