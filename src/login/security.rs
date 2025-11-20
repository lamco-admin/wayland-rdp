//! Security and resource management
//!
//! Provides security features including account lockout,
//! resource limits, and cgroups integration.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

/// Resource limits
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory (bytes)
    pub max_memory_bytes: u64,

    /// CPU shares
    pub cpu_shares: u64,

    /// Maximum number of processes
    pub max_processes: u32,

    /// Maximum open files
    pub max_open_files: u32,
}

impl ResourceLimits {
    pub fn new(max_memory_mb: u64, cpu_shares: u64, max_processes: u32, max_open_files: u32) -> Self {
        Self {
            max_memory_bytes: max_memory_mb * 1024 * 1024,
            cpu_shares,
            max_processes,
            max_open_files,
        }
    }
}

/// Failed login tracker for a single account
#[derive(Debug)]
struct FailedLoginTracker {
    attempts: Vec<SystemTime>,
    locked_until: Option<SystemTime>,
}

impl FailedLoginTracker {
    fn new() -> Self {
        Self {
            attempts: Vec::new(),
            locked_until: None,
        }
    }

    fn record_failure(&mut self) {
        self.attempts.push(SystemTime::now());

        // Keep only recent attempts (last hour)
        let cutoff = SystemTime::now() - Duration::from_secs(3600);
        self.attempts.retain(|&time| time > cutoff);
    }

    fn recent_failures(&self, window: Duration) -> usize {
        let cutoff = SystemTime::now() - window;
        self.attempts.iter().filter(|&&time| time > cutoff).count()
    }

    fn lock(&mut self, duration: Duration) {
        self.locked_until = Some(SystemTime::now() + duration);
        info!("Account locked until {:?}", self.locked_until);
    }

    fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            if SystemTime::now() < locked_until {
                return true;
            }
        }
        false
    }

    fn reset(&mut self) {
        self.attempts.clear();
        self.locked_until = None;
    }
}

/// Security manager
pub struct SecurityManager {
    /// Track failed login attempts by username
    failed_attempts: HashMap<String, FailedLoginTracker>,

    /// Resource limits
    limits: ResourceLimits,

    /// Maximum failed attempts before lockout
    max_failed_attempts: u32,

    /// Lockout duration
    lockout_duration: Duration,

    /// Enable account lockout
    enable_lockout: bool,

    /// Audit log path
    audit_log_path: Option<PathBuf>,
}

impl SecurityManager {
    /// Create new security manager
    pub fn new(
        limits: ResourceLimits,
        max_failed_attempts: u32,
        lockout_duration_secs: u32,
        enable_lockout: bool,
    ) -> Self {
        Self {
            failed_attempts: HashMap::new(),
            limits,
            max_failed_attempts,
            lockout_duration: Duration::from_secs(lockout_duration_secs as u64),
            enable_lockout,
            audit_log_path: None,
        }
    }

    /// Set audit log path
    pub fn set_audit_log(&mut self, path: PathBuf) {
        self.audit_log_path = Some(path);
    }

    /// Check if login is allowed for user
    pub fn check_login_allowed(&self, username: &str) -> Result<()> {
        if !self.enable_lockout {
            return Ok(());
        }

        if let Some(tracker) = self.failed_attempts.get(username) {
            if tracker.is_locked() {
                self.audit_log("LOCKOUT", username, "Account is locked")?;
                anyhow::bail!("Account temporarily locked due to too many failed login attempts");
            }
        }

        Ok(())
    }

    /// Record failed login attempt
    pub fn record_failed_login(&mut self, username: &str) -> Result<()> {
        if !self.enable_lockout {
            return Ok(());
        }

        let tracker = self.failed_attempts
            .entry(username.to_string())
            .or_insert_with(FailedLoginTracker::new);

        tracker.record_failure();

        let recent_failures = tracker.recent_failures(Duration::from_secs(300)); // 5 minutes

        warn!("Failed login attempt for {}: {} recent failures", username, recent_failures);

        if recent_failures >= self.max_failed_attempts as usize {
            tracker.lock(self.lockout_duration);
            self.audit_log("LOCKOUT", username, &format!("Account locked after {} failed attempts", recent_failures))?;
        }

        self.audit_log("AUTH_FAILURE", username, "Authentication failed")?;

        Ok(())
    }

    /// Record successful login
    pub fn record_successful_login(&mut self, username: &str) -> Result<()> {
        // Reset failed attempts on successful login
        if let Some(tracker) = self.failed_attempts.get_mut(username) {
            tracker.reset();
        }

        self.audit_log("AUTH_SUCCESS", username, "Authentication successful")?;

        Ok(())
    }

    /// Apply resource limits to a process
    pub fn apply_resource_limits(&self, pid: u32, uid: u32) -> Result<()> {
        info!("Applying resource limits to PID {} (UID {})", pid, uid);

        // Apply cgroup limits if available
        if self.try_apply_cgroup_limits(pid, uid).is_err() {
            warn!("Failed to apply cgroup limits - falling back to rlimits");
            self.apply_rlimits(pid)?;
        }

        Ok(())
    }

    /// Try to apply cgroup v2 resource limits
    fn try_apply_cgroup_limits(&self, pid: u32, uid: u32) -> Result<()> {
        let cgroup_path = PathBuf::from(format!("/sys/fs/cgroup/user.slice/user-{}.slice", uid));

        if !cgroup_path.exists() {
            anyhow::bail!("Cgroup path does not exist: {:?}", cgroup_path);
        }

        // Memory limit
        let memory_max = cgroup_path.join("memory.max");
        std::fs::write(&memory_max, format!("{}", self.limits.max_memory_bytes))
            .context("Failed to write memory limit")?;

        debug!("Set memory limit to {} bytes", self.limits.max_memory_bytes);

        // CPU weight
        let cpu_weight = cgroup_path.join("cpu.weight");
        std::fs::write(&cpu_weight, format!("{}", self.limits.cpu_shares))
            .context("Failed to write CPU weight")?;

        debug!("Set CPU weight to {}", self.limits.cpu_shares);

        // Add process to cgroup
        let procs = cgroup_path.join("cgroup.procs");
        std::fs::write(&procs, format!("{}", pid))
            .context("Failed to add process to cgroup")?;

        debug!("Added PID {} to cgroup", pid);

        Ok(())
    }

    /// Apply resource limits using setrlimit
    fn apply_rlimits(&self, _pid: u32) -> Result<()> {
        // This would use libc::setrlimit to apply limits
        // For now, this is a placeholder

        debug!("Applied rlimits (placeholder)");

        Ok(())
    }

    /// Write to audit log
    fn audit_log(&self, event_type: &str, username: &str, message: &str) -> Result<()> {
        if let Some(log_path) = &self.audit_log_path {
            use std::io::Write;

            let timestamp = chrono::Utc::now().to_rfc3339();
            let log_entry = format!("{} [{}] {}: {}\n", timestamp, event_type, username, message);

            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .context("Failed to open audit log")?;

            file.write_all(log_entry.as_bytes())
                .context("Failed to write to audit log")?;

            file.sync_all()
                .context("Failed to sync audit log")?;
        }

        Ok(())
    }

    /// Get resource limits
    pub fn get_limits(&self) -> &ResourceLimits {
        &self.limits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failed_login_tracking() {
        let mut tracker = FailedLoginTracker::new();

        // Record failures
        for _ in 0..3 {
            tracker.record_failure();
        }

        assert_eq!(tracker.recent_failures(Duration::from_secs(300)), 3);
        assert!(!tracker.is_locked());

        // Lock account
        tracker.lock(Duration::from_secs(60));
        assert!(tracker.is_locked());

        // Reset
        tracker.reset();
        assert!(!tracker.is_locked());
        assert_eq!(tracker.recent_failures(Duration::from_secs(300)), 0);
    }

    #[test]
    fn test_security_manager() {
        let limits = ResourceLimits::new(2048, 1024, 256, 1024);
        let mut manager = SecurityManager::new(limits, 3, 300, true);

        // Should allow login initially
        assert!(manager.check_login_allowed("testuser").is_ok());

        // Record failures
        for _ in 0..3 {
            manager.record_failed_login("testuser").unwrap();
        }

        // Should be locked now
        assert!(manager.check_login_allowed("testuser").is_err());

        // Successful login should reset
        manager.record_successful_login("testuser").unwrap();
        assert!(manager.check_login_allowed("testuser").is_ok());
    }

    #[test]
    fn test_resource_limits() {
        let limits = ResourceLimits::new(2048, 1024, 256, 1024);

        assert_eq!(limits.max_memory_bytes, 2048 * 1024 * 1024);
        assert_eq!(limits.cpu_shares, 1024);
        assert_eq!(limits.max_processes, 256);
    }
}
