//! Clipboard Error Types
//!
//! Comprehensive error handling for the clipboard synchronization module.

use std::fmt;
use thiserror::Error;

/// Result type for clipboard operations
pub type Result<T> = std::result::Result<T, ClipboardError>;

/// Clipboard module error types
#[derive(Error, Debug)]
pub enum ClipboardError {
    /// Portal clipboard error
    #[error("Portal clipboard error: {0}")]
    PortalError(String),

    /// Format conversion error
    #[error("Format conversion failed: {0}")]
    FormatConversionFailed(String),

    /// Unsupported clipboard format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Invalid UTF-8 data
    #[error("Invalid UTF-8 data")]
    InvalidUtf8,

    /// Invalid UTF-16 data
    #[error("Invalid UTF-16 data")]
    InvalidUtf16,

    /// Invalid data structure
    #[error("Invalid data structure: {0}")]
    InvalidData(String),

    /// Image decode error
    #[error("Image decode error: {0}")]
    ImageDecodeError(String),

    /// Image encode error
    #[error("Image encode error: {0}")]
    ImageEncodeError(String),

    /// Image creation error
    #[error("Image creation error")]
    ImageCreateError,

    /// Unsupported bit depth
    #[error("Unsupported bit depth: {0}")]
    UnsupportedBitDepth(u16),

    /// Unknown format ID
    #[error("Unknown format ID: {0}")]
    UnknownFormat(u32),

    /// Data size exceeds limit
    #[error("Data size {0} exceeds maximum allowed {1}")]
    DataSizeExceeded(usize, usize),

    /// Transfer timeout
    #[error("Transfer timeout after {0}ms")]
    TransferTimeout(u64),

    /// Transfer cancelled
    #[error("Transfer cancelled")]
    TransferCancelled,

    /// Invalid state for operation
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Channel send error
    #[error("Channel send error")]
    ChannelSend,

    /// Channel receive error
    #[error("Channel receive error")]
    ChannelReceive,

    /// RDP connection error
    #[error("RDP connection error: {0}")]
    RdpConnectionError(String),

    /// Loop detected
    #[error("Clipboard loop detected")]
    LoopDetected,

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// DBus error
    #[error("DBus error: {0}")]
    DBus(String),

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Error classification for recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    /// Portal-related errors
    Portal,
    /// Format conversion errors
    FormatConversion,
    /// Data validation errors
    DataValidation,
    /// Transfer errors
    Transfer,
    /// State errors
    State,
    /// Communication errors
    Communication,
    /// Loop detection errors
    Loop,
    /// Unknown error type
    Unknown,
}

/// Classify error for recovery strategy selection
pub fn classify_error(error: &ClipboardError) -> ErrorType {
    match error {
        ClipboardError::PortalError(_) | ClipboardError::DBus(_) => ErrorType::Portal,

        ClipboardError::FormatConversionFailed(_)
        | ClipboardError::UnsupportedFormat(_)
        | ClipboardError::ImageDecodeError(_)
        | ClipboardError::ImageEncodeError(_)
        | ClipboardError::ImageCreateError
        | ClipboardError::UnsupportedBitDepth(_)
        | ClipboardError::UnknownFormat(_) => ErrorType::FormatConversion,

        ClipboardError::InvalidUtf8
        | ClipboardError::InvalidUtf16
        | ClipboardError::InvalidData(_)
        | ClipboardError::DataSizeExceeded(_, _) => ErrorType::DataValidation,

        ClipboardError::TransferTimeout(_)
        | ClipboardError::TransferCancelled
        | ClipboardError::RdpConnectionError(_) => ErrorType::Transfer,

        ClipboardError::InvalidState(_) => ErrorType::State,

        ClipboardError::ChannelSend | ClipboardError::ChannelReceive => ErrorType::Communication,

        ClipboardError::LoopDetected => ErrorType::Loop,

        _ => ErrorType::Unknown,
    }
}

/// Error context for recovery decisions
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Format ID if applicable
    pub format_id: Option<u32>,

    /// MIME type if applicable
    pub mime_type: Option<String>,

    /// Data size if applicable
    pub data_size: Option<usize>,

    /// Retry attempt number
    pub attempt: u32,

    /// Additional context information
    pub details: String,
}

impl ErrorContext {
    /// Create new error context
    pub fn new() -> Self {
        Self {
            format_id: None,
            mime_type: None,
            data_size: None,
            attempt: 0,
            details: String::new(),
        }
    }

    /// Set format ID
    pub fn with_format_id(mut self, id: u32) -> Self {
        self.format_id = Some(id);
        self
    }

    /// Set MIME type
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Set data size
    pub fn with_data_size(mut self, size: usize) -> Self {
        self.data_size = Some(size);
        self
    }

    /// Set attempt number
    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }

    /// Set details
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = details.into();
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Recovery action to take after error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Retry the operation
    Retry(RetryConfig),

    /// Try with fallback format
    RetryWithFallbackFormat,

    /// Reduce data size and retry
    ReduceDataSize(usize),

    /// Skip this transfer
    Skip,

    /// Reset clipboard state
    ResetState,

    /// Request new portal session
    RequestNewSession,

    /// Fail and propagate error
    Fail,
}

/// Retry configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,

    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,

    /// Backoff multiplier
    pub backoff_multiplier: u32,

    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            backoff_multiplier: 2,
            max_delay_ms: 5000,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for given attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        let delay = self.initial_delay_ms * (self.backoff_multiplier as u64).pow(attempt);
        let delay = delay.min(self.max_delay_ms);
        std::time::Duration::from_millis(delay)
    }
}

/// Determine recovery action for error
pub fn recovery_action(error: &ClipboardError, context: &ErrorContext) -> RecoveryAction {
    match classify_error(error) {
        ErrorType::Portal => {
            if context.attempt < 2 {
                RecoveryAction::Retry(RetryConfig::default())
            } else {
                RecoveryAction::RequestNewSession
            }
        }

        ErrorType::FormatConversion => {
            if context.attempt == 0 {
                RecoveryAction::RetryWithFallbackFormat
            } else {
                RecoveryAction::Skip
            }
        }

        ErrorType::DataValidation => match error {
            ClipboardError::DataSizeExceeded(size, max) => {
                let reduced_size = max / 2;
                if *size > reduced_size {
                    RecoveryAction::ReduceDataSize(reduced_size)
                } else {
                    RecoveryAction::Skip
                }
            }
            _ => RecoveryAction::Skip,
        },

        ErrorType::Transfer => {
            if context.attempt < 3 {
                RecoveryAction::Retry(RetryConfig::default())
            } else {
                RecoveryAction::Fail
            }
        }

        ErrorType::State => RecoveryAction::ResetState,

        ErrorType::Communication => {
            if context.attempt < 2 {
                RecoveryAction::Retry(RetryConfig {
                    max_retries: 2,
                    initial_delay_ms: 50,
                    backoff_multiplier: 2,
                    max_delay_ms: 1000,
                })
            } else {
                RecoveryAction::Fail
            }
        }

        ErrorType::Loop => RecoveryAction::Skip,

        ErrorType::Unknown => RecoveryAction::Fail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let error = ClipboardError::PortalError("test".to_string());
        assert_eq!(classify_error(&error), ErrorType::Portal);

        let error = ClipboardError::FormatConversionFailed("test".to_string());
        assert_eq!(classify_error(&error), ErrorType::FormatConversion);

        let error = ClipboardError::InvalidUtf8;
        assert_eq!(classify_error(&error), ErrorType::DataValidation);

        let error = ClipboardError::TransferTimeout(5000);
        assert_eq!(classify_error(&error), ErrorType::Transfer);

        let error = ClipboardError::LoopDetected;
        assert_eq!(classify_error(&error), ErrorType::Loop);
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new()
            .with_format_id(13)
            .with_mime_type("text/plain")
            .with_data_size(1024)
            .with_attempt(2)
            .with_details("test error");

        assert_eq!(ctx.format_id, Some(13));
        assert_eq!(ctx.mime_type, Some("text/plain".to_string()));
        assert_eq!(ctx.data_size, Some(1024));
        assert_eq!(ctx.attempt, 2);
        assert_eq!(ctx.details, "test error");
    }

    #[test]
    fn test_retry_config() {
        let config = RetryConfig::default();

        assert_eq!(config.delay_for_attempt(0).as_millis(), 100);
        assert_eq!(config.delay_for_attempt(1).as_millis(), 200);
        assert_eq!(config.delay_for_attempt(2).as_millis(), 400);
        assert_eq!(config.delay_for_attempt(3).as_millis(), 800);

        // Should cap at max_delay_ms
        assert_eq!(config.delay_for_attempt(10).as_millis(), 5000);
    }

    #[test]
    fn test_recovery_action_portal_error() {
        let error = ClipboardError::PortalError("test".to_string());
        let ctx = ErrorContext::new().with_attempt(0);

        match recovery_action(&error, &ctx) {
            RecoveryAction::Retry(_) => {}
            _ => panic!("Expected Retry action"),
        }

        let ctx = ErrorContext::new().with_attempt(3);
        match recovery_action(&error, &ctx) {
            RecoveryAction::RequestNewSession => {}
            _ => panic!("Expected RequestNewSession action"),
        }
    }

    #[test]
    fn test_recovery_action_format_error() {
        let error = ClipboardError::FormatConversionFailed("test".to_string());
        let ctx = ErrorContext::new().with_attempt(0);

        match recovery_action(&error, &ctx) {
            RecoveryAction::RetryWithFallbackFormat => {}
            _ => panic!("Expected RetryWithFallbackFormat action"),
        }
    }

    #[test]
    fn test_recovery_action_size_exceeded() {
        let error = ClipboardError::DataSizeExceeded(20_000_000, 16_777_216);
        let ctx = ErrorContext::new();

        match recovery_action(&error, &ctx) {
            RecoveryAction::ReduceDataSize(size) => {
                assert_eq!(size, 16_777_216 / 2);
            }
            _ => panic!("Expected ReduceDataSize action"),
        }
    }

    #[test]
    fn test_recovery_action_loop_detected() {
        let error = ClipboardError::LoopDetected;
        let ctx = ErrorContext::new();

        match recovery_action(&error, &ctx) {
            RecoveryAction::Skip => {}
            _ => panic!("Expected Skip action"),
        }
    }
}
