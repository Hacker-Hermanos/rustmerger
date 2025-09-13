// ============================================================================
// Encoding Module - Public API
//
// This module provides comprehensive encoding support for rustmerger,
// addressing GitHub Issue #1: UTF-8 encoding errors with common wordlists
// https://github.com/Hacker-Hermanos/rustmerger/issues/1
//
// The issue: Popular wordlists like rockyou.txt use Windows-1252/ISO-8859-1
// encoding, causing rustmerger to skip entire files when using strict UTF-8.
//
// The solution: Auto-detect file encodings and convert to UTF-8 while
// preserving all characters for password cracking compatibility.
// ============================================================================

use anyhow::Result;
use encoding_rs::{Encoding, ISO_8859_15, ISO_8859_2, UTF_8, WINDOWS_1252};
use std::path::Path;

// Re-export submodules for public API
pub mod converter;
pub mod detector;
pub mod stats;
pub mod strategies;

// Re-export key types for convenience
pub use converter::EncodingConverter;
pub use detector::EncodingDetector;
pub use stats::EncodingStats;
pub use strategies::{EncodingStrategy, RecoveryAction};

/// Main encoding handler that orchestrates detection, conversion, and statistics
///
/// This is the primary interface for encoding operations in rustmerger.
/// It combines detection, conversion, and error recovery strategies.
pub struct EncodingHandler {
    strategy: EncodingStrategy,
    stats: EncodingStats,
    verbose: bool,
}

impl EncodingHandler {
    /// Create a new encoding handler with auto-detection strategy
    pub fn new(verbose: bool) -> Self {
        Self {
            strategy: EncodingStrategy::AutoDetect,
            stats: EncodingStats::new(),
            verbose,
        }
    }

    /// Create a new encoding handler with a specific strategy
    pub fn with_strategy(strategy: EncodingStrategy, verbose: bool) -> Self {
        Self {
            strategy,
            stats: EncodingStats::new(),
            verbose,
        }
    }

    /// Detect or determine encoding for a file
    ///
    /// This is the main entry point for encoding detection.
    /// Returns the detected encoding or falls back based on strategy.
    pub async fn detect_or_default(&mut self, path: &Path) -> Result<&'static Encoding> {
        self.stats.record_file_processed();

        let encoding = match &self.strategy {
            EncodingStrategy::AutoDetect => {
                match detector::EncodingDetector::detect_file(path).await? {
                    Some(detected) => {
                        if self.verbose {
                            println!(
                                "📝 Detected encoding: {} for {}",
                                detected.name(),
                                path.display()
                            );
                        }
                        self.stats.record_encoding_detected(detected.name());
                        detected
                    }
                    None => {
                        if self.verbose {
                            println!(
                                "⚠️  Could not detect encoding for {}, using Windows-1252 fallback",
                                path.display()
                            );
                        }
                        self.stats.record_encoding_fallback("windows-1252");
                        WINDOWS_1252
                    }
                }
            }
            EncodingStrategy::ForceEncoding(enc) => {
                if self.verbose {
                    println!(
                        "🔧 Using forced encoding: {} for {}",
                        enc.name(),
                        path.display()
                    );
                }
                self.stats.record_encoding_forced(enc.name());
                *enc
            }
            EncodingStrategy::TrySequence(encodings) => {
                // Try each encoding in sequence
                for &enc in encodings {
                    if let Ok(true) = detector::EncodingDetector::validate_encoding(path, enc).await
                    {
                        if self.verbose {
                            println!(
                                "✅ Validated encoding: {} for {}",
                                enc.name(),
                                path.display()
                            );
                        }
                        self.stats.record_encoding_detected(enc.name());
                        return Ok(enc);
                    }
                }
                // If none worked, use Windows-1252 as ultimate fallback
                if self.verbose {
                    println!(
                        "⚠️  No encodings in sequence worked for {}, using Windows-1252",
                        path.display()
                    );
                }
                self.stats.record_encoding_fallback("windows-1252");
                WINDOWS_1252
            }
        };

        Ok(encoding)
    }

    /// Get current statistics
    pub fn get_stats(&self) -> &EncodingStats {
        &self.stats
    }

    /// Print encoding summary information
    pub fn print_summary(&self) {
        if self.verbose {
            self.stats.print_summary();
        }
    }
}

/// Common encodings used in password wordlists
///
/// Based on research of popular wordlists and their typical encodings.
/// Priority order reflects likelihood in password wordlists.
pub const COMMON_WORDLIST_ENCODINGS: &[&'static Encoding] = &[
    UTF_8,        // Modern files, most common now
    WINDOWS_1252, // Legacy Windows files, very common in wordlists
    ISO_8859_15,  // European languages with Euro symbol
    ISO_8859_2,   // Central European languages
];

/// Get the default fallback encoding for password wordlists
///
/// Windows-1252 is chosen as the default because:
/// 1. Most legacy wordlists (rockyou.txt, etc.) use this encoding
/// 2. It's backward compatible with ASCII
/// 3. It handles common Western European characters
/// 4. Hashcat and John the Ripper expect this encoding for legacy lists
pub fn default_wordlist_encoding() -> &'static Encoding {
    WINDOWS_1252
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encoding_handler_creation() {
        let handler = EncodingHandler::new(true);
        assert!(matches!(handler.strategy, EncodingStrategy::AutoDetect));
        assert_eq!(handler.stats.files_processed(), 0);
    }

    #[test]
    fn test_default_wordlist_encoding() {
        let encoding = default_wordlist_encoding();
        assert_eq!(encoding.name(), "windows-1252");
    }
}
