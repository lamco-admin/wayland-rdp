//! Resource Management with cgroups v2
//!
//! Provides comprehensive resource isolation and limits for multi-user sessions:
//! - Memory limits and OOM protection
//! - CPU quotas and shares
//! - I/O bandwidth limits
//! - Process count limits
//! - Network bandwidth limits (via tc)
//!
//! Uses cgroups v2 (unified hierarchy) for modern Linux systems.

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::headless::config::{HeadlessConfig, ResourceConfig, SessionLimits};

/// Resource manager for session isolation
pub struct ResourceManager {
    config: Arc<HeadlessConfig>,
    cgroup_root: PathBuf,
    sessions: Arc<RwLock<std::collections::HashMap<String, SessionResources>>>,
}

/// Resource tracking for a session
#[derive(Debug, Clone)]
struct SessionResources {
    session_id: String,
    uid: u32,
    cgroup_path: PathBuf,
    limits: SessionLimits,
    created_at: std::time::SystemTime,
}

/// Resource limits enforcement
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Memory limit in bytes
    pub memory_max: u64,

    /// Memory swap limit in bytes
    pub swap_max: u64,

    /// CPU quota in microseconds per 100ms
    pub cpu_quota: u64,

    /// CPU weight/shares (1-10000)
    pub cpu_weight: u32,

    /// Maximum number of processes
    pub pids_max: u32,

    /// I/O weight (1-10000)
    pub io_weight: u32,
}

impl ResourceManager {
    /// Create new resource manager
    pub async fn new(config: Arc<HeadlessConfig>) -> Result<Self> {
        info!("Initializing resource manager");

        if !config.resources.enable_cgroups {
            warn!("cgroups disabled - resource limits will not be enforced!");
            return Ok(Self {
                config,
                cgroup_root: PathBuf::from("/sys/fs/cgroup"),
                sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            });
        }

        // Detect cgroup version
        Self::check_cgroup_version(&config.resources)?;

        // Determine cgroup root path
        let cgroup_root = PathBuf::from("/sys/fs/cgroup");
        let service_cgroup = cgroup_root.join(&config.resources.cgroup_base);

        // Create base cgroup if it doesn't exist
        if !service_cgroup.exists() {
            Self::create_cgroup(&service_cgroup).await?;
            info!("Created base cgroup: {:?}", service_cgroup);
        }

        // Enable controllers
        Self::enable_controllers(&service_cgroup, &["memory", "cpu", "pids", "io"]).await?;

        info!("Resource manager initialized successfully");

        Ok(Self {
            config,
            cgroup_root: service_cgroup,
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Apply resource limits to a session
    pub async fn apply_session_limits(&self, session_id: &str, uid: u32) -> Result<()> {
        if !self.config.resources.enable_cgroups {
            debug!("cgroups disabled, skipping resource limits");
            return Ok(());
        }

        info!("Applying resource limits for session: {}", session_id);

        let limits = &self.config.resources.session_limits;

        // Create session cgroup
        let session_cgroup = self.cgroup_root.join(session_id);
        Self::create_cgroup(&session_cgroup).await?;

        // Apply memory limits
        if limits.max_memory > 0 {
            let memory_max = (limits.max_memory * 1024 * 1024) as u64; // MB to bytes
            Self::write_cgroup_file(&session_cgroup, "memory.max", &memory_max.to_string())
                .await?;

            debug!("Set memory limit: {} MB", limits.max_memory);
        }

        // Apply swap limits
        if limits.max_swap > 0 {
            let swap_max = (limits.max_swap * 1024 * 1024) as u64;
            Self::write_cgroup_file(&session_cgroup, "memory.swap.max", &swap_max.to_string())
                .await?;
        } else {
            // Disable swap
            Self::write_cgroup_file(&session_cgroup, "memory.swap.max", "0").await?;
        }

        // Apply CPU limits
        if limits.cpu_quota > 0 {
            // CPU quota: percentage to microseconds per 100ms
            // 100% = 100000 microseconds per 100ms period
            let quota_us = (limits.cpu_quota * 1000) as u64;
            let period_us = 100000u64; // 100ms

            Self::write_cgroup_file(
                &session_cgroup,
                "cpu.max",
                &format!("{} {}", quota_us, period_us),
            )
            .await?;

            debug!("Set CPU quota: {}%", limits.cpu_quota);
        }

        // Apply CPU weight/shares
        if limits.cpu_shares > 0 {
            Self::write_cgroup_file(&session_cgroup, "cpu.weight", &limits.cpu_shares.to_string())
                .await?;

            debug!("Set CPU weight: {}", limits.cpu_shares);
        }

        // Apply process limits
        if limits.max_processes > 0 {
            Self::write_cgroup_file(&session_cgroup, "pids.max", &limits.max_processes.to_string())
                .await?;

            debug!("Set process limit: {}", limits.max_processes);
        }

        // Apply I/O limits
        Self::write_cgroup_file(&session_cgroup, "io.weight", "100").await?;

        // Add session to tracking
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(
                session_id.to_string(),
                SessionResources {
                    session_id: session_id.to_string(),
                    uid,
                    cgroup_path: session_cgroup,
                    limits: limits.clone(),
                    created_at: std::time::SystemTime::now(),
                },
            );
        }

        info!("Resource limits applied for session: {}", session_id);
        Ok(())
    }

    /// Move process to session cgroup
    pub async fn add_process_to_session(&self, session_id: &str, pid: u32) -> Result<()> {
        if !self.config.resources.enable_cgroups {
            return Ok(());
        }

        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        Self::write_cgroup_file(&session.cgroup_path, "cgroup.procs", &pid.to_string()).await?;

        debug!("Added process {} to session {}", pid, session_id);
        Ok(())
    }

    /// Release resources for a session
    pub async fn release_session_resources(&self, session_id: &str) -> Result<()> {
        if !self.config.resources.enable_cgroups {
            return Ok(());
        }

        info!("Releasing resources for session: {}", session_id);

        let session_cgroup = {
            let mut sessions = self.sessions.write().await;
            sessions
                .remove(session_id)
                .map(|s| s.cgroup_path)
        };

        if let Some(cgroup_path) = session_cgroup {
            // Kill all processes in the cgroup
            Self::kill_cgroup_processes(&cgroup_path).await?;

            // Remove cgroup
            if cgroup_path.exists() {
                tokio::fs::remove_dir(&cgroup_path)
                    .await
                    .context("Failed to remove cgroup")?;
            }

            debug!("Removed cgroup: {:?}", cgroup_path);
        }

        info!("Resources released for session: {}", session_id);
        Ok(())
    }

    /// Get resource usage statistics for a session
    pub async fn get_session_stats(&self, session_id: &str) -> Result<SessionStats> {
        if !self.config.resources.enable_cgroups {
            return Ok(SessionStats::default());
        }

        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        let memory_current = Self::read_cgroup_file(&session.cgroup_path, "memory.current")
            .await?
            .trim()
            .parse::<u64>()
            .unwrap_or(0);

        let cpu_stat = Self::read_cgroup_file(&session.cgroup_path, "cpu.stat").await?;
        let cpu_usage_us = Self::parse_cpu_stat(&cpu_stat);

        let pids_current = Self::read_cgroup_file(&session.cgroup_path, "pids.current")
            .await?
            .trim()
            .parse::<u32>()
            .unwrap_or(0);

        Ok(SessionStats {
            memory_bytes: memory_current,
            cpu_usage_us,
            process_count: pids_current,
        })
    }

    /// Get total memory usage across all sessions
    pub async fn total_memory_usage(&self) -> u64 {
        let sessions = self.sessions.read().await;

        let mut total = 0u64;
        for (session_id, _) in sessions.iter() {
            if let Ok(stats) = self.get_session_stats(session_id).await {
                total += stats.memory_bytes;
            }
        }

        total
    }

    /// Get total CPU usage across all sessions
    pub async fn total_cpu_usage(&self) -> f32 {
        // TODO: Implement CPU usage calculation
        0.0
    }

    /// Cleanup all session resources
    pub async fn cleanup(&self) -> Result<()> {
        info!("Cleaning up all session resources");

        let session_ids: Vec<String> = {
            let sessions = self.sessions.read().await;
            sessions.keys().cloned().collect()
        };

        for session_id in session_ids {
            if let Err(e) = self.release_session_resources(&session_id).await {
                warn!("Failed to release resources for session {}: {}", session_id, e);
            }
        }

        info!("Resource cleanup complete");
        Ok(())
    }

    // Helper functions

    fn check_cgroup_version(config: &ResourceConfig) -> Result<()> {
        let cgroup2_mount = Path::new("/sys/fs/cgroup/cgroup.controllers");

        if !cgroup2_mount.exists() {
            if config.cgroup_version == 2 {
                anyhow::bail!("cgroups v2 not available on this system");
            }
            warn!("cgroups v2 not detected, falling back to v1 (limited functionality)");
        }

        Ok(())
    }

    async fn create_cgroup(path: &Path) -> Result<()> {
        if !path.exists() {
            tokio::fs::create_dir_all(path)
                .await
                .context("Failed to create cgroup directory")?;
        }
        Ok(())
    }

    async fn enable_controllers(cgroup_path: &Path, controllers: &[&str]) -> Result<()> {
        let subtree_control = cgroup_path.join("cgroup.subtree_control");

        if !subtree_control.exists() {
            debug!("cgroup.subtree_control not found, skipping controller enable");
            return Ok(());
        }

        for controller in controllers {
            let enable_cmd = format!("+{}", controller);
            if let Err(e) = tokio::fs::write(&subtree_control, enable_cmd.as_bytes()).await {
                warn!("Failed to enable controller {}: {}", controller, e);
            }
        }

        Ok(())
    }

    async fn write_cgroup_file(cgroup_path: &Path, filename: &str, value: &str) -> Result<()> {
        let file_path = cgroup_path.join(filename);

        tokio::fs::write(&file_path, value)
            .await
            .with_context(|| format!("Failed to write {} = {}", filename, value))?;

        Ok(())
    }

    async fn read_cgroup_file(cgroup_path: &Path, filename: &str) -> Result<String> {
        let file_path = cgroup_path.join(filename);

        tokio::fs::read_to_string(&file_path)
            .await
            .with_context(|| format!("Failed to read {}", filename))
    }

    fn parse_cpu_stat(stat: &str) -> u64 {
        for line in stat.lines() {
            if line.starts_with("usage_usec") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    return value.parse().unwrap_or(0);
                }
            }
        }
        0
    }

    async fn kill_cgroup_processes(cgroup_path: &Path) -> Result<()> {
        let procs_file = cgroup_path.join("cgroup.procs");

        if !procs_file.exists() {
            return Ok(());
        }

        let procs = tokio::fs::read_to_string(&procs_file).await?;

        for line in procs.lines() {
            if let Ok(pid) = line.trim().parse::<i32>() {
                // Send SIGTERM first
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid),
                    nix::sys::signal::Signal::SIGTERM,
                );
            }
        }

        // Wait a bit for graceful shutdown
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Force kill any remaining processes
        let procs = tokio::fs::read_to_string(&procs_file).await.unwrap_or_default();

        for line in procs.lines() {
            if let Ok(pid) = line.trim().parse::<i32>() {
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid),
                    nix::sys::signal::Signal::SIGKILL,
                );
            }
        }

        Ok(())
    }
}

/// Session resource usage statistics
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    /// Current memory usage in bytes
    pub memory_bytes: u64,

    /// CPU usage in microseconds
    pub cpu_usage_us: u64,

    /// Number of processes
    pub process_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_stat_parsing() {
        let stat = "usage_usec 123456\nuser_usec 100000\nsystem_usec 23456";
        assert_eq!(ResourceManager::parse_cpu_stat(stat), 123456);
    }
}
