// ============================================================================
// Encoding Detection Module
//
// Implements auto-detection of file encodings using chardetng library
// and validation strategies for rustmerger Issue #1 fix.
//
// Detection Strategy:
// 1. Sample first 8KB of file for performance
// 2. Use chardetng for initial detection
// 3. Validate by attempting to read some content
// 4. Fallback to common wordlist encodings if detection fails
// ============================================================================

use anyhow::{Context, Result};
use chardetng::EncodingDetector as CharDetector;
use encoding_rs::{Encoding, UTF_8, WINDOWS_1252};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};

/// Sample size for encoding detection (8KB should be sufficient)
const DETECTION_SAMPLE_SIZE: usize = 8192;

/// Maximum file size to attempt detection on (100MB limit for performance)
const MAX_DETECTION_FILE_SIZE: u64 = 100 * 1024 * 1024;

pub struct EncodingDetector;

impl EncodingDetector {
    /// Detect encoding of a file using chardetng
    ///
    /// Returns Some(encoding) if detection is confident, None if uncertain.
    /// This method prioritizes accuracy over speed by sampling file content.
    pub async fn detect_file(path: &Path) -> Result<Option<&'static Encoding>> {
        // Check file size first
        let metadata = tokio::fs::metadata(path)
            .await
            .with_context(|| format!("Failed to read metadata for {}", path.display()))?;

        if metadata.len() > MAX_DETECTION_FILE_SIZE {
            // For very large files, assume Windows-1252 (most common for wordlists)
            return Ok(Some(WINDOWS_1252));
        }

        if metadata.len() == 0 {
            // Empty files are technically UTF-8
            return Ok(Some(UTF_8));
        }

        // Read sample for detection
        let sample = Self::read_sample(path).await?;

        // Try chardetng detection first
        if let Some(encoding) = Self::detect_with_chardetng(&sample) {
            // Validate the detection by trying to decode some content
            if Self::validate_encoding_with_sample(&sample, encoding).await {
                return Ok(Some(encoding));
            }
        }

        // If chardetng failed, try heuristic detection
        Self::heuristic_detection(&sample).await
    }

    /// Read a sample of the file for encoding detection
    async fn read_sample(path: &Path) -> Result<Vec<u8>> {
        let file = File::open(path).await.with_context(|| {
            format!(
                "Failed to open file for encoding detection: {}",
                path.display()
            )
        })?;

        let mut reader = BufReader::new(file);
        let mut buffer = vec![0; DETECTION_SAMPLE_SIZE];

        let bytes_read = reader
            .read(&mut buffer)
            .await
            .with_context(|| format!("Failed to read file sample: {}", path.display()))?;

        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    /// Use chardetng library for encoding detection
    fn detect_with_chardetng(sample: &[u8]) -> Option<&'static Encoding> {
        let mut detector = CharDetector::new();
        detector.feed(sample, true);
        Some(detector.guess(None, true))
    }

    /// Validate an encoding by attempting to decode sample content
    async fn validate_encoding_with_sample(sample: &[u8], encoding: &'static Encoding) -> bool {
        let (decoded, _, had_errors) = encoding.decode(sample);

        // Consider valid if:
        // 1. No decoding errors occurred
        // 2. The decoded text contains reasonable characters
        // 3. Not too many null bytes (indicates binary data)

        if had_errors {
            return false;
        }

        let text = decoded.as_ref();
        let null_count = text.chars().filter(|&c| c == '\0').count();
        let total_chars = text.chars().count();

        // Reject if more than 5% null characters (likely binary)
        if total_chars > 0 && (null_count as f32 / total_chars as f32) > 0.05 {
            return false;
        }

        // Look for common password wordlist patterns
        let has_common_chars = text
            .chars()
            .any(|c| c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || c.is_whitespace());

        has_common_chars
    }

    /// Validate that a specific encoding can decode a file correctly
    pub async fn validate_encoding(path: &Path, encoding: &'static Encoding) -> Result<bool> {
        let sample = Self::read_sample(path).await?;
        Ok(Self::validate_encoding_with_sample(&sample, encoding).await)
    }

    /// Heuristic-based encoding detection when chardetng fails
    ///
    /// This implements fallback logic based on common patterns in wordlist files:
    /// 1. Check for UTF-8 BOM
    /// 2. Attempt UTF-8 decoding
    /// 3. Look for high-byte characters (indicates non-ASCII)
    /// 4. Apply wordlist-specific heuristics
    async fn heuristic_detection(sample: &[u8]) -> Result<Option<&'static Encoding>> {
        if sample.is_empty() {
            return Ok(Some(UTF_8));
        }

        // Check for UTF-8 BOM
        if sample.len() >= 3 && sample[0..3] == [0xEF, 0xBB, 0xBF] {
            return Ok(Some(UTF_8));
        }

        // Try UTF-8 first (most common in modern files)
        if std::str::from_utf8(sample).is_ok() {
            return Ok(Some(UTF_8));
        }

        // Check for high-byte characters (> 127)
        let has_high_bytes = sample.iter().any(|&b| b > 127);

        if has_high_bytes {
            // High bytes present, likely Windows-1252 or ISO-8859-1
            // Windows-1252 is more common in wordlists
            return Ok(Some(WINDOWS_1252));
        }

        // Pure ASCII - UTF-8 is fine
        Ok(Some(UTF_8))
    }

    /// Quick check if a file is likely binary (not suitable for text processing)
    pub async fn is_likely_binary(path: &Path) -> Result<bool> {
        let sample = Self::read_sample(path).await?;

        if sample.is_empty() {
            return Ok(false);
        }

        // Count null bytes
        let null_count = sample.iter().filter(|&&b| b == 0).count();
        let total_bytes = sample.len();

        // If more than 10% null bytes, likely binary
        Ok((null_count as f32 / total_bytes as f32) > 0.1)
    }

    /// Get encoding confidence score for logging/debugging
    pub fn get_detection_confidence(sample: &[u8], encoding: &'static Encoding) -> f32 {
        let (_, _, had_errors) = encoding.decode(sample);

        if had_errors {
            return 0.0;
        }

        // Simple confidence calculation based on successful decode
        // and presence of reasonable characters
        let (decoded, _, _) = encoding.decode(sample);
        let text = decoded.as_ref();

        let printable_count = text
            .chars()
            .filter(|c| c.is_ascii_graphic() || c.is_whitespace())
            .count();
        let total_chars = text.chars().count();

        if total_chars == 0 {
            return 1.0; // Empty is fine
        }

        printable_count as f32 / total_chars as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_detect_utf8_file() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "password123\ncafé\nnaïve")?;

        let encoding = EncodingDetector::detect_file(temp_file.path()).await?;
        assert!(encoding.is_some());
        assert_eq!(encoding.unwrap().name(), "UTF-8");

        Ok(())
    }

    #[tokio::test]
    async fn test_detect_empty_file() -> Result<()> {
        let temp_file = NamedTempFile::new()?;

        let encoding = EncodingDetector::detect_file(temp_file.path()).await?;
        assert!(encoding.is_some());
        assert_eq!(encoding.unwrap().name(), "UTF-8");

        Ok(())
    }

    #[test]
    fn test_confidence_calculation() {
        let sample = b"password123\nadmin\n";
        let confidence = EncodingDetector::get_detection_confidence(sample, UTF_8);
        assert!(confidence > 0.8); // Should be high confidence for ASCII text
    }
}
