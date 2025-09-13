// ============================================================================
// Encoding Conversion Module
//
// Provides utilities for converting between different character encodings
// while preserving all characters for password cracking compatibility.
//
// This is a critical component for fixing rustmerger Issue #1, ensuring
// that files like rockyou.txt (Windows-1252) are properly converted to UTF-8
// without losing any password characters.
// ============================================================================

use anyhow::{Context, Result};
use encoding_rs::{Encoding, UTF_8};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};

/// Buffer size for streaming conversion operations (64KB)
const CONVERSION_BUFFER_SIZE: usize = 64 * 1024;

pub struct EncodingConverter;

impl EncodingConverter {
    /// Create an async reader that automatically converts from source encoding to UTF-8
    ///
    /// This is a simplified approach that reads the entire file content,
    /// converts it, and provides an async reader for the converted content.
    /// For very large files, consider using the streaming approach instead.
    pub async fn create_converting_reader(
        path: &Path,
        source_encoding: &'static Encoding,
    ) -> Result<AsyncBufReader<std::io::Cursor<Vec<u8>>>> {
        // Read the entire file
        let file_contents = tokio::fs::read(path)
            .await
            .with_context(|| format!("Failed to read file for conversion: {}", path.display()))?;

        // Convert to UTF-8
        let (converted_string, _, had_errors) = source_encoding.decode(&file_contents);

        if had_errors {
            log::warn!(
                "Encoding conversion had errors for {}: some characters may be replaced",
                path.display()
            );
        }

        // Create a cursor from the converted bytes
        let converted_bytes = converted_string.as_bytes().to_vec();
        let cursor = std::io::Cursor::new(converted_bytes);
        let reader = AsyncBufReader::with_capacity(CONVERSION_BUFFER_SIZE, cursor);

        Ok(reader)
    }

    /// Convert a byte array from source encoding to UTF-8 string
    ///
    /// This method handles the conversion of raw bytes to UTF-8 strings,
    /// providing detailed error information and handling replacement characters.
    pub fn convert_bytes_to_utf8(
        bytes: &[u8],
        source_encoding: &'static Encoding,
    ) -> Result<(String, bool)> {
        let (decoded, _, had_errors) = source_encoding.decode(bytes);

        if had_errors {
            // Log the conversion issues but continue processing
            log::warn!(
                "Encoding conversion had errors from {} - some characters may be replaced",
                source_encoding.name()
            );
        }

        Ok((decoded.into_owned(), had_errors))
    }

    /// Convert a single line from source encoding to UTF-8
    ///
    /// This is a convenience method for line-by-line processing.
    /// Handles newline characters appropriately.
    pub fn convert_line_to_utf8(
        line_bytes: &[u8],
        source_encoding: &'static Encoding,
    ) -> Result<String> {
        // Remove trailing newline characters before conversion
        let trimmed_bytes = Self::trim_newline_bytes(line_bytes);

        let (converted, had_errors) = Self::convert_bytes_to_utf8(trimmed_bytes, source_encoding)?;

        if had_errors {
            log::debug!(
                "Line conversion had errors from {}: {}",
                source_encoding.name(),
                String::from_utf8_lossy(line_bytes)
            );
        }

        Ok(converted)
    }

    /// Remove trailing newline bytes from a byte array
    fn trim_newline_bytes(bytes: &[u8]) -> &[u8] {
        let mut end = bytes.len();

        // Remove trailing \r\n or \n
        while end > 0 {
            match bytes[end - 1] {
                b'\n' | b'\r' => end -= 1,
                _ => break,
            }
        }

        &bytes[..end]
    }

    /// Create a line-by-line iterator that converts encoding on the fly
    ///
    /// This provides an interface for processing files line by line
    /// with automatic encoding conversion.
    pub async fn create_line_iterator(
        path: &Path,
        source_encoding: &'static Encoding,
    ) -> Result<AsyncBufReader<std::io::Cursor<Vec<u8>>>> {
        Self::create_converting_reader(path, source_encoding).await
    }

    /// Test if conversion from source encoding would lose data
    ///
    /// This method can be used to validate that a conversion is safe
    /// before processing large files.
    pub fn test_conversion_safety(
        sample_bytes: &[u8],
        source_encoding: &'static Encoding,
    ) -> Result<bool> {
        let (_, _, had_errors) = source_encoding.decode(sample_bytes);

        // If we had errors, the conversion might lose data
        if had_errors {
            return Ok(false);
        }

        // For additional safety, we could attempt round-trip conversion
        // but that's more complex and may not be necessary for wordlists

        Ok(true)
    }

    /// Get detailed conversion statistics for a sample
    ///
    /// Useful for logging and debugging conversion issues.
    pub fn analyze_conversion(
        sample_bytes: &[u8],
        source_encoding: &'static Encoding,
    ) -> ConversionAnalysis {
        let (decoded, encoding_used, had_errors) = source_encoding.decode(sample_bytes);

        let original_size = sample_bytes.len();
        let converted_size = decoded.len();
        let replacement_chars = decoded.chars().filter(|&c| c == '\u{FFFD}').count();

        ConversionAnalysis {
            original_bytes: original_size,
            converted_bytes: converted_size,
            replacement_characters: replacement_chars,
            had_errors,
            encoding_used: encoding_used.name().to_string(),
            size_ratio: if original_size > 0 {
                converted_size as f64 / original_size as f64
            } else {
                1.0
            },
        }
    }
}

/// Analysis results for encoding conversion
#[derive(Debug, Clone)]
pub struct ConversionAnalysis {
    pub original_bytes: usize,
    pub converted_bytes: usize,
    pub replacement_characters: usize,
    pub had_errors: bool,
    pub encoding_used: String,
    pub size_ratio: f64,
}

impl ConversionAnalysis {
    /// Check if the conversion appears to be successful
    pub fn is_successful(&self) -> bool {
        !self.had_errors && self.replacement_characters == 0
    }

    /// Get a human-readable summary of the conversion
    pub fn summary(&self) -> String {
        if self.is_successful() {
            format!(
                "Clean conversion from {} ({} → {} bytes, {:.1}x size)",
                self.encoding_used, self.original_bytes, self.converted_bytes, self.size_ratio
            )
        } else {
            format!(
                "Lossy conversion from {} ({} → {} bytes, {} replacements)",
                self.encoding_used,
                self.original_bytes,
                self.converted_bytes,
                self.replacement_characters
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::WINDOWS_1252;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_utf8_passthrough() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "password123\ncafé")?;

        let mut reader =
            EncodingConverter::create_converting_reader(temp_file.path(), UTF_8).await?;
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        assert_eq!(line.trim(), "password123");
        Ok(())
    }

    #[test]
    fn test_byte_conversion() -> Result<()> {
        // Test Windows-1252 specific character (é = 0xE9)
        let windows1252_bytes = b"caf\xE9";
        let (converted, had_errors) =
            EncodingConverter::convert_bytes_to_utf8(windows1252_bytes, WINDOWS_1252)?;

        assert_eq!(converted, "café");
        assert!(!had_errors);
        Ok(())
    }

    #[test]
    fn test_line_conversion() -> Result<()> {
        let line_with_newline = b"password\r\n";
        let converted = EncodingConverter::convert_line_to_utf8(line_with_newline, UTF_8)?;

        assert_eq!(converted, "password");
        Ok(())
    }

    #[test]
    fn test_conversion_analysis() {
        let sample = b"test\xE9data";
        let analysis = EncodingConverter::analyze_conversion(sample, WINDOWS_1252);

        assert!(!analysis.had_errors);
        assert_eq!(analysis.replacement_characters, 0);
        assert!(analysis.is_successful());
    }

    #[test]
    fn test_newline_trimming() {
        assert_eq!(EncodingConverter::trim_newline_bytes(b"test\r\n"), b"test");
        assert_eq!(EncodingConverter::trim_newline_bytes(b"test\n"), b"test");
        assert_eq!(EncodingConverter::trim_newline_bytes(b"test\r"), b"test");
        assert_eq!(EncodingConverter::trim_newline_bytes(b"test"), b"test");
    }
}
