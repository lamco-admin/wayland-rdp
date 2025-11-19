//! Headless deployment configuration
//!
//! Configuration for headless RDP server operation including multi-user
//! session management, resource limits, and authentication settings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Complete headless server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadlessConfig {
    /// Server listening configuration
    pub listen_address: String,

    /// Headless mode enable flag
    pub enabled: bool,

    /// Compositor configuration
    pub compositor: CompositorConfig,

    /// Multi-user session configuration
    pub multiuser: MultiUserConfig,

    /// Authentication configuration
    pub authentication: AuthenticationConfig,

    /// Resource management configuration
    pub resources: ResourceConfig,

    /// Portal backend configuration
    pub portal: PortalConfig,

    /// Application auto-start configuration
    pub autostart: AutoStartConfig,

    /// Session management configuration
    pub session: SessionConfig,
}

/// Compositor configuration for headless operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositorConfig {
    /// Compositor type ("smithay", "weston", "cage")
    pub compositor_type: String,

    /// Default virtual display resolution (width, height)
    pub default_resolution: (u32, u32),

    /// Refresh rate in Hz
    pub refresh_rate: u32,

    /// Rendering backend ("llvmpipe", "virgl", "auto")
    pub render_backend: String,

    /// Enable software rendering fallback
    pub software_fallback: bool,

    /// Path to compositor binary (if external)
    pub compositor_path: Option<PathBuf>,

    /// Additional compositor arguments
    pub compositor_args: Vec<String>,

    /// Enable DMA-BUF zero-copy (if hardware available)
    pub enable_dmabuf: bool,

    /// Compositor memory limit (MB, 0 = no limit)
    pub memory_limit: usize,
}

/// Multi-user session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiUserConfig {
    /// Enable multi-user mode
    pub enabled: bool,

    /// Maximum concurrent sessions (0 = unlimited)
    pub max_sessions: usize,

    /// Maximum sessions per user (0 = unlimited)
    pub max_sessions_per_user: usize,

    /// Port allocation strategy ("sequential", "random")
    pub port_allocation: String,

    /// Starting port for user sessions
    pub base_port: u16,

    /// Port range size
    pub port_range: u16,

    /// Enable session persistence
    pub enable_persistence: bool,

    /// Session persistence directory
    pub persistence_dir: PathBuf,

    /// Idle session timeout (seconds, 0 = no timeout)
    pub idle_timeout: u64,

    /// Enable session reconnection
    pub enable_reconnection: bool,

    /// Reconnection timeout (seconds)
    pub reconnection_timeout: u64,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Authentication provider ("pam", "ldap", "none")
    pub provider: String,

    /// PAM service name
    pub pam_service: String,

    /// Enable password authentication
    pub enable_password: bool,

    /// Enable public key authentication
    pub enable_pubkey: bool,

    /// Authorized keys directory
    pub authorized_keys_dir: PathBuf,

    /// Enable two-factor authentication
    pub enable_2fa: bool,

    /// 2FA provider ("totp", "u2f")
    pub twofa_provider: String,

    /// LDAP configuration (if using LDAP auth)
    pub ldap: Option<LdapConfig>,

    /// Enable authentication caching
    pub enable_cache: bool,

    /// Cache timeout (seconds)
    pub cache_timeout: u64,

    /// Maximum failed login attempts
    pub max_failed_attempts: usize,

    /// Lockout duration (seconds)
    pub lockout_duration: u64,
}

/// LDAP authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapConfig {
    /// LDAP server URI
    pub server_uri: String,

    /// Base DN for user search
    pub base_dn: String,

    /// User search filter
    pub user_filter: String,

    /// Bind DN for LDAP authentication
    pub bind_dn: String,

    /// Bind password (or path to file)
    pub bind_password: String,

    /// Enable LDAP over TLS
    pub use_tls: bool,

    /// TLS certificate validation
    pub verify_cert: bool,

    /// Group membership filter
    pub group_filter: Option<String>,

    /// Required group DN
    pub required_group: Option<String>,
}

/// Resource management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Enable cgroup resource isolation
    pub enable_cgroups: bool,

    /// Cgroup controller version (1 or 2)
    pub cgroup_version: u8,

    /// Base cgroup path
    pub cgroup_base: String,

    /// Per-session limits
    pub session_limits: SessionLimits,

    /// System resource limits
    pub system_limits: SystemLimits,

    /// Enable OOM protection
    pub enable_oom_protection: bool,

    /// OOM score adjustment
    pub oom_score_adj: i32,

    /// CPU scheduling priority
    pub cpu_priority: i32,

    /// I/O scheduling class ("realtime", "best-effort", "idle")
    pub io_priority: String,
}

/// Per-session resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLimits {
    /// Maximum memory per session (MB, 0 = unlimited)
    pub max_memory: usize,

    /// Memory swap limit (MB, 0 = no swap)
    pub max_swap: usize,

    /// CPU shares (relative weight)
    pub cpu_shares: usize,

    /// CPU quota (percentage, 100 = 1 full core)
    pub cpu_quota: usize,

    /// Maximum processes per session
    pub max_processes: usize,

    /// Maximum open files per session
    pub max_files: usize,

    /// Disk quota per session (MB, 0 = unlimited)
    pub max_disk: usize,

    /// Network bandwidth limit (kbps, 0 = unlimited)
    pub max_bandwidth: usize,
}

/// System-wide resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLimits {
    /// Total memory limit for all sessions (MB)
    pub total_memory: usize,

    /// Total CPU limit for all sessions (percentage)
    pub total_cpu: usize,

    /// Reserve memory for system (MB)
    pub reserved_memory: usize,

    /// Reserve CPU for system (percentage)
    pub reserved_cpu: usize,
}

/// Portal backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalConfig {
    /// Portal backend type ("embedded", "system")
    pub backend_type: String,

    /// Auto-grant permissions (headless mode)
    pub auto_grant_permissions: bool,

    /// Permission policy
    pub permission_policy: PermissionPolicyConfig,

    /// Enable portal emulation
    pub enable_emulation: bool,

    /// D-Bus connection type ("session", "system")
    pub dbus_connection: String,

    /// Portal timeout (seconds)
    pub timeout: u64,
}

/// Permission policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicyConfig {
    /// Default policy ("allow", "deny", "ask")
    pub default_policy: String,

    /// Screen capture policy
    pub screencast_policy: String,

    /// Input injection policy
    pub remote_desktop_policy: String,

    /// Clipboard policy
    pub clipboard_policy: String,

    /// Per-user policy overrides
    pub user_policies: std::collections::HashMap<String, UserPolicy>,
}

/// Per-user permission policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPolicy {
    /// Allow screen capture
    pub allow_screencast: bool,

    /// Allow input injection
    pub allow_remote_desktop: bool,

    /// Allow clipboard access
    pub allow_clipboard: bool,

    /// Allowed applications
    pub allowed_apps: Vec<String>,

    /// Blocked applications
    pub blocked_apps: Vec<String>,
}

/// Application auto-start configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoStartConfig {
    /// Enable auto-start
    pub enabled: bool,

    /// Applications to auto-start
    pub applications: Vec<AppConfig>,

    /// Startup delay (milliseconds)
    pub startup_delay: u64,

    /// Environment variables
    pub environment: std::collections::HashMap<String, String>,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Application name
    pub name: String,

    /// Command to execute
    pub command: String,

    /// Command arguments
    pub args: Vec<String>,

    /// Working directory
    pub working_dir: Option<PathBuf>,

    /// Restart policy ("no", "on-failure", "always")
    pub restart_policy: String,

    /// Restart delay (milliseconds)
    pub restart_delay: u64,
}

/// Session management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Enable systemd-logind integration
    pub use_systemd_logind: bool,

    /// Session class ("user", "greeter", "lock-screen")
    pub session_class: String,

    /// Session type ("wayland", "x11", "tty")
    pub session_type: String,

    /// Enable session logging
    pub enable_logging: bool,

    /// Session log directory
    pub log_dir: PathBuf,

    /// Enable session recording
    pub enable_recording: bool,

    /// Recording directory
    pub recording_dir: PathBuf,

    /// Session cleanup on logout
    pub cleanup_on_logout: bool,

    /// Kill user processes on logout
    pub kill_user_processes: bool,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            listen_address: "0.0.0.0:3389".to_string(),
            enabled: true,
            compositor: CompositorConfig::default(),
            multiuser: MultiUserConfig::default(),
            authentication: AuthenticationConfig::default(),
            resources: ResourceConfig::default(),
            portal: PortalConfig::default(),
            autostart: AutoStartConfig::default(),
            session: SessionConfig::default(),
        }
    }
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            compositor_type: "smithay".to_string(),
            default_resolution: (1920, 1080),
            refresh_rate: 60,
            render_backend: "llvmpipe".to_string(),
            software_fallback: true,
            compositor_path: None,
            compositor_args: Vec::new(),
            enable_dmabuf: true,
            memory_limit: 512, // 512 MB per compositor
        }
    }
}

impl Default for MultiUserConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_sessions: 10,
            max_sessions_per_user: 2,
            port_allocation: "sequential".to_string(),
            base_port: 3389,
            port_range: 100,
            enable_persistence: true,
            persistence_dir: PathBuf::from("/var/lib/wrd-server/sessions"),
            idle_timeout: 3600, // 1 hour
            enable_reconnection: true,
            reconnection_timeout: 300, // 5 minutes
        }
    }
}

impl Default for AuthenticationConfig {
    fn default() -> Self {
        Self {
            provider: "pam".to_string(),
            pam_service: "wrd-server".to_string(),
            enable_password: true,
            enable_pubkey: false,
            authorized_keys_dir: PathBuf::from("/etc/wrd-server/authorized_keys"),
            enable_2fa: false,
            twofa_provider: "totp".to_string(),
            ldap: None,
            enable_cache: true,
            cache_timeout: 300,
            max_failed_attempts: 5,
            lockout_duration: 900, // 15 minutes
        }
    }
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            enable_cgroups: true,
            cgroup_version: 2,
            cgroup_base: "wrd-server".to_string(),
            session_limits: SessionLimits::default(),
            system_limits: SystemLimits::default(),
            enable_oom_protection: true,
            oom_score_adj: -100,
            cpu_priority: 0,
            io_priority: "best-effort".to_string(),
        }
    }
}

impl Default for SessionLimits {
    fn default() -> Self {
        Self {
            max_memory: 2048,   // 2 GB
            max_swap: 0,        // No swap
            cpu_shares: 1024,   // Default weight
            cpu_quota: 200,     // 2 full cores
            max_processes: 256,
            max_files: 4096,
            max_disk: 10240,    // 10 GB
            max_bandwidth: 0,   // Unlimited
        }
    }
}

impl Default for SystemLimits {
    fn default() -> Self {
        Self {
            total_memory: 16384,    // 16 GB
            total_cpu: 800,         // 8 cores
            reserved_memory: 2048,  // 2 GB for system
            reserved_cpu: 100,      // 1 core for system
        }
    }
}

impl Default for PortalConfig {
    fn default() -> Self {
        Self {
            backend_type: "embedded".to_string(),
            auto_grant_permissions: true,
            permission_policy: PermissionPolicyConfig::default(),
            enable_emulation: true,
            dbus_connection: "session".to_string(),
            timeout: 30,
        }
    }
}

impl Default for PermissionPolicyConfig {
    fn default() -> Self {
        Self {
            default_policy: "allow".to_string(),
            screencast_policy: "allow".to_string(),
            remote_desktop_policy: "allow".to_string(),
            clipboard_policy: "allow".to_string(),
            user_policies: std::collections::HashMap::new(),
        }
    }
}

impl Default for AutoStartConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            applications: Vec::new(),
            startup_delay: 1000, // 1 second
            environment: std::collections::HashMap::new(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            use_systemd_logind: true,
            session_class: "user".to_string(),
            session_type: "wayland".to_string(),
            enable_logging: true,
            log_dir: PathBuf::from("/var/log/wrd-server"),
            enable_recording: false,
            recording_dir: PathBuf::from("/var/lib/wrd-server/recordings"),
            cleanup_on_logout: true,
            kill_user_processes: false,
        }
    }
}

impl HeadlessConfig {
    /// Load configuration from TOML file
    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate compositor settings
        if self.compositor.default_resolution.0 < 640 || self.compositor.default_resolution.1 < 480
        {
            anyhow::bail!("Resolution too small (minimum 640x480)");
        }

        // Validate multi-user settings
        if self.multiuser.enabled {
            if self.multiuser.max_sessions == 0 {
                tracing::warn!("Unlimited sessions enabled - may cause resource exhaustion");
            }
            if self.multiuser.base_port + self.multiuser.port_range > 65535 {
                anyhow::bail!("Port range exceeds maximum port number");
            }
        }

        // Validate resource limits
        if self.resources.session_limits.max_memory > self.resources.system_limits.total_memory {
            anyhow::bail!("Per-session memory limit exceeds total system limit");
        }

        Ok(())
    }
}
