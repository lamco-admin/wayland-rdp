//! Input Handling Error Types
//!
//! Comprehensive error handling for the input handling module.

use thiserror::Error;

/// Result type for input operations
pub type Result<T> = std::result::Result<T, InputError>;

/// Input module error types
#[derive(Error, Debug)]
pub enum InputError {
    /// Portal remote desktop error
    #[error("Portal remote desktop error: {0}")]
    PortalError(String),

    /// Scancode translation error
    #[error("Scancode translation failed: {0}")]
    ScancodeTranslationFailed(String),

    /// Unknown scancode
    #[error("Unknown scancode: 0x{0:04X}")]
    UnknownScancode(u16),

    /// Unknown keycode
    #[error("Unknown keycode: {0}")]
    UnknownKeycode(u32),

    /// Coordinate transformation error
    #[error("Coordinate transformation error: {0}")]
    CoordinateTransformError(String),

    /// Monitor not found
    #[error("Monitor not found: {0}")]
    MonitorNotFound(u32),

    /// Invalid coordinate
    #[error("Invalid coordinate: ({0}, {1})")]
    InvalidCoordinate(f64, f64),

    /// Invalid monitor configuration
    #[error("Invalid monitor configuration: {0}")]
    InvalidMonitorConfig(String),

    /// Layout error
    #[error("Keyboard layout error: {0}")]
    LayoutError(String),

    /// Layout not found
    #[error("Layout not found: {0}")]
    LayoutNotFound(String),

    /// XKB error
    #[error("XKB error: {0}")]
    XkbError(String),

    /// Event queue full
    #[error("Event queue is full")]
    EventQueueFull,

    /// Event send error
    #[error("Failed to send event")]
    EventSendFailed,

    /// Event receive error
    #[error("Failed to receive event")]
    EventReceiveFailed,

    /// Input latency too high
    #[error("Input latency too high: {0}ms (max: {1}ms)")]
    LatencyTooHigh(u64, u64),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Portal session error
    #[error("Portal session error: {0}")]
    PortalSessionError(String),

    /// DBus error
    #[error("DBus error: {0}")]
    DBusError(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid key event
    #[error("Invalid key event: {0}")]
    InvalidKeyEvent(String),

    /// Invalid mouse event
    #[error("Invalid mouse event: {0}")]
    InvalidMouseEvent(String),

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Error classification for recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    /// Portal-related errors
    Portal,
    /// Translation errors
    Translation,
    /// Coordinate errors
    Coordinate,
    /// Layout errors
    Layout,
    /// Event queue errors
    EventQueue,
    /// Performance errors
    Performance,
    /// State errors
    State,
    /// Unknown error type
    Unknown,
}

/// Classify error for recovery strategy selection
pub fn classify_error(error: &InputError) -> ErrorType {
    match error {
        InputError::PortalError(_)
        | InputError::PortalSessionError(_)
        | InputError::DBusError(_) => ErrorType::Portal,

        InputError::ScancodeTranslationFailed(_)
        | InputError::UnknownScancode(_)
        | InputError::UnknownKeycode(_) => ErrorType::Translation,

        InputError::CoordinateTransformError(_)
        | InputError::MonitorNotFound(_)
        | InputError::InvalidCoordinate(_, _)
        | InputError::InvalidMonitorConfig(_) => ErrorType::Coordinate,

        InputError::LayoutError(_) | InputError::LayoutNotFound(_) | InputError::XkbError(_) => {
            ErrorType::Layout
        }

        InputError::EventQueueFull
        | InputError::EventSendFailed
        | InputError::EventReceiveFailed => ErrorType::EventQueue,

        InputError::LatencyTooHigh(_, _) => ErrorType::Performance,

        InputError::InvalidState(_) => ErrorType::State,

        _ => ErrorType::Unknown,
    }
}

/// Error context for recovery decisions
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Scancode if applicable
    pub scancode: Option<u16>,

    /// Keycode if applicable
    pub keycode: Option<u32>,

    /// Mouse coordinates if applicable
    pub coordinates: Option<(f64, f64)>,

    /// Monitor ID if applicable
    pub monitor_id: Option<u32>,

    /// Keyboard layout if applicable
    pub layout: Option<String>,

    /// Retry attempt number
    pub attempt: u32,

    /// Additional context information
    pub details: String,
}

impl ErrorContext {
    /// Create new error context
    pub fn new() -> Self {
        Self {
            scancode: None,
            keycode: None,
            coordinates: None,
            monitor_id: None,
            layout: None,
            attempt: 0,
            details: String::new(),
        }
    }

    /// Set scancode
    pub fn with_scancode(mut self, scancode: u16) -> Self {
        self.scancode = Some(scancode);
        self
    }

    /// Set keycode
    pub fn with_keycode(mut self, keycode: u32) -> Self {
        self.keycode = Some(keycode);
        self
    }

    /// Set coordinates
    pub fn with_coordinates(mut self, x: f64, y: f64) -> Self {
        self.coordinates = Some((x, y));
        self
    }

    /// Set monitor ID
    pub fn with_monitor_id(mut self, id: u32) -> Self {
        self.monitor_id = Some(id);
        self
    }

    /// Set layout
    pub fn with_layout(mut self, layout: impl Into<String>) -> Self {
        self.layout = Some(layout.into());
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

    /// Use fallback scancode mapping
    UseFallbackMapping,

    /// Clamp coordinates to monitor bounds
    ClampCoordinates,

    /// Switch to default keyboard layout
    UseDefaultLayout,

    /// Skip this event
    Skip,

    /// Reset input state
    ResetState,

    /// Request new portal session
    RequestNewSession,

    /// Increase event queue size
    IncreaseQueueSize,

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
            initial_delay_ms: 10,
            backoff_multiplier: 2,
            max_delay_ms: 1000,
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
pub fn recovery_action(error: &InputError, context: &ErrorContext) -> RecoveryAction {
    match classify_error(error) {
        ErrorType::Portal => {
            if context.attempt < 2 {
                RecoveryAction::Retry(RetryConfig::default())
            } else {
                RecoveryAction::RequestNewSession
            }
        }

        ErrorType::Translation => {
            if context.attempt == 0 {
                RecoveryAction::UseFallbackMapping
            } else {
                RecoveryAction::Skip
            }
        }

        ErrorType::Coordinate => match error {
            InputError::InvalidCoordinate(_, _) => RecoveryAction::ClampCoordinates,
            InputError::MonitorNotFound(_) => RecoveryAction::ClampCoordinates,
            _ => RecoveryAction::Skip,
        },

        ErrorType::Layout => {
            if context.attempt == 0 {
                RecoveryAction::UseDefaultLayout
            } else {
                RecoveryAction::Skip
            }
        }

        ErrorType::EventQueue => match error {
            InputError::EventQueueFull => RecoveryAction::IncreaseQueueSize,
            _ => {
                if context.attempt < 2 {
                    RecoveryAction::Retry(RetryConfig {
                        max_retries: 2,
                        initial_delay_ms: 5,
                        backoff_multiplier: 2,
                        max_delay_ms: 100,
                    })
                } else {
                    RecoveryAction::Fail
                }
            }
        },

        ErrorType::Performance => RecoveryAction::Skip,

        ErrorType::State => RecoveryAction::ResetState,

        ErrorType::Unknown => RecoveryAction::Fail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        let error = InputError::PortalError("test".to_string());
        assert_eq!(classify_error(&error), ErrorType::Portal);

        let error = InputError::UnknownScancode(0x1234);
        assert_eq!(classify_error(&error), ErrorType::Translation);

        let error = InputError::InvalidCoordinate(100.0, 200.0);
        assert_eq!(classify_error(&error), ErrorType::Coordinate);

        let error = InputError::LayoutError("test".to_string());
        assert_eq!(classify_error(&error), ErrorType::Layout);

        let error = InputError::EventQueueFull;
        assert_eq!(classify_error(&error), ErrorType::EventQueue);

        let error = InputError::LatencyTooHigh(100, 20);
        assert_eq!(classify_error(&error), ErrorType::Performance);
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new()
            .with_scancode(0x1E)
            .with_keycode(30)
            .with_coordinates(100.0, 200.0)
            .with_monitor_id(1)
            .with_layout("us")
            .with_attempt(2)
            .with_details("test error");

        assert_eq!(ctx.scancode, Some(0x1E));
        assert_eq!(ctx.keycode, Some(30));
        assert_eq!(ctx.coordinates, Some((100.0, 200.0)));
        assert_eq!(ctx.monitor_id, Some(1));
        assert_eq!(ctx.layout, Some("us".to_string()));
        assert_eq!(ctx.attempt, 2);
        assert_eq!(ctx.details, "test error");
    }

    #[test]
    fn test_retry_config() {
        let config = RetryConfig::default();

        assert_eq!(config.delay_for_attempt(0).as_millis(), 10);
        assert_eq!(config.delay_for_attempt(1).as_millis(), 20);
        assert_eq!(config.delay_for_attempt(2).as_millis(), 40);

        // Should cap at max_delay_ms
        assert_eq!(config.delay_for_attempt(10).as_millis(), 1000);
    }

    #[test]
    fn test_recovery_action_portal_error() {
        let error = InputError::PortalError("test".to_string());
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
    fn test_recovery_action_translation_error() {
        let error = InputError::UnknownScancode(0x1234);
        let ctx = ErrorContext::new().with_attempt(0);

        match recovery_action(&error, &ctx) {
            RecoveryAction::UseFallbackMapping => {}
            _ => panic!("Expected UseFallbackMapping action"),
        }
    }

    #[test]
    fn test_recovery_action_coordinate_error() {
        let error = InputError::InvalidCoordinate(100.0, 200.0);
        let ctx = ErrorContext::new();

        match recovery_action(&error, &ctx) {
            RecoveryAction::ClampCoordinates => {}
            _ => panic!("Expected ClampCoordinates action"),
        }
    }

    #[test]
    fn test_recovery_action_queue_full() {
        let error = InputError::EventQueueFull;
        let ctx = ErrorContext::new();

        match recovery_action(&error, &ctx) {
            RecoveryAction::IncreaseQueueSize => {}
            _ => panic!("Expected IncreaseQueueSize action"),
        }
    }
}
