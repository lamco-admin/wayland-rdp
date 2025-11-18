//! PipeWire Error Types
//!
//! Comprehensive error handling for the PipeWire integration module.

use thiserror::Error;

/// Result type for PipeWire operations
pub type Result<T> = std::result::Result<T, PipeWireError>;

/// PipeWire integration error types
#[derive(Error, Debug)]
pub enum PipeWireError {
    /// PipeWire initialization failed
    #[error("PipeWire initialization failed: {0}")]
    InitializationFailed(String),

    /// Connection to PipeWire failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Stream creation failed
    #[error("Stream creation failed: {0}")]
    StreamCreationFailed(String),

    /// Format negotiation failed
    #[error("Format negotiation failed: {0}")]
    FormatNegotiationFailed(String),

    /// Buffer allocation failed
    #[error("Buffer allocation failed: {0}")]
    BufferAllocationFailed(String),

    /// DMA-BUF import failed
    #[error("DMA-BUF import failed: {0}")]
    DmaBufImportFailed(String),

    /// Frame extraction failed
    #[error("Frame extraction failed: {0}")]
    FrameExtractionFailed(String),

    /// Stream not found
    #[error("Stream not found: {0}")]
    StreamNotFound(u32),

    /// Too many streams
    #[error("Too many streams (max: {0})")]
    TooManyStreams(usize),

    /// Stream stalled
    #[error("Stream {0} stalled")]
    StreamStalled(u32),

    /// Format conversion failed
    #[error("Format conversion failed: {0}")]
    FormatConversionFailed(String),

    /// Timeout waiting for stream
    #[error("Timeout waiting for stream")]
    Timeout,

    /// Permission denied
    #[error("Permission denied")]
    PermissionDenied,

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Buffer not available
    #[error("No buffers available")]
    NoBuffersAvailable,

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Portal error
    #[error("Portal error: {0}")]
    Portal(String),

    /// FFI error
    #[error("FFI error: {0}")]
    Ffi(String),

    /// Thread communication failed
    #[error("Thread communication failed: {0}")]
    ThreadCommunicationFailed(String),

    /// Thread panicked
    #[error("Thread panicked: {0}")]
    ThreadPanic(String),

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Error classification for recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    /// Connection-related errors
    Connection,
    /// Stream-related errors
    Stream,
    /// Buffer-related errors
    Buffer,
    /// Format-related errors
    Format,
    /// Resource-related errors
    Resource,
    /// Permission-related errors
    Permission,
    /// Timeout errors
    Timeout,
    /// Unknown error type
    Unknown,
}

/// Classify error for recovery strategy selection
pub fn classify_error(error: &PipeWireError) -> ErrorType {
    match error {
        PipeWireError::ConnectionFailed(_) | PipeWireError::InitializationFailed(_) => {
            ErrorType::Connection
        }

        PipeWireError::StreamCreationFailed(_)
        | PipeWireError::StreamNotFound(_)
        | PipeWireError::StreamStalled(_) => ErrorType::Stream,

        PipeWireError::BufferAllocationFailed(_) | PipeWireError::NoBuffersAvailable => {
            ErrorType::Buffer
        }

        PipeWireError::FormatNegotiationFailed(_) | PipeWireError::FormatConversionFailed(_) => {
            ErrorType::Format
        }

        PipeWireError::TooManyStreams(_) | PipeWireError::DmaBufImportFailed(_) => {
            ErrorType::Resource
        }

        PipeWireError::PermissionDenied | PipeWireError::Portal(_) => ErrorType::Permission,

        PipeWireError::Timeout => ErrorType::Timeout,

        _ => ErrorType::Unknown,
    }
}

/// Error context for recovery decisions
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Stream ID if applicable
    pub stream_id: Option<u32>,

    /// Portal FD if available
    pub portal_fd: Option<i32>,

    /// Retry attempt number
    pub attempt: u32,

    /// Additional context information
    pub details: String,
}

impl ErrorContext {
    /// Create new error context
    pub fn new() -> Self {
        Self {
            stream_id: None,
            portal_fd: None,
            attempt: 0,
            details: String::new(),
        }
    }

    /// Set stream ID
    pub fn with_stream_id(mut self, id: u32) -> Self {
        self.stream_id = Some(id);
        self
    }

    /// Set portal FD
    pub fn with_portal_fd(mut self, fd: i32) -> Self {
        self.portal_fd = Some(fd);
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

    /// Reconnect to PipeWire
    Reconnect(i32),

    /// Restart the stream
    RestartStream(u32),

    /// Try with fallback format
    RetryWithFallbackFormat,

    /// Reduce buffer count
    ReduceBufferCount,

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let error = PipeWireError::ConnectionFailed("test".to_string());
        assert_eq!(classify_error(&error), ErrorType::Connection);

        let error = PipeWireError::StreamStalled(1);
        assert_eq!(classify_error(&error), ErrorType::Stream);

        let error = PipeWireError::NoBuffersAvailable;
        assert_eq!(classify_error(&error), ErrorType::Buffer);
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new()
            .with_stream_id(42)
            .with_attempt(3)
            .with_details("test error");

        assert_eq!(ctx.stream_id, Some(42));
        assert_eq!(ctx.attempt, 3);
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
}
