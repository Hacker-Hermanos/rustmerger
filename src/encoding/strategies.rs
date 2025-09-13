// ============================================================================
// Encoding Strategies Module
//
// Defines encoding strategies and error recovery mechanisms for rustmerger.
// This module implements the policy layer that decides how to handle
// encoding detection failures and conversion errors.
//
// Key for Issue #1 fix: Ensures that encoding problems don't cause
// entire wordlists to be silently skipped.
// ============================================================================

use encoding_rs::{Encoding, ISO_8859_15, ISO_8859_2, UTF_8, WINDOWS_1252};
use std::fmt;

/// Strategy for determining file encodings
#[derive(Debug, Clone)]
pub enum EncodingStrategy {
    /// Automatically detect encoding using chardetng and heuristics
    AutoDetect,

    /// Force a specific encoding for all files
    ForceEncoding(&'static Encoding),

    /// Try a sequence of encodings until one works
    TrySequence(Vec<&'static Encoding>),
}

impl EncodingStrategy {
    /// Create the default strategy for password wordlists
    ///
    /// This prioritizes the most common encodings found in wordlists:
    /// 1. UTF-8 (modern files)
    /// 2. Windows-1252 (rockyou.txt and many legacy wordlists)
    /// 3. ISO-8859-15 (European with Euro symbol)
    /// 4. ISO-8859-1 (Basic Latin-1)
    pub fn default_wordlist_strategy() -> Self {
        EncodingStrategy::TrySequence(vec![UTF_8, WINDOWS_1252, ISO_8859_15, ISO_8859_2])
    }

    /// Create a strategy that forces Windows-1252 (useful for legacy wordlists)
    pub fn force_windows1252() -> Self {
        EncodingStrategy::ForceEncoding(WINDOWS_1252)
    }

    /// Create a strategy that forces UTF-8 (useful when you know files are UTF-8)
    pub fn force_utf8() -> Self {
        EncodingStrategy::ForceEncoding(UTF_8)
    }
}

impl fmt::Display for EncodingStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodingStrategy::AutoDetect => write!(f, "auto-detect"),
            EncodingStrategy::ForceEncoding(enc) => write!(f, "force {}", enc.name()),
            EncodingStrategy::TrySequence(encodings) => {
                write!(f, "try sequence: ")?;
                for (i, enc) in encodings.iter().enumerate() {
                    if i > 0 {
                        write!(f, " → ")?;
                    }
                    write!(f, "{}", enc.name())?;
                }
                Ok(())
            }
        }
    }
}

/// Action to take when encountering encoding/conversion errors
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryAction {
    /// Skip the problematic line/file and continue
    Skip,

    /// Use replacement characters (�) for invalid sequences
    Replace,

    /// Try a different encoding
    Fallback(&'static Encoding),

    /// Stop processing and return an error
    Abort,
}

impl RecoveryAction {
    /// Get a human-readable description of the action
    pub fn description(&self) -> &'static str {
        match self {
            RecoveryAction::Skip => "skip invalid content",
            RecoveryAction::Replace => "replace with � character",
            RecoveryAction::Fallback(_) => "try different encoding",
            RecoveryAction::Abort => "abort processing",
        }
    }
}

/// Context information for encoding error recovery
#[derive(Debug)]
pub struct ErrorContext {
    pub file_path: String,
    pub line_number: Option<usize>,
    pub error_message: String,
    pub attempted_encoding: String,
}

impl ErrorContext {
    pub fn new(
        file_path: String,
        line_number: Option<usize>,
        error_message: String,
        attempted_encoding: String,
    ) -> Self {
        Self {
            file_path,
            line_number,
            error_message,
            attempted_encoding,
        }
    }

    /// Create a formatted error message for logging
    pub fn format_error(&self) -> String {
        if let Some(line) = self.line_number {
            format!(
                "Encoding error in {} at line {}: {} (tried {})",
                self.file_path, line, self.error_message, self.attempted_encoding
            )
        } else {
            format!(
                "Encoding error in {}: {} (tried {})",
                self.file_path, self.error_message, self.attempted_encoding
            )
        }
    }
}

/// Policy for handling encoding errors
#[derive(Debug, Clone)]
pub struct ErrorRecoveryPolicy {
    /// What to do when encoding detection fails
    pub detection_failure_action: RecoveryAction,

    /// What to do when conversion has errors (replacement characters)
    pub conversion_error_action: RecoveryAction,

    /// What to do when a file appears to be binary
    pub binary_file_action: RecoveryAction,

    /// Maximum number of fallback attempts before giving up
    pub max_fallback_attempts: usize,

    /// Whether to be strict about encoding (fail fast) or permissive
    pub strict_mode: bool,
}

impl ErrorRecoveryPolicy {
    /// Create a permissive policy suitable for password cracking workflows
    ///
    /// This policy prioritizes data preservation over strict correctness:
    /// - Uses fallback encodings when detection fails
    /// - Replaces invalid characters rather than skipping lines
    /// - Continues processing even with some errors
    pub fn permissive_wordlist_policy() -> Self {
        Self {
            detection_failure_action: RecoveryAction::Fallback(WINDOWS_1252),
            conversion_error_action: RecoveryAction::Replace,
            binary_file_action: RecoveryAction::Skip,
            max_fallback_attempts: 3,
            strict_mode: false,
        }
    }

    /// Create a strict policy that fails fast on any encoding issues
    ///
    /// This policy is suitable when you need to ensure perfect encoding
    /// handling and want to catch problems early.
    pub fn strict_policy() -> Self {
        Self {
            detection_failure_action: RecoveryAction::Abort,
            conversion_error_action: RecoveryAction::Abort,
            binary_file_action: RecoveryAction::Abort,
            max_fallback_attempts: 0,
            strict_mode: true,
        }
    }

    /// Create the default policy for rustmerger (balanced approach)
    ///
    /// This balances data preservation with error reporting:
    /// - Tries fallbacks for detection failures
    /// - Uses replacement characters for conversion errors
    /// - Logs all issues but continues processing
    pub fn default_policy() -> Self {
        Self {
            detection_failure_action: RecoveryAction::Fallback(WINDOWS_1252),
            conversion_error_action: RecoveryAction::Replace,
            binary_file_action: RecoveryAction::Skip,
            max_fallback_attempts: 2,
            strict_mode: false,
        }
    }

    /// Determine what action to take for a specific error context
    pub fn determine_action(&self, context: &ErrorContext, attempt_count: usize) -> RecoveryAction {
        // Check if we've exceeded fallback attempts
        if attempt_count >= self.max_fallback_attempts {
            if self.strict_mode {
                return RecoveryAction::Abort;
            } else {
                return RecoveryAction::Replace;
            }
        }

        // Determine action based on error type and policy
        if context.error_message.contains("detection") {
            self.detection_failure_action.clone()
        } else if context.error_message.contains("conversion") {
            self.conversion_error_action.clone()
        } else if context.error_message.contains("binary") {
            self.binary_file_action.clone()
        } else {
            // Default fallback
            if self.strict_mode {
                RecoveryAction::Abort
            } else {
                RecoveryAction::Replace
            }
        }
    }

    /// Get the next fallback encoding to try
    pub fn get_fallback_encoding(
        &self,
        failed_encoding: &'static Encoding,
    ) -> Option<&'static Encoding> {
        // Define fallback sequence based on failed encoding
        if failed_encoding == UTF_8 {
            Some(WINDOWS_1252)
        } else if failed_encoding == WINDOWS_1252 {
            Some(ISO_8859_15)
        } else if failed_encoding == ISO_8859_15 {
            Some(ISO_8859_2)
        } else if failed_encoding == ISO_8859_2 {
            None // No more fallbacks
        } else {
            Some(WINDOWS_1252) // Default fallback for unknown encodings
        }
    }

    /// Check if we should log this error (based on verbosity and error frequency)
    pub fn should_log_error(&self, _context: &ErrorContext) -> bool {
        // In strict mode, always log errors
        if self.strict_mode {
            return true;
        }

        // For permissive mode, log the first few errors but not floods
        // This would need additional state tracking in practice
        true
    }
}

impl Default for ErrorRecoveryPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_strategy_display() {
        let auto = EncodingStrategy::AutoDetect;
        assert_eq!(auto.to_string(), "auto-detect");

        let force = EncodingStrategy::ForceEncoding(UTF_8);
        assert_eq!(force.to_string(), "force UTF-8");

        let sequence = EncodingStrategy::TrySequence(vec![UTF_8, WINDOWS_1252]);
        assert!(sequence.to_string().contains("UTF-8"));
        assert!(sequence.to_string().contains("windows-1252"));
    }

    #[test]
    fn test_recovery_actions() {
        let policy = ErrorRecoveryPolicy::permissive_wordlist_policy();
        assert!(!policy.strict_mode);
        assert_eq!(policy.max_fallback_attempts, 3);

        let strict_policy = ErrorRecoveryPolicy::strict_policy();
        assert!(strict_policy.strict_mode);
        assert_eq!(strict_policy.max_fallback_attempts, 0);
    }

    #[test]
    fn test_fallback_encoding_sequence() {
        let policy = ErrorRecoveryPolicy::default_policy();

        assert_eq!(policy.get_fallback_encoding(UTF_8), Some(WINDOWS_1252));
        assert_eq!(
            policy.get_fallback_encoding(WINDOWS_1252),
            Some(ISO_8859_15)
        );
        assert_eq!(policy.get_fallback_encoding(ISO_8859_2), None);
    }

    #[test]
    fn test_error_context_formatting() {
        let context = ErrorContext::new(
            "test.txt".to_string(),
            Some(42),
            "invalid sequence".to_string(),
            "utf-8".to_string(),
        );

        let formatted = context.format_error();
        assert!(formatted.contains("test.txt"));
        assert!(formatted.contains("42"));
        assert!(formatted.contains("invalid sequence"));
        assert!(formatted.contains("utf-8"));
    }
}
