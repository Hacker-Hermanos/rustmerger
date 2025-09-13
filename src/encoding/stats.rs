// ============================================================================
// Encoding Statistics Module
//
// Tracks and reports encoding detection and conversion statistics for
// rustmerger Issue #1 fix. Provides detailed information about encoding
// operations to help users understand what happened during processing.
//
// This module helps with transparency and debugging when processing
// wordlists with various encodings.
// ============================================================================

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Statistics collector for encoding operations
#[derive(Debug, Clone)]
pub struct EncodingStats {
    files_processed: usize,
    encodings_detected: HashMap<String, usize>,
    encodings_forced: HashMap<String, usize>,
    encoding_fallbacks: HashMap<String, usize>,
    conversion_errors: usize,
    bytes_processed: u64,
    processing_time: Duration,
    start_time: Option<Instant>,
}

impl EncodingStats {
    /// Create a new statistics collector
    pub fn new() -> Self {
        Self {
            files_processed: 0,
            encodings_detected: HashMap::new(),
            encodings_forced: HashMap::new(),
            encoding_fallbacks: HashMap::new(),
            conversion_errors: 0,
            bytes_processed: 0,
            processing_time: Duration::default(),
            start_time: None,
        }
    }

    /// Start timing for statistics
    pub fn start_timing(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Stop timing and record duration
    pub fn stop_timing(&mut self) {
        if let Some(start) = self.start_time.take() {
            self.processing_time = start.elapsed();
        }
    }

    /// Record that a file was processed
    pub fn record_file_processed(&mut self) {
        self.files_processed += 1;
    }

    /// Record successful encoding detection
    pub fn record_encoding_detected(&mut self, encoding_name: &str) {
        *self
            .encodings_detected
            .entry(encoding_name.to_string())
            .or_insert(0) += 1;
    }

    /// Record that an encoding was forced by user
    pub fn record_encoding_forced(&mut self, encoding_name: &str) {
        *self
            .encodings_forced
            .entry(encoding_name.to_string())
            .or_insert(0) += 1;
    }

    /// Record that we fell back to a default encoding
    pub fn record_encoding_fallback(&mut self, encoding_name: &str) {
        *self
            .encoding_fallbacks
            .entry(encoding_name.to_string())
            .or_insert(0) += 1;
    }

    /// Record a conversion error
    pub fn record_conversion_error(&mut self) {
        self.conversion_errors += 1;
    }

    /// Record bytes processed during conversion
    pub fn record_bytes_processed(&mut self, bytes: u64) {
        self.bytes_processed += bytes;
    }

    /// Get the number of files processed
    pub fn files_processed(&self) -> usize {
        self.files_processed
    }

    /// Get the total bytes processed
    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed
    }

    /// Get the number of conversion errors
    pub fn conversion_errors(&self) -> usize {
        self.conversion_errors
    }

    /// Get processing duration
    pub fn processing_time(&self) -> Duration {
        self.processing_time
    }

    /// Print a comprehensive summary of encoding statistics
    pub fn print_summary(&self) {
        println!("\n📊 Encoding Processing Summary:");
        println!("├─ Files processed: {}", self.files_processed);

        // Show detected encodings
        if !self.encodings_detected.is_empty() {
            println!("├─ Encodings auto-detected:");
            for (encoding, count) in &self.encodings_detected {
                println!("│  ├─ {}: {} files", encoding, count);
            }
        }

        // Show forced encodings
        if !self.encodings_forced.is_empty() {
            println!("├─ Encodings forced by user:");
            for (encoding, count) in &self.encodings_forced {
                println!("│  ├─ {}: {} files", encoding, count);
            }
        }

        // Show fallback encodings
        if !self.encoding_fallbacks.is_empty() {
            println!("├─ Fallback encodings used:");
            for (encoding, count) in &self.encoding_fallbacks {
                println!("│  ├─ {}: {} files", encoding, count);
            }
        }

        // Show error information
        if self.conversion_errors > 0 {
            println!(
                "├─ Conversion errors: {} (characters replaced with �)",
                self.conversion_errors
            );
        } else {
            println!("├─ Conversion errors: None ✓");
        }

        // Show processing statistics
        println!("├─ Data processed: {}", format_bytes(self.bytes_processed));

        if self.processing_time.as_secs() > 0 || self.processing_time.as_millis() > 0 {
            println!(
                "├─ Processing time: {:.2}s",
                self.processing_time.as_secs_f64()
            );

            if self.bytes_processed > 0 {
                let throughput = self.bytes_processed as f64 / self.processing_time.as_secs_f64();
                println!("└─ Throughput: {}/s", format_bytes(throughput as u64));
            } else {
                println!("└─ Throughput: N/A");
            }
        } else {
            println!("└─ Processing time: < 1ms");
        }
    }

    /// Print a compact one-line summary
    pub fn print_compact_summary(&self) {
        let primary_encoding = self.get_most_common_encoding();
        println!(
            "📊 Processed {} files ({}, {} errors, {})",
            self.files_processed,
            primary_encoding,
            self.conversion_errors,
            format_bytes(self.bytes_processed)
        );
    }

    /// Get the most commonly detected/used encoding
    pub fn get_most_common_encoding(&self) -> String {
        // Combine all encoding counts
        let mut combined = HashMap::new();

        for (encoding, count) in &self.encodings_detected {
            *combined.entry(encoding.clone()).or_insert(0) += count;
        }

        for (encoding, count) in &self.encodings_forced {
            *combined.entry(encoding.clone()).or_insert(0) += count;
        }

        for (encoding, count) in &self.encoding_fallbacks {
            *combined.entry(encoding.clone()).or_insert(0) += count;
        }

        combined
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(encoding, count)| format!("{} ({})", encoding, count))
            .unwrap_or_else(|| "none".to_string())
    }

    /// Get success rate (percentage of files processed without errors)
    pub fn success_rate(&self) -> f64 {
        if self.files_processed == 0 {
            return 100.0;
        }

        let successful_files = self.files_processed.saturating_sub(self.conversion_errors);
        (successful_files as f64 / self.files_processed as f64) * 100.0
    }

    /// Check if encoding processing was fully successful
    pub fn is_fully_successful(&self) -> bool {
        self.conversion_errors == 0 && self.files_processed > 0
    }

    /// Get a summary for logging
    pub fn log_summary(&self) -> String {
        format!(
            "Encoding stats: {} files, {} encoding(s), {} errors, {:.1}% success rate",
            self.files_processed,
            self.unique_encodings_count(),
            self.conversion_errors,
            self.success_rate()
        )
    }

    /// Count unique encodings encountered
    fn unique_encodings_count(&self) -> usize {
        let mut unique = std::collections::HashSet::new();

        for encoding in self.encodings_detected.keys() {
            unique.insert(encoding);
        }
        for encoding in self.encodings_forced.keys() {
            unique.insert(encoding);
        }
        for encoding in self.encoding_fallbacks.keys() {
            unique.insert(encoding);
        }

        unique.len()
    }

    /// Merge statistics from another collector
    pub fn merge(&mut self, other: &EncodingStats) {
        self.files_processed += other.files_processed;
        self.conversion_errors += other.conversion_errors;
        self.bytes_processed += other.bytes_processed;
        self.processing_time += other.processing_time;

        for (encoding, count) in &other.encodings_detected {
            *self.encodings_detected.entry(encoding.clone()).or_insert(0) += count;
        }

        for (encoding, count) in &other.encodings_forced {
            *self.encodings_forced.entry(encoding.clone()).or_insert(0) += count;
        }

        for (encoding, count) in &other.encoding_fallbacks {
            *self.encoding_fallbacks.entry(encoding.clone()).or_insert(0) += count;
        }
    }
}

impl Default for EncodingStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Format bytes in human-readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes = bytes as f64;
    let i = (bytes.log10() / 1000_f64.log10()).floor() as usize;
    let i = i.min(UNITS.len() - 1);

    let size = bytes / (1000_f64.powi(i as i32));

    if i == 0 {
        format!("{} {}", bytes as u64, UNITS[i])
    } else {
        format!("{:.1} {}", size, UNITS[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_collection() {
        let mut stats = EncodingStats::new();

        stats.record_file_processed();
        stats.record_encoding_detected("utf-8");
        stats.record_bytes_processed(1024);

        assert_eq!(stats.files_processed(), 1);
        assert_eq!(stats.bytes_processed(), 1024);
        assert_eq!(stats.conversion_errors(), 0);
        assert!(stats.is_fully_successful());
    }

    #[test]
    fn test_success_rate() {
        let mut stats = EncodingStats::new();

        stats.record_file_processed();
        stats.record_file_processed();
        stats.record_conversion_error();

        assert_eq!(stats.success_rate(), 50.0);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.1 GB");
    }

    #[test]
    fn test_most_common_encoding() {
        let mut stats = EncodingStats::new();

        stats.record_encoding_detected("utf-8");
        stats.record_encoding_detected("utf-8");
        stats.record_encoding_detected("windows-1252");

        let common = stats.get_most_common_encoding();
        assert!(common.contains("utf-8"));
        assert!(common.contains("2"));
    }
}
