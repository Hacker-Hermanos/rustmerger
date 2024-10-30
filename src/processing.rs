use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::fs::File;
use log::warn;
use std::path::PathBuf;
use anyhow::Result;
use crate::progress::Progress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStage {
    Initializing,
    ValidatingFiles,
    ProcessingFiles,
    Merging,
    Completed,
    Failed,
}

pub struct FileProcessor;

impl FileProcessor {
    pub async fn process_file(progress: &mut Progress, file: PathBuf) -> Result<()> {
        let file_path = file.clone();

        let file = match File::open(&file).await {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open file {:?}: {}", file, e);
                return Ok(());
            }
        };

        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            if !line.is_empty() {
                // Process line here if needed
            }
        }

        progress.add_processed_file(file_path).await?;
        Ok(())
    }
}