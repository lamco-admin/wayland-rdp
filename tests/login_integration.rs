//! Login service integration tests
//!
//! Tests the complete login service stack.

#[cfg(all(feature = "headless-compositor", feature = "pam-auth"))]
mod login_tests {
    use wrd_server::login::{LoginConfig, SecurityManager, ResourceLimits};

    #[test]
    fn test_login_config_default() {
        let config = LoginConfig::default();
        assert_eq!(config.network.port, 3389);
        assert_eq!(config.security.max_failed_attempts, 5);
    }

    #[test]
    fn test_security_manager() {
        let limits = ResourceLimits {
            max_memory_mb: 2048,
            cpu_shares: 1024,
            max_processes: 256,
            max_open_files: 1024,
        };

        let mut security = SecurityManager::new(limits, 5, std::time::Duration::from_secs(300));

        // Test failed login tracking
        assert!(!security.is_locked_out("testuser"));

        for _ in 0..5 {
            security.record_failed_login("testuser");
        }

        assert!(security.is_locked_out("testuser"));
    }

    #[test]
    fn test_resource_limits() {
        let limits = ResourceLimits {
            max_memory_mb: 1024,
            cpu_shares: 512,
            max_processes: 128,
            max_open_files: 512,
        };

        assert_eq!(limits.max_memory_mb, 1024);
        assert_eq!(limits.cpu_shares, 512);
    }
}
