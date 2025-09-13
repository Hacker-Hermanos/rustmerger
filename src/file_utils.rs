use anyhow::Result; // Import the Result type from the anyhow crate for error handling
use log::warn;
use std::{
    fs::{File, OpenOptions}, // Import File and OpenOptions for file operations
    io::{BufRead, BufReader, BufWriter, Write}, // Import I/O traits and structs for reading and writing files
    path::Path,                                 // Import the Path struct for handling file paths
}; // Import the warn macro from the log crate for logging warnings

// Define a struct for file utility functions
pub struct FileUtils;

impl FileUtils {
    // Ensure a directory exists, creating it if necessary
    pub async fn ensure_dir(path: &Path) -> Result<()> {
        // Check if the directory does not exist
        if !path.exists() {
            // Create the directory and all its parent directories
            tokio::fs::create_dir_all(path).await?;
        }
        Ok(())
    }

    // Atomically write content to a file
    pub async fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
        // Create a temporary file path with a ".tmp" extension
        let temp_path = path.with_extension("tmp");
        // Write the content to the temporary file
        tokio::fs::write(&temp_path, content).await?;
        // Rename the temporary file to the target file path
        tokio::fs::rename(temp_path, path).await?;
        Ok(())
    }

    // Read lines from a file and return them as a vector of strings
    pub fn read_lines(path: &Path) -> Result<Vec<String>> {
        // Open the file for reading
        let file = File::open(path)?;
        // Create a buffered reader for the file
        let reader = BufReader::new(file);
        // Initialize an empty vector to store the lines
        let mut lines = Vec::new();

        // Iterate over the lines in the file
        for line in reader.lines() {
            match line {
                // If the line is read successfully, add it to the vector
                Ok(line) => lines.push(line),
                // If there is an error reading the line, log a warning
                Err(e) => warn!("Error reading line: {}", e),
            }
        }

        Ok(lines)
    }

    // Append unique lines to a file, avoiding duplicates
    pub async fn append_unique_lines(path: &Path, lines: &[String]) -> Result<()> {
        // Read existing lines from the file into a HashSet to avoid duplicates
        let mut existing = if path.exists() {
            Self::read_lines(path)?
                .into_iter()
                .collect::<std::collections::HashSet<_>>()
        } else {
            std::collections::HashSet::new()
        };

        // Open the file for appending, creating it if it doesn't exist
        let mut writer = BufWriter::new(OpenOptions::new().create(true).append(true).open(path)?);

        // Iterate over the new lines to be added
        for line in lines {
            // If the line is not already in the HashSet, add it and write it to the file
            if existing.insert(line.clone()) {
                if let Err(e) = writeln!(writer, "{}", line) {
                    warn!("Failed to write line: {}", e);
                }
            }
        }
        // Flush the writer to ensure all data is written to the file
        if let Err(e) = writer.flush() {
            warn!("Failed to flush writer: {}", e);
        }

        Ok(())
    }

    // Clean up temporary files in a directory with a specific prefix
    pub async fn cleanup_temp_files(dir: &Path, prefix: &str) -> Result<()> {
        // Read the directory entries
        let mut entries = tokio::fs::read_dir(dir).await?;
        // Iterate over the directory entries
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            // Check if the file name starts with the specified prefix
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(prefix))
                .unwrap_or(false)
            {
                // Remove the file and log a warning if there is an error
                if let Err(e) = tokio::fs::remove_file(&path).await {
                    warn!("Failed to remove temp file {:?}: {}", path, e);
                }
            }
        }
        Ok(())
    }
}
