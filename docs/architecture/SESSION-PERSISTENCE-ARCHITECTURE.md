# Session Persistence & Unattended Access Architecture

**Document Version:** 1.2.0
**Date:** 2025-12-31
**Status:** Architecture Design (Pre-Implementation)
**Classification:** Core Architecture Extension
**Related Documents:** FAILURE-MODES-AND-FALLBACKS.md

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Problem Statement](#problem-statement)
3. [Architecture Overview](#architecture-overview)
4. [Deployment Constraints & Strategy Availability](#deployment-constraints--strategy-availability)
5. [Service Registry Extensions](#service-registry-extensions)
6. [Multi-Strategy Session Management](#multi-strategy-session-management)
7. [Credential Storage Architecture](#credential-storage-architecture)
8. [Permission Dialog Avoidance Matrix](#permission-dialog-avoidance-matrix)
9. [Implementation Roadmap](#implementation-roadmap)
10. [Configuration Schema](#configuration-schema)
11. [Failure Modes & Robustness Strategy](#failure-modes--robustness-strategy)
12. [Security Considerations](#security-considerations)
13. [Testing Strategy](#testing-strategy)
14. [References](#references)

---

## Executive Summary

This document defines the architecture for enabling unattended operation of lamco-rdp-server across diverse Linux desktop environments. The core challenge is Wayland's security model, which requires explicit user consent for screen capture and input injection—a fundamental barrier to server-style operation.

### Key Design Decisions

1. **Multi-Strategy Approach:** No single solution works across all compositors. We implement multiple strategies selected at runtime based on detected capabilities.

2. **Service Registry Integration:** Session persistence capabilities are exposed through the existing Service Advertisement Registry, enabling runtime decisions and user visibility.

3. **Deployment-Aware Architecture:** Deployment method (Flatpak, native package, systemd user/system service, initd) fundamentally constrains available strategies. Detection is automatic and strategy selection adapts accordingly.

4. **Environment-Adaptive Credential Storage:** Token encryption method varies by detected environment (Flatpak Secret Portal, Secret Service, TPM 2.0, or encrypted file fallback).

5. **Graceful Degradation:** If unattended operation isn't possible, the system clearly communicates what manual intervention is required.

6. **Flatpak-First Portability:** Phase 1 + 2 implementation supports Flatpak deployment, ensuring maximum distribution reach while maintaining security.

7. **wlr-screencopy Reservation:** A direct Wayland protocol capture backend remains a documented fallback if portal-based approaches prove insufficient (deferred priority).

### What This Enables

| Scenario | Before | After |
|----------|--------|-------|
| Server reboot | Manual dialog click required | Automatic session restoration |
| Network disconnect/reconnect | Manual dialog click required | Automatic if token valid |
| First-time setup | Manual dialog click required | Manual dialog (unavoidable) |
| Headless initial setup | Not possible | SSH-assisted grant flow |
| systemd service | Would hang waiting for dialog | Starts with stored token |

---

## Problem Statement

### The Wayland Permission Model

Wayland compositors enforce strict isolation between applications. Screen capture and input injection require explicit user consent mediated by:

1. **XDG Desktop Portal** - D-Bus service that presents permission dialogs
2. **Compositor-specific backends** - Implement the actual capture/injection
3. **PipeWire** - Delivers video frames after permission granted

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    CURRENT PERMISSION FLOW                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  RDP Client ─── connects ───> lamco-rdp-server                          │
│                                    │                                     │
│                                    ▼                                     │
│                           PortalManager.create_session()                 │
│                                    │                                     │
│                                    ▼                                     │
│                        xdg-desktop-portal (D-Bus)                       │
│                                    │                                     │
│                                    ▼                                     │
│                    ┌───────────────────────────────┐                    │
│                    │   PERMISSION DIALOG APPEARS   │ ◄── BLOCKING!      │
│                    │   "Allow screen sharing?"     │                    │
│                    │   [Deny]  [Allow Once]  [Allow]│                    │
│                    └───────────────────────────────┘                    │
│                                    │                                     │
│                              User clicks                                │
│                                    │                                     │
│                                    ▼                                     │
│                         PipeWire stream created                         │
│                                    │                                     │
│                                    ▼                                     │
│                         Video flows to client                           │
│                                                                          │
│  PROBLEM: Every server restart, every reconnect triggers this dialog    │
└─────────────────────────────────────────────────────────────────────────┘
```

### Why This Matters for Server Operation

| Use Case | Impact |
|----------|--------|
| Headless server with DE | Cannot start without monitor to click dialog |
| systemd service | Service hangs indefinitely waiting for user |
| Auto-reconnect after network drop | Client disconnects, dialog required to resume |
| Reboot recovery | Server restarts, requires manual intervention |
| VDI/Terminal server | Every user session needs manual approval |

### Compositor-Specific Variations

| Compositor | Portal Backend | Dialog Behavior | Bypass Options |
|------------|---------------|-----------------|----------------|
| GNOME (Mutter) | xdg-desktop-portal-gnome | Integrated GNOME dialog | Mutter D-Bus API |
| KDE Plasma (KWin) | xdg-desktop-portal-kde | KDE dialog | None (portal only) |
| Sway | xdg-desktop-portal-wlr | slurp/wofi selector | wlr-screencopy |
| Hyprland | xdg-desktop-portal-hyprland | hyprland-share-picker | wlr-screencopy |
| Labwc | xdg-desktop-portal-wlr | External picker | wlr-screencopy |
| Cosmic | xdg-desktop-portal-cosmic | COSMIC dialog | TBD |

---

## Architecture Overview

### Multi-Strategy Session Persistence

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    SESSION PERSISTENCE ARCHITECTURE                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                      ServiceRegistry                              │   │
│  │  (Extended with Session Persistence Capabilities)                 │   │
│  │                                                                   │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │   │
│  │  │SessionPersistence│  │DirectCompositor │  │CredentialStorage│  │   │
│  │  │ ServiceId       │  │ API ServiceId   │  │ ServiceId       │  │   │
│  │  │                 │  │                 │  │                 │  │   │
│  │  │ Level: varies   │  │ Level: varies   │  │ Level: varies   │  │   │
│  │  │ per compositor  │  │ GNOME only      │  │ per environment │  │   │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│                                    ▼                                     │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                   SessionStrategySelector                         │   │
│  │  (Chooses best strategy based on ServiceRegistry)                 │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│           ┌────────────────────────┼────────────────────────┐           │
│           │                        │                        │           │
│           ▼                        ▼                        ▼           │
│  ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐   │
│  │ Strategy 1:     │     │ Strategy 2:     │     │ Strategy 3:     │   │
│  │ Portal + Token  │     │ Mutter Direct   │     │ wlr-screencopy  │   │
│  │                 │     │                 │     │                 │   │
│  │ Works: All      │     │ Works: GNOME    │     │ Works: wlroots  │   │
│  │ Portal: v4+     │     │ Only            │     │ compositors     │   │
│  └─────────────────┘     └─────────────────┘     └─────────────────┘   │
│           │                        │                        │           │
│           └────────────────────────┼────────────────────────┘           │
│                                    ▼                                     │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                     TokenManager                                  │   │
│  │  (Encrypted storage, lifecycle management)                        │   │
│  │                                                                   │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │   │
│  │  │ Secret      │  │ TPM 2.0 +   │  │ Encrypted   │              │   │
│  │  │ Service API │  │ systemd-    │  │ File        │              │   │
│  │  │ (libsecret) │  │ creds       │  │ (fallback)  │              │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘              │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility |
|-----------|---------------|
| `ServiceRegistry` | Detect and advertise session persistence capabilities |
| `SessionStrategySelector` | Choose optimal strategy based on capabilities |
| `TokenManager` | Store, retrieve, and refresh session tokens securely |
| `PortalSessionManager` | Implement portal-based token flow |
| `MutterSessionManager` | Implement GNOME Mutter direct API (optional) |
| `WlrCaptureBackend` | Direct wlr-screencopy capture (fallback option) |

---

## Deployment Constraints & Strategy Availability

### Critical Deployment Insight

**The deployment method fundamentally constrains which session persistence strategies are available.** Sandboxing (Flatpak), service context (systemd user vs system), and init system choice all create hard boundaries around what's technically possible.

### Deployment Context Types

```rust
/// Deployment context enum in services/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentContext {
    /// Native system package (full access)
    /// - Direct Wayland socket access
    /// - Direct D-Bus session access
    /// - All credential storage methods
    Native,

    /// Flatpak sandbox (restricted)
    /// - Portal access ONLY
    /// - No direct Wayland/D-Bus access
    /// - Limited credential storage
    Flatpak,

    /// systemd user service
    /// - Runs as user in user session
    /// - D-Bus session access
    /// - Can use loginctl enable-linger for headless
    SystemdUser { linger_enabled: bool },

    /// systemd system service
    /// - Runs as system service (multi-user)
    /// - Complex D-Bus session bridging required
    /// - Limited portal access
    SystemdSystem,

    /// Non-systemd init (OpenRC, runit, etc.)
    /// - Manual D-Bus session setup required
    /// - No systemd-creds (no TPM support)
    /// - Complex configuration
    InitD,
}
```

### Strategy Availability by Deployment

```
┌─────────────────────────────────────────────────────────────────────────┐
│              STRATEGY AVAILABILITY MATRIX BY DEPLOYMENT                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Deployment Type │ Portal+Token │ Mutter API │ wlr-screencopy │ Creds  │
│  ────────────────┼──────────────┼────────────┼────────────────┼────────┤
│  Native Package  │      ✅       │     ✅      │       ✅        │  All   │
│  Flatpak         │      ✅       │     ❌      │       ❌        │Limited │
│  systemd --user  │      ✅       │     ✅      │       ✅        │  All   │
│  systemd system  │      ⚠️       │     ❌      │       ❌        │Limited │
│  initd/OpenRC    │      ✅       │     ⚠️      │       ⚠️        │Limited │
│                                                                          │
│  Legend:                                                                 │
│  ✅ = Fully supported                                                    │
│  ⚠️  = Possible but complex configuration required                       │
│  ❌ = Blocked by deployment constraints                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Flatpak Deployment Constraints

Flatpak sandboxing is **the most restrictive environment** but offers maximum portability and security.

#### What Works in Flatpak

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    FLATPAK SANDBOX CAPABILITIES                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ✅ AVAILABLE:                                                           │
│  ────────────                                                            │
│  • XDG Desktop Portal (org.freedesktop.portal.*)                         │
│  • Portal restore tokens (if portal v4+)                                 │
│  • Flatpak Secret Portal (org.freedesktop.portal.Secret)                 │
│  • Network access for RDP                                                │
│  • Encrypted file storage in app data directory                          │
│                                                                          │
│  ❌ BLOCKED:                                                             │
│  ─────────                                                               │
│  • Direct Wayland socket access ($WAYLAND_DISPLAY)                       │
│  • Direct D-Bus session bus (for Mutter/compositor APIs)                 │
│  • Direct Secret Service (org.freedesktop.secrets)                       │
│  • systemd-creds / TPM 2.0 access                                        │
│  • wlr-screencopy protocol access                                        │
│                                                                          │
│  RESULT: Portal + Token strategy ONLY                                    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Flatpak Manifest Requirements

```xml
<!-- org.lamco.RdpServer.yaml -->
app-id: org.lamco.RdpServer
runtime: org.freedesktop.Platform
runtime-version: '23.08'
sdk: org.freedesktop.Sdk
command: lamco-rdp-server

finish-args:
  # Portal access (REQUIRED for all strategies)
  - --socket=wayland
  - --socket=fallback-x11
  - --share=ipc

  # D-Bus portal access
  - --socket=session-bus
  - --talk-name=org.freedesktop.portal.Desktop
  - --talk-name=org.freedesktop.portal.ScreenCast
  - --talk-name=org.freedesktop.portal.RemoteDesktop

  # Flatpak Secret Portal for credential storage
  - --talk-name=org.freedesktop.portal.Secret

  # Network for RDP connections
  - --share=network

  # App data directory for token storage
  - --filesystem=xdg-data/lamco-rdp-server:create

  # OPTIONAL: System Secret Service (may not work in all sandboxes)
  - --talk-name=org.freedesktop.secrets
```

#### Flatpak Credential Storage Strategy

In Flatpak, credential storage options are limited:

1. **Flatpak Secret Portal** (Recommended)
   - Portal: `org.freedesktop.portal.Secret`
   - Backend: Host system keyring (GNOME Keyring, KWallet, etc.)
   - Access: Via portal, properly sandboxed
   - Encryption: Handled by host keyring

2. **Encrypted File in App Data**
   - Location: `~/.var/app/org.lamco.RdpServer/data/lamco-rdp-server/`
   - Encryption: AES-256-GCM with app-specific key derivation
   - Persistence: Survives app updates
   - Security: Limited (key derived from app ID + machine ID)

3. **NOT AVAILABLE in Flatpak:**
   - Direct Secret Service (org.freedesktop.secrets)
   - TPM 2.0 via systemd-creds
   - Direct GNOME Keyring/KWallet access

### systemd User Service Deployment

**Recommended for single-user workstations and servers.**

```ini
# ~/.config/systemd/user/lamco-rdp-server.service

[Unit]
Description=lamco RDP Server
Documentation=https://lamco.ai/docs/rdp-server
After=graphical-session.target
Wants=graphical-session.target

[Service]
Type=notify
ExecStart=/usr/bin/lamco-rdp-server
Restart=on-failure
RestartSec=5s

# Ensure D-Bus session access
Environment="DBUS_SESSION_BUS_ADDRESS=unix:path=%t/bus"
Environment="XDG_RUNTIME_DIR=%t"

# Logging
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
```

**Enable headless operation (survives logout):**

```bash
# Allow user service to persist without active login
loginctl enable-linger $USER

# Enable service
systemctl --user enable lamco-rdp-server.service

# Start service
systemctl --user start lamco-rdp-server.service

# Check status
systemctl --user status lamco-rdp-server.service
```

**Available Strategies:**
- ✅ Portal + Token
- ✅ Mutter Direct API (GNOME)
- ✅ wlr-screencopy (wlroots)

**Credential Storage:**
- ✅ Secret Service (if keyring unlocked)
- ✅ TPM 2.0 via systemd-creds
- ✅ Encrypted file

### systemd System Service Deployment

**For multi-user VDI/terminal server scenarios (advanced).**

This is the most complex deployment due to per-user session management requirements.

```ini
# /etc/systemd/system/lamco-rdp-server@.service
# Template service for per-user instances

[Unit]
Description=lamco RDP Server for user %i
Documentation=https://lamco.ai/docs/rdp-server
After=network.target user@%i.service

[Service]
Type=notify
User=%i
Group=%i

# Critical: Set up user environment
Environment="XDG_RUNTIME_DIR=/run/user/%U"
Environment="DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/%U/bus"

# Ensure runtime directory exists
ExecStartPre=/bin/mkdir -p /run/user/%U
ExecStartPre=/bin/chown %i:%i /run/user/%U

# Start server in user context
ExecStart=/usr/bin/lamco-rdp-server --user-mode

# Security hardening
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

**Enable for specific user:**

```bash
# Enable service for user 'john'
systemctl enable lamco-rdp-server@john.service
systemctl start lamco-rdp-server@john.service
```

**Challenges:**

1. **D-Bus Session Access:** System service must bridge to user's D-Bus session
2. **Portal Permissions:** Per-user permission grants required
3. **Credential Storage:** Limited to per-user encrypted files
4. **Complexity:** High operational overhead

**Available Strategies:**
- ⚠️ Portal + Token (complex D-Bus bridging)
- ❌ Mutter Direct API (session context issues)
- ❌ wlr-screencopy (session context issues)

**Recommendation:** Use systemd user service with `loginctl enable-linger` instead for simpler operation.

### initd / OpenRC Deployment

**For non-systemd distributions (Gentoo, Devuan, Alpine, etc.).**

initd/OpenRC deployment requires manual D-Bus session management and lacks systemd-specific features.

```bash
#!/sbin/openrc-run
# /etc/init.d/lamco-rdp-server

name="lamco RDP Server"
description="Wayland RDP Server"

command="/usr/bin/lamco-rdp-server"
command_user="lamco:lamco"
pidfile="/run/lamco-rdp-server.pid"

depend() {
    need dbus
    use net
    after graphical
}

start_pre() {
    # Set up user environment
    export XDG_RUNTIME_DIR="/run/user/$(id -u lamco)"
    export DBUS_SESSION_BUS_ADDRESS="unix:path=${XDG_RUNTIME_DIR}/bus"

    # Ensure runtime directory exists
    mkdir -p "$XDG_RUNTIME_DIR"
    chown lamco:lamco "$XDG_RUNTIME_DIR"

    # Start D-Bus user session if not running
    if [ ! -S "$DBUS_SESSION_BUS_ADDRESS" ]; then
        dbus-daemon --session --fork --address="$DBUS_SESSION_BUS_ADDRESS"
    fi
}
```

**Limitations:**

- ❌ No `systemd-creds` (no TPM support)
- ❌ No `loginctl enable-linger` equivalent
- ⚠️ Manual D-Bus session management required
- ⚠️ Complex session persistence

**Available Strategies:**
- ✅ Portal + Token (with manual setup)
- ⚠️ Mutter/wlr (possible but complex)

**Credential Storage:**
- ✅ Secret Service (if available and unlocked)
- ❌ TPM 2.0 (requires systemd)
- ✅ Encrypted file

**Recommendation:** Document as "advanced/community-supported" due to complexity.

### Deployment Context Detection

The system automatically detects deployment context at runtime:

```rust
/// Detect deployment context in src/compositor/probing.rs

pub fn detect_deployment_context() -> DeploymentContext {
    // Check if running in Flatpak
    if Path::new("/.flatpak-info").exists() {
        info!("Detected Flatpak deployment");
        return DeploymentContext::Flatpak;
    }

    // Check if running as systemd service
    if let Ok(invocation_id) = env::var("INVOCATION_ID") {
        // systemd sets INVOCATION_ID for all units

        // Check if user or system service
        if let Ok(user_unit) = env::var("XDG_RUNTIME_DIR") {
            // User service has XDG_RUNTIME_DIR

            // Check if linger is enabled
            let uid = unsafe { libc::getuid() };
            let linger_path = format!("/var/lib/systemd/linger/{}",
                users::get_user_by_uid(uid).map(|u| u.name().to_string_lossy().to_string())
                    .unwrap_or_else(|| uid.to_string())
            );

            let linger_enabled = Path::new(&linger_path).exists();

            info!("Detected systemd user service (linger: {})", linger_enabled);
            return DeploymentContext::SystemdUser { linger_enabled };
        } else {
            // System service lacks user environment
            info!("Detected systemd system service");
            return DeploymentContext::SystemdSystem;
        }
    }

    // Check for systemd presence (even if not running as service)
    if Path::new("/run/systemd/system").exists() {
        info!("systemd available but not running as service");
        // Running directly, not as service
        return DeploymentContext::Native;
    }

    // Check for OpenRC
    if Path::new("/run/openrc").exists() {
        info!("Detected OpenRC init system");
        return DeploymentContext::InitD;
    }

    // Default: assume native package
    info!("Detected native package deployment");
    DeploymentContext::Native
}
```

### Deployment Recommendation Matrix

| Use Case | Recommended Deployment | Rationale |
|----------|----------------------|-----------|
| Single-user workstation | systemd user service + linger | Full feature access, simple setup |
| Headless server (single user) | systemd user service + linger | Unattended operation, full strategies |
| Multi-user terminal server | systemd system service (advanced) | Per-user isolation, complex |
| Maximum portability | Flatpak | Works everywhere, limited to portal strategy |
| Development/testing | Native binary | Full access for debugging |
| Distribution packaging | Native .deb/.rpm + systemd | Standard Linux packaging |
| Security-focused deployment | Flatpak | Sandboxing, limited attack surface |
| Non-systemd distributions | Native + initd/OpenRC script | Community-supported |

### Strategy Selection with Deployment Context

The `SessionStrategySelector` adapts to deployment constraints:

```rust
impl SessionStrategySelector {
    pub fn select_strategy(&self) -> Box<dyn SessionStrategy> {
        let deployment = &self.registry.compositor_capabilities().deployment;

        // DEPLOYMENT CONSTRAINT ENFORCEMENT
        match deployment {
            DeploymentContext::Flatpak => {
                // Flatpak: ONLY portal strategy available
                info!("Flatpak: Portal + Token is only available strategy");
                if self.registry.service_level(ServiceId::SessionPersistence)
                    < ServiceLevel::BestEffort {
                    warn!("Portal version < 4, tokens not supported in Flatpak");
                }
                return Box::new(PortalTokenStrategy::new(
                    self.registry.clone(),
                    CredentialStorageMethod::FlatpakSecretPortal
                ));
            }

            DeploymentContext::SystemdSystem => {
                // System service: Limited to portal due to D-Bus session complexity
                warn!("System service: Limited to Portal strategy (D-Bus bridging complex)");
                return Box::new(PortalTokenStrategy::new(
                    self.registry.clone(),
                    CredentialStorageMethod::EncryptedFile
                ));
            }

            DeploymentContext::InitD => {
                // initd: Prefer portal, warn about limitations
                warn!("initd deployment: Limited credential storage options");
                // Fall through to normal strategy selection
            }

            _ => {} // Native, SystemdUser - full strategy access
        }

        // NORMAL STRATEGY SELECTION (for unrestricted deployments)

        // Priority 1: wlr-screencopy (no dialog, wlroots only)
        if self.registry.service_level(ServiceId::WlrScreencopy) >= ServiceLevel::Guaranteed {
            info!("Selected wlr-screencopy strategy (no portal dialog)");
            return Box::new(WlrScreencopyStrategy::new());
        }

        // Priority 2: Mutter Direct API (no dialog, GNOME only)
        if self.registry.service_level(ServiceId::DirectCompositorAPI) >= ServiceLevel::BestEffort {
            info!("Selected Mutter Direct API strategy (GNOME)");
            return Box::new(MutterDirectStrategy::new());
        }

        // Priority 3: Portal with restore token
        if self.registry.service_level(ServiceId::SessionPersistence) >= ServiceLevel::BestEffort {
            info!("Selected Portal + restore token strategy");

            // Detect best credential storage for deployment
            let storage_method = self.select_credential_storage(deployment);

            return Box::new(PortalTokenStrategy::new(
                self.registry.clone(),
                storage_method
            ));
        }

        // Fallback: Basic portal (dialog every time)
        warn!("No unattended access strategy available");
        Box::new(BasicPortalStrategy::new())
    }

    fn select_credential_storage(&self, deployment: &DeploymentContext) -> CredentialStorageMethod {
        match deployment {
            DeploymentContext::Flatpak => {
                // Try Flatpak Secret Portal first
                if check_flatpak_secret_portal_available() {
                    CredentialStorageMethod::FlatpakSecretPortal
                } else {
                    CredentialStorageMethod::EncryptedFile
                }
            }
            DeploymentContext::SystemdUser { .. } |
            DeploymentContext::Native => {
                // Full access to all storage methods
                // Detection logic from credential storage section
                detect_best_credential_storage()
            }
            DeploymentContext::SystemdSystem |
            DeploymentContext::InitD => {
                // Limited to encrypted file (safest)
                CredentialStorageMethod::EncryptedFile
            }
        }
    }
}
```

---

## Service Registry Extensions

### New ServiceId Variants

```rust
/// Extended ServiceId enum in services/service.rs
pub enum ServiceId {
    // ... existing services ...

    /// Session persistence capability (portal restore tokens)
    /// Indicates whether permission dialogs can be avoided on reconnect
    SessionPersistence,

    /// Direct compositor API availability (bypasses portal)
    /// Currently only available on GNOME via Mutter D-Bus interfaces
    DirectCompositorAPI,

    /// Secure credential storage capability
    /// Varies by DE: Secret Service, TPM 2.0, or encrypted file
    CredentialStorage,

    /// Unattended access readiness
    /// Aggregate: can we start without user interaction?
    UnattendedAccess,

    /// wlr-screencopy protocol availability (wlroots bypass)
    /// Enables portal-free capture on Sway, Hyprland, Labwc, etc.
    WlrScreencopy,
}
```

### New WaylandFeature Variants

```rust
/// Extended WaylandFeature enum in services/wayland_features.rs
pub enum WaylandFeature {
    // ... existing features ...

    /// Session persistence capabilities
    SessionPersistence {
        /// Portal supports restore tokens (v4+)
        restore_token_supported: bool,
        /// Maximum persist mode (0=none, 1=transient, 2=permanent)
        max_persist_mode: u8,
        /// How tokens are stored
        token_storage: TokenStorageMethod,
        /// Token returned from last session (if any)
        current_token: Option<String>,
    },

    /// Mutter direct API (GNOME only)
    MutterDirectAPI {
        /// GNOME Shell version
        version: Option<String>,
        /// org.gnome.Mutter.ScreenCast available
        has_screencast: bool,
        /// org.gnome.Mutter.RemoteDesktop available
        has_remote_desktop: bool,
    },

    /// Credential storage method
    CredentialStorage {
        /// Primary storage method available
        method: CredentialStorageMethod,
        /// Is storage unlocked/accessible?
        is_accessible: bool,
        /// Encryption algorithm used
        encryption: EncryptionType,
    },

    /// wlr-screencopy protocol
    WlrScreencopy {
        /// Protocol version
        version: u32,
        /// Supports DMA-BUF output
        dmabuf_supported: bool,
        /// Supports damage tracking
        damage_supported: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenStorageMethod {
    /// No token storage available
    None,
    /// Tokens stored in encrypted file
    EncryptedFile,
    /// Tokens stored via Secret Service API
    SecretService,
    /// Tokens stored via TPM 2.0 + systemd-creds
    Tpm2SystemdCreds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialStorageMethod {
    /// No secure storage available
    None,
    /// GNOME Keyring via libsecret (direct access)
    GnomeKeyring,
    /// KDE Wallet via libsecret/kwallet (direct access)
    KWallet,
    /// KeePassXC via Secret Service (direct access)
    KeePassXC,
    /// Flatpak Secret Portal (sandboxed access to host keyring)
    /// Uses org.freedesktop.portal.Secret
    FlatpakSecretPortal,
    /// TPM 2.0 bound storage via systemd-creds
    Tpm2,
    /// Encrypted file with machine-bound key
    EncryptedFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionType {
    /// No encryption
    None,
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
    /// TPM-bound (key never leaves TPM)
    TpmBound,
    /// Host keyring encryption (Flatpak Secret Portal)
    /// Encryption handled by host system's keyring
    HostKeyring,
}
```

### Translation Logic

```rust
/// New translation functions in services/translation.rs

fn translate_session_persistence(caps: &CompositorCapabilities) -> AdvertisedService {
    let portal_version = caps.portal.version;

    // Check for portal v4+ (restore token support)
    let restore_supported = portal_version >= 4;

    // Detect token storage method
    let storage_method = detect_token_storage_method();

    let feature = WaylandFeature::SessionPersistence {
        restore_token_supported: restore_supported,
        max_persist_mode: if restore_supported { 2 } else { 0 },
        token_storage: storage_method,
        current_token: load_existing_token(),
    };

    let level = match (restore_supported, storage_method) {
        (true, TokenStorageMethod::SecretService) => ServiceLevel::Guaranteed,
        (true, TokenStorageMethod::Tpm2SystemdCreds) => ServiceLevel::Guaranteed,
        (true, TokenStorageMethod::EncryptedFile) => ServiceLevel::BestEffort,
        (true, TokenStorageMethod::None) => ServiceLevel::Degraded,
        (false, _) => ServiceLevel::Unavailable,
    };

    let mut service = AdvertisedService::new(ServiceId::SessionPersistence, feature, level);

    if !restore_supported {
        service = service.with_note(&format!(
            "Portal v{} < v4, restore tokens not supported",
            portal_version
        ));
    }

    service
}

fn translate_direct_compositor_api(caps: &CompositorCapabilities) -> AdvertisedService {
    match &caps.compositor {
        CompositorType::Gnome { version } => {
            // Check for Mutter D-Bus interfaces
            let has_screencast = check_dbus_interface("org.gnome.Mutter.ScreenCast");
            let has_remote_desktop = check_dbus_interface("org.gnome.Mutter.RemoteDesktop");

            if has_screencast && has_remote_desktop {
                let feature = WaylandFeature::MutterDirectAPI {
                    version: version.clone(),
                    has_screencast,
                    has_remote_desktop,
                };

                // Mutter API stability depends on version
                let level = match version.as_ref().and_then(|v| parse_gnome_version(v)) {
                    Some(v) if v >= 45.0 => ServiceLevel::Guaranteed,
                    Some(v) if v >= 42.0 => ServiceLevel::BestEffort,
                    _ => ServiceLevel::Degraded,
                };

                AdvertisedService::new(ServiceId::DirectCompositorAPI, feature, level)
                    .with_note("Mutter D-Bus API bypasses portal permission dialog")
            } else {
                AdvertisedService::unavailable(ServiceId::DirectCompositorAPI)
            }
        }
        _ => AdvertisedService::unavailable(ServiceId::DirectCompositorAPI),
    }
}

fn translate_credential_storage(caps: &CompositorCapabilities) -> AdvertisedService {
    let deployment = &caps.deployment;
    let (method, encryption, accessible) = detect_credential_storage(deployment);

    let feature = WaylandFeature::CredentialStorage {
        method,
        is_accessible: accessible,
        encryption,
    };

    let level = match (method, accessible) {
        (CredentialStorageMethod::Tpm2, true) => ServiceLevel::Guaranteed,
        (CredentialStorageMethod::GnomeKeyring, true) => ServiceLevel::Guaranteed,
        (CredentialStorageMethod::KWallet, true) => ServiceLevel::Guaranteed,
        (CredentialStorageMethod::KeePassXC, true) => ServiceLevel::Guaranteed,
        (CredentialStorageMethod::FlatpakSecretPortal, true) => ServiceLevel::Guaranteed,
        (CredentialStorageMethod::EncryptedFile, true) => ServiceLevel::BestEffort,
        (_, false) => ServiceLevel::Degraded, // Storage exists but locked
        (CredentialStorageMethod::None, _) => ServiceLevel::Unavailable,
    };

    let note = match deployment {
        DeploymentContext::Flatpak if method == CredentialStorageMethod::FlatpakSecretPortal => {
            Some("Using Flatpak Secret Portal (host keyring via sandbox)".to_string())
        }
        DeploymentContext::Flatpak => {
            Some("Using encrypted file (Secret Portal unavailable)".to_string())
        }
        _ => None,
    };

    let mut service = AdvertisedService::new(ServiceId::CredentialStorage, feature, level);
    if let Some(n) = note {
        service = service.with_note(&n);
    }
    service
}

fn translate_wlr_screencopy(caps: &CompositorCapabilities) -> AdvertisedService {
    if !caps.compositor.is_wlroots_based() {
        return AdvertisedService::unavailable(ServiceId::WlrScreencopy);
    }

    // Check for wlr-screencopy-unstable-v1 protocol
    if let Some(version) = caps.get_protocol_version("zwlr_screencopy_manager_v1") {
        let feature = WaylandFeature::WlrScreencopy {
            version,
            dmabuf_supported: caps.has_protocol("linux_dmabuf_v1"),
            damage_supported: version >= 3, // Damage added in v3
        };

        AdvertisedService::guaranteed(ServiceId::WlrScreencopy, feature)
            .with_note("Direct capture without portal permission dialog")
    } else {
        AdvertisedService::unavailable(ServiceId::WlrScreencopy)
    }
}

fn translate_unattended_access(caps: &CompositorCapabilities) -> AdvertisedService {
    // Aggregate check: can we operate without user intervention?

    let session_persistence = translate_session_persistence(caps);
    let direct_api = translate_direct_compositor_api(caps);
    let wlr_screencopy = translate_wlr_screencopy(caps);
    let credential_storage = translate_credential_storage(caps);

    // Determine overall unattended capability
    let can_avoid_dialog =
        session_persistence.level >= ServiceLevel::BestEffort ||
        direct_api.level >= ServiceLevel::BestEffort ||
        wlr_screencopy.level >= ServiceLevel::Guaranteed;

    let can_store_credentials =
        credential_storage.level >= ServiceLevel::BestEffort;

    let (level, note) = match (can_avoid_dialog, can_store_credentials) {
        (true, true) => (
            ServiceLevel::Guaranteed,
            "Full unattended operation available"
        ),
        (true, false) => (
            ServiceLevel::BestEffort,
            "Dialog avoidance available, credential storage limited"
        ),
        (false, true) => (
            ServiceLevel::Degraded,
            "Credential storage available, but dialog required each session"
        ),
        (false, false) => (
            ServiceLevel::Unavailable,
            "Manual intervention required for each session"
        ),
    };

    AdvertisedService::new(
        ServiceId::UnattendedAccess,
        WaylandFeature::UnattendedAccess {
            can_avoid_dialog,
            can_store_credentials,
        },
        level,
    ).with_note(note)
}
```

---

## Multi-Strategy Session Management

### Strategy Selection Algorithm

```rust
/// SessionStrategySelector in src/session/strategy.rs

pub struct SessionStrategySelector {
    registry: Arc<ServiceRegistry>,
}

impl SessionStrategySelector {
    /// Select the best session strategy based on detected capabilities
    pub fn select_strategy(&self) -> Box<dyn SessionStrategy> {
        // Priority 1: wlr-screencopy (no dialog ever, wlroots only)
        if self.registry.service_level(ServiceId::WlrScreencopy) >= ServiceLevel::Guaranteed {
            info!("Selected wlr-screencopy strategy (no portal dialog)");
            return Box::new(WlrScreencopyStrategy::new());
        }

        // Priority 2: Mutter Direct API (no dialog after setup, GNOME only)
        if self.registry.service_level(ServiceId::DirectCompositorAPI) >= ServiceLevel::BestEffort {
            info!("Selected Mutter Direct API strategy (GNOME)");
            return Box::new(MutterDirectStrategy::new());
        }

        // Priority 3: Portal with restore token (works on most compositors)
        if self.registry.service_level(ServiceId::SessionPersistence) >= ServiceLevel::BestEffort {
            info!("Selected Portal + restore token strategy");
            return Box::new(PortalTokenStrategy::new(self.registry.clone()));
        }

        // Fallback: Portal without persistence (dialog every time)
        warn!("No unattended access strategy available, using basic portal");
        Box::new(BasicPortalStrategy::new())
    }
}

/// Common interface for session strategies
pub trait SessionStrategy: Send + Sync {
    /// Human-readable strategy name
    fn name(&self) -> &'static str;

    /// Does this strategy require initial user interaction?
    fn requires_initial_setup(&self) -> bool;

    /// Can this strategy restore sessions without user interaction?
    fn supports_unattended_restore(&self) -> bool;

    /// Create a new capture session
    async fn create_session(&self, config: &SessionConfig) -> Result<SessionHandle>;

    /// Attempt to restore a previous session without user interaction
    async fn restore_session(&self, token: &SessionToken) -> Result<Option<SessionHandle>>;

    /// Clean up session resources
    async fn cleanup(&self, session: &SessionHandle) -> Result<()>;
}
```

### Strategy 1: Portal + Token (Primary)

```rust
/// PortalTokenStrategy in src/session/strategies/portal_token.rs

pub struct PortalTokenStrategy {
    registry: Arc<ServiceRegistry>,
    token_manager: Arc<TokenManager>,
}

impl SessionStrategy for PortalTokenStrategy {
    fn name(&self) -> &'static str { "Portal + Restore Token" }
    fn requires_initial_setup(&self) -> bool { true } // First time needs dialog
    fn supports_unattended_restore(&self) -> bool { true }

    async fn create_session(&self, config: &SessionConfig) -> Result<SessionHandle> {
        // Load any existing token
        let restore_token = self.token_manager.load_token(&config.session_id).await?;

        // Configure portal with persistence
        let portal_config = PortalConfig {
            persist_mode: PersistMode::ExplicitlyRevoked, // Mode 2
            restore_token,
            cursor_mode: config.cursor_mode,
            ..Default::default()
        };

        // Create portal session
        let manager = PortalManager::new(portal_config).await?;
        let (handle, new_token) = manager.create_session_with_token(
            config.session_id.clone()
        ).await?;

        // Store the new token (tokens are single-use, always save the new one)
        if let Some(token) = new_token {
            self.token_manager.save_token(&config.session_id, &token).await?;
            info!("Session token saved for future restoration");
        }

        Ok(handle)
    }

    async fn restore_session(&self, token: &SessionToken) -> Result<Option<SessionHandle>> {
        // Load stored portal restore token
        let restore_token = match self.token_manager.load_token(&token.session_id).await? {
            Some(t) => t,
            None => {
                info!("No stored token, cannot restore without dialog");
                return Ok(None);
            }
        };

        // Attempt restoration with token
        let portal_config = PortalConfig {
            persist_mode: PersistMode::ExplicitlyRevoked,
            restore_token: Some(restore_token),
            ..Default::default()
        };

        let manager = PortalManager::new(portal_config).await?;

        match manager.try_restore_session(&token.session_id).await {
            Ok((handle, new_token)) => {
                // Save the new token
                if let Some(t) = new_token {
                    self.token_manager.save_token(&token.session_id, &t).await?;
                }
                info!("Session restored successfully without user interaction");
                Ok(Some(handle))
            }
            Err(e) if e.is_permission_required() => {
                warn!("Token invalid, user interaction required: {}", e);
                Ok(None) // Signal that dialog is needed
            }
            Err(e) => Err(e),
        }
    }
}
```

### Strategy 2: Mutter Direct API (GNOME)

```rust
/// MutterDirectStrategy in src/session/strategies/mutter_direct.rs

pub struct MutterDirectStrategy {
    connection: zbus::Connection,
}

impl SessionStrategy for MutterDirectStrategy {
    fn name(&self) -> &'static str { "Mutter Direct D-Bus API" }
    fn requires_initial_setup(&self) -> bool { false } // No portal dialog
    fn supports_unattended_restore(&self) -> bool { true }

    async fn create_session(&self, config: &SessionConfig) -> Result<SessionHandle> {
        // Use org.gnome.Mutter.ScreenCast directly (not portal)
        let screencast_proxy = MutterScreenCastProxy::new(&self.connection).await?;

        // Create session
        let session_path = screencast_proxy.create_session(HashMap::new()).await?;
        let session_proxy = MutterScreenCastSessionProxy::new(
            &self.connection,
            session_path
        ).await?;

        // Record the monitor (or virtual for headless)
        let stream_path = if config.headless {
            session_proxy.record_virtual(
                HashMap::from([("cursor-mode", 2u32.into())]) // Metadata
            ).await?
        } else {
            // Get primary monitor connector
            let connector = get_primary_monitor_connector()?;
            session_proxy.record_monitor(
                &connector,
                HashMap::from([("cursor-mode", 2u32.into())])
            ).await?
        };

        // Start the session
        let stream_proxy = MutterScreenCastStreamProxy::new(
            &self.connection,
            stream_path
        ).await?;

        // Get PipeWire node ID
        let pipewire_node = stream_proxy.pipewire_node_id().await?;

        // For input, use org.gnome.Mutter.RemoteDesktop
        let remote_desktop = MutterRemoteDesktopProxy::new(&self.connection).await?;
        let rd_session = remote_desktop.create_session().await?;
        rd_session.start().await?;

        info!("Mutter session created without portal dialog");

        Ok(SessionHandle {
            pipewire_node,
            session_type: SessionType::MutterDirect,
            // ... other fields
        })
    }

    async fn restore_session(&self, _token: &SessionToken) -> Result<Option<SessionHandle>> {
        // Mutter direct API doesn't use tokens - just create new session
        // This always works without user interaction
        Ok(None) // Signal to use create_session instead
    }
}
```

### Strategy 3: wlr-screencopy (wlroots Bypass)

```rust
/// WlrScreencopyStrategy in src/session/strategies/wlr_screencopy.rs
///
/// NOTE: This strategy requires a separate capture pipeline that doesn't
/// use PipeWire. Implementation deferred to Phase 4 if other strategies
/// prove insufficient.

pub struct WlrScreencopyStrategy {
    // This would use wayland-client to connect directly
}

impl SessionStrategy for WlrScreencopyStrategy {
    fn name(&self) -> &'static str { "wlr-screencopy Direct Capture" }
    fn requires_initial_setup(&self) -> bool { false }
    fn supports_unattended_restore(&self) -> bool { true }

    async fn create_session(&self, config: &SessionConfig) -> Result<SessionHandle> {
        // Direct Wayland protocol usage
        // No portal, no dialog, no PipeWire

        // Connect to Wayland compositor
        let (display, mut queue) = wayland_client::Connection::connect_to_env()?;
        let registry = display.get_registry();

        // Find screencopy manager
        let screencopy_manager = find_global::<ZwlrScreencopyManagerV1>(&registry)?;

        // Get output to capture
        let output = find_primary_output(&registry)?;

        // Create frame capture
        let frame = screencopy_manager.capture_output(
            true, // overlay_cursor
            &output,
        );

        // This produces DMA-BUF frames directly
        // Would need integration with our encoding pipeline

        todo!("Full wlr-screencopy implementation deferred to Phase 4")
    }
}
```

---

## Credential Storage Architecture

### Detection Algorithm

The credential storage detection algorithm now accounts for deployment context:

```rust
/// Credential storage detection in src/session/credentials.rs

pub async fn detect_credential_storage(
    deployment: &DeploymentContext
) -> (CredentialStorageMethod, EncryptionType, bool) {

    // FLATPAK-SPECIFIC DETECTION
    if matches!(deployment, DeploymentContext::Flatpak) {
        return detect_flatpak_credential_storage().await;
    }

    // TPM 2.0 only available with systemd (not in Flatpak or initd)
    match deployment {
        DeploymentContext::SystemdUser { .. } |
        DeploymentContext::SystemdSystem => {
            if let Ok(has_tpm) = check_tpm2_available() {
                if has_tpm {
                    let accessible = check_systemd_creds_accessible().await;
                    return (
                        CredentialStorageMethod::Tpm2,
                        EncryptionType::TpmBound,
                        accessible,
                    );
                }
            }
        }
        _ => {} // TPM via systemd-creds not available
    }

    // Secret Service API (GNOME Keyring, KWallet, KeePassXC)
    // Not directly available in Flatpak (must use portal)
    if !matches!(deployment, DeploymentContext::Flatpak) {
        if let Ok(service) = detect_secret_service().await {
            match service {
                SecretServiceBackend::GnomeKeyring => {
                    let accessible = check_keyring_unlocked().await;
                    return (
                        CredentialStorageMethod::GnomeKeyring,
                        EncryptionType::Aes256Gcm,
                        accessible,
                    );
                }
                SecretServiceBackend::KWallet => {
                    let accessible = check_kwallet_open().await;
                    return (
                        CredentialStorageMethod::KWallet,
                        EncryptionType::Aes256Gcm,
                        accessible,
                    );
                }
                SecretServiceBackend::KeePassXC => {
                    let accessible = check_keepassxc_unlocked().await;
                    return (
                        CredentialStorageMethod::KeePassXC,
                        EncryptionType::ChaCha20Poly1305,
                        accessible,
                    );
                }
            }
        }
    }

    // Encrypted file fallback (always available)
    let key_available = check_encryption_key_available().await;
    (
        CredentialStorageMethod::EncryptedFile,
        EncryptionType::Aes256Gcm,
        key_available,
    )
}

/// Flatpak-specific credential storage detection
async fn detect_flatpak_credential_storage() -> (CredentialStorageMethod, EncryptionType, bool) {
    // Priority 1: Flatpak Secret Portal (recommended)
    if check_flatpak_secret_portal_available().await {
        info!("Flatpak: Using Secret Portal for credential storage");
        return (
            CredentialStorageMethod::FlatpakSecretPortal,
            EncryptionType::HostKeyring, // Encryption handled by host
            true,
        );
    }

    // Priority 2: Encrypted file in app data directory
    // Location: ~/.var/app/org.lamco.RdpServer/data/lamco-rdp-server/
    info!("Flatpak: Using encrypted file storage (Secret Portal unavailable)");
    (
        CredentialStorageMethod::EncryptedFile,
        EncryptionType::Aes256Gcm,
        true, // Always available in Flatpak app data dir
    )
}

async fn check_flatpak_secret_portal_available() -> bool {
    // Check if org.freedesktop.portal.Secret is available
    let connection = match zbus::Connection::session().await {
        Ok(conn) => conn,
        Err(_) => return false,
    };

    let proxy = match zbus::fdo::DBusProxy::new(&connection).await {
        Ok(p) => p,
        Err(_) => return false,
    };

    match proxy.list_names().await {
        Ok(names) => names.iter().any(|n| n == "org.freedesktop.portal.Secret"),
        Err(_) => false,
    }
}

fn check_tpm2_available() -> Result<bool> {
    // Check via systemd-creds has-tpm2 (systemd 250+)
    let output = Command::new("systemd-creds")
        .arg("has-tpm2")
        .output()?;

    Ok(output.status.success() &&
       String::from_utf8_lossy(&output.stdout).trim() == "yes")
}

async fn detect_secret_service() -> Result<SecretServiceBackend> {
    let connection = zbus::Connection::session().await?;

    // Check if Secret Service is available
    let proxy = zbus::fdo::DBusProxy::new(&connection).await?;
    let names = proxy.list_names().await?;

    if !names.iter().any(|n| n == "org.freedesktop.secrets") {
        return Err(anyhow!("Secret Service not available"));
    }

    // Detect which backend is providing it
    if names.iter().any(|n| n.starts_with("org.gnome.keyring")) {
        Ok(SecretServiceBackend::GnomeKeyring)
    } else if names.iter().any(|n| n.starts_with("org.kde.kwalletd")) {
        Ok(SecretServiceBackend::KWallet)
    } else if names.iter().any(|n| n.contains("keepassxc")) {
        Ok(SecretServiceBackend::KeePassXC)
    } else {
        // Generic Secret Service (could be any compliant backend)
        Ok(SecretServiceBackend::GnomeKeyring) // Assume GNOME-like behavior
    }
}
```

### TokenManager Implementation

```rust
/// TokenManager in src/session/token_manager.rs

pub struct TokenManager {
    storage_method: CredentialStorageMethod,
    storage_path: PathBuf,
    secret_service: Option<SecretServiceClient>,
    flatpak_secret_portal: Option<FlatpakSecretPortal>,
    tpm_creds: Option<TpmCredentialStore>,
}

impl TokenManager {
    pub async fn new(method: CredentialStorageMethod) -> Result<Self> {
        // Determine storage path based on deployment
        let storage_path = if Path::new("/.flatpak-info").exists() {
            // Flatpak: Use app data directory
            PathBuf::from(env::var("XDG_DATA_HOME")
                .unwrap_or_else(|_| format!("{}/.local/share", env::var("HOME").unwrap())))
                .join("lamco-rdp-server")
                .join("sessions")
        } else {
            // Native: Use standard data directory
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("lamco-rdp-server")
                .join("sessions")
        };

        fs::create_dir_all(&storage_path)?;

        let secret_service = match method {
            CredentialStorageMethod::GnomeKeyring |
            CredentialStorageMethod::KWallet |
            CredentialStorageMethod::KeePassXC => {
                Some(SecretServiceClient::connect().await?)
            }
            _ => None,
        };

        let flatpak_secret_portal = match method {
            CredentialStorageMethod::FlatpakSecretPortal => {
                Some(FlatpakSecretPortal::new().await?)
            }
            _ => None,
        };

        let tpm_creds = match method {
            CredentialStorageMethod::Tpm2 => {
                Some(TpmCredentialStore::new()?)
            }
            _ => None,
        };

        Ok(Self {
            storage_method: method,
            storage_path,
            secret_service,
            flatpak_secret_portal,
            tpm_creds,
        })
    }

    pub async fn save_token(&self, session_id: &str, token: &str) -> Result<()> {
        let key = format!("lamco-rdp-session-{}", session_id);

        match self.storage_method {
            CredentialStorageMethod::GnomeKeyring |
            CredentialStorageMethod::KWallet |
            CredentialStorageMethod::KeePassXC => {
                // Store via Secret Service API (direct)
                let ss = self.secret_service.as_ref().unwrap();
                ss.store_secret(
                    &key,
                    token,
                    &[("application", "lamco-rdp-server")],
                ).await?;
                info!("Token stored in Secret Service");
            }

            CredentialStorageMethod::FlatpakSecretPortal => {
                // Store via Flatpak Secret Portal
                let portal = self.flatpak_secret_portal.as_ref().unwrap();
                portal.store_secret(
                    &key,
                    token,
                    &[
                        ("application", "org.lamco.RdpServer"),
                        ("session_id", session_id),
                    ],
                ).await?;
                info!("Token stored via Flatpak Secret Portal");
            }

            CredentialStorageMethod::Tpm2 => {
                // Store via systemd-creds (TPM-bound)
                let tpm = self.tpm_creds.as_ref().unwrap();
                tpm.store(&key, token.as_bytes()).await?;
                info!("Token stored in TPM 2.0 bound storage");
            }

            CredentialStorageMethod::EncryptedFile => {
                // Encrypt and store to file
                let encrypted = self.encrypt_token(token)?;
                let path = self.storage_path.join(format!("{}.token", session_id));
                fs::write(&path, &encrypted)?;

                // Restrict permissions (Unix only)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
                }

                info!("Token stored in encrypted file: {:?}", path);
            }

            CredentialStorageMethod::None => {
                warn!("No secure storage available, token not persisted");
                return Err(anyhow!("No credential storage available"));
            }
        }

        // Also save metadata
        self.save_token_metadata(session_id).await?;

        Ok(())
    }

    pub async fn load_token(&self, session_id: &str) -> Result<Option<String>> {
        let key = format!("lamco-rdp-session-{}", session_id);

        match self.storage_method {
            CredentialStorageMethod::GnomeKeyring |
            CredentialStorageMethod::KWallet |
            CredentialStorageMethod::KeePassXC => {
                // Load via Secret Service API (direct)
                let ss = self.secret_service.as_ref().unwrap();
                match ss.lookup_secret(&key).await {
                    Ok(token) => Ok(Some(token)),
                    Err(e) if e.is_not_found() => Ok(None),
                    Err(e) => Err(e.into()),
                }
            }

            CredentialStorageMethod::FlatpakSecretPortal => {
                // Load via Flatpak Secret Portal
                let portal = self.flatpak_secret_portal.as_ref().unwrap();
                match portal.retrieve_secret(&key).await {
                    Ok(token) => Ok(Some(token)),
                    Err(e) if e.is_not_found() => Ok(None),
                    Err(e) => Err(e.into()),
                }
            }

            CredentialStorageMethod::Tpm2 => {
                // Load via systemd-creds (TPM-bound)
                let tpm = self.tpm_creds.as_ref().unwrap();
                match tpm.load(&key).await {
                    Ok(bytes) => Ok(Some(String::from_utf8(bytes)?)),
                    Err(e) if e.is_not_found() => Ok(None),
                    Err(e) => Err(e.into()),
                }
            }

            CredentialStorageMethod::EncryptedFile => {
                // Load from encrypted file
                let path = self.storage_path.join(format!("{}.token", session_id));
                if path.exists() {
                    let encrypted = fs::read(&path)?;
                    let token = self.decrypt_token(&encrypted)?;
                    Ok(Some(token))
                } else {
                    Ok(None)
                }
            }

            CredentialStorageMethod::None => Ok(None),
        }
    }

    fn encrypt_token(&self, token: &str) -> Result<Vec<u8>> {
        // Use machine-bound key derivation
        let key = derive_machine_key()?;

        // AES-256-GCM encryption
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};
        use rand::RngCore;

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, token.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);

        Ok(result)
    }

    fn decrypt_token(&self, data: &[u8]) -> Result<String> {
        if data.len() < 12 {
            return Err(anyhow!("Invalid encrypted data"));
        }

        let key = derive_machine_key()?;

        use aes_gcm::{Aes256Gcm, Key, Nonce};
        use aes_gcm::aead::{Aead, KeyInit};

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;

        Ok(String::from_utf8(plaintext)?)
    }
}

/// Derive a machine-specific encryption key
fn derive_machine_key() -> Result<[u8; 32]> {
    use sha2::{Sha256, Digest};

    let mut hasher = Sha256::new();

    // Machine ID (stable across reboots)
    if let Ok(machine_id) = fs::read_to_string("/etc/machine-id") {
        hasher.update(machine_id.trim().as_bytes());
    }

    // Boot ID (changes on reboot - optional additional binding)
    // Uncomment to invalidate tokens on reboot:
    // if let Ok(boot_id) = fs::read_to_string("/proc/sys/kernel/random/boot_id") {
    //     hasher.update(boot_id.trim().as_bytes());
    // }

    // Application-specific salt
    hasher.update(b"lamco-rdp-server-token-encryption-v1");

    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);

    Ok(key)
}
```

### Flatpak Secret Portal Implementation

For Flatpak deployments, we use the standardized Secret Portal for credential storage:

```rust
/// Flatpak Secret Portal wrapper in src/session/flatpak_secret.rs

use zbus::{Connection, zvariant::{ObjectPath, Value}};
use std::collections::HashMap;

pub struct FlatpakSecretPortal {
    connection: Connection,
}

impl FlatpakSecretPortal {
    pub async fn new() -> Result<Self> {
        let connection = Connection::session().await?;

        // Verify Secret Portal is available
        if !Self::is_available(&connection).await {
            return Err(anyhow!("Flatpak Secret Portal not available"));
        }

        Ok(Self { connection })
    }

    async fn is_available(connection: &Connection) -> bool {
        let proxy = match zbus::fdo::DBusProxy::new(connection).await {
            Ok(p) => p,
            Err(_) => return false,
        };

        match proxy.list_names().await {
            Ok(names) => names.iter().any(|n| n == "org.freedesktop.portal.Secret"),
            Err(_) => false,
        }
    }

    pub async fn store_secret(
        &self,
        key: &str,
        value: &str,
        attributes: &[(&str, &str)],
    ) -> Result<()> {
        // Build D-Bus method call to org.freedesktop.portal.Secret
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Secret",
        ).await?;

        // Create attributes dictionary
        let mut attr_dict: HashMap<String, Value> = HashMap::new();
        for (k, v) in attributes {
            attr_dict.insert(k.to_string(), Value::new(v));
        }
        attr_dict.insert("xdg:schema".to_string(), Value::new("org.lamco.rdpserver.token"));

        // Call RetrieveSecret method (Flatpak Secret Portal API)
        // Note: The actual storage happens via the host's Secret Service
        let options: HashMap<String, Value> = HashMap::new();

        proxy.call_method(
            "RetrieveSecret",
            &(key, attr_dict, options),
        ).await?;

        // Store the actual secret value
        // The portal will prompt once for access if needed
        Ok(())
    }

    pub async fn retrieve_secret(&self, key: &str) -> Result<String> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Secret",
        ).await?;

        // Retrieve secret from portal
        let result: (Vec<u8>,) = proxy.call_method("RetrieveSecret", &(key,)).await?;

        String::from_utf8(result.0)
            .map_err(|e| anyhow!("Invalid UTF-8 in secret: {}", e))
    }
}
```

**How it works:**

1. **Sandboxed Access:** Flatpak app talks to `org.freedesktop.portal.Secret`
2. **Portal Mediation:** Portal forwards request to host's Secret Service
3. **Host Storage:** Actual storage in GNOME Keyring, KWallet, or KeePassXC on host
4. **Encryption:** Handled by host system (AES-256-GCM typically)
5. **Security:** App never directly accesses host keyring, properly sandboxed

**Benefits for Flatpak deployment:**

- ✅ Leverages host system's secure keyring
- ✅ No separate encryption key management needed
- ✅ Proper sandboxing maintained
- ✅ Survives app updates
- ✅ User's existing keyring infrastructure

---

## Permission Dialog Avoidance Matrix

### Comprehensive Capability Matrix

| Environment | Strategy | Initial Setup | Reconnect | Reboot | systemd Service |
|-------------|----------|---------------|-----------|--------|-----------------|
| **GNOME 45+ (Portal v5)** | Portal+Token | Dialog | No Dialog | No Dialog | Works |
| **GNOME 45+ (Mutter API)** | Mutter Direct | No Dialog | No Dialog | No Dialog | Works |
| **GNOME 45+ (Flatpak)** | Portal+Token | Dialog | No Dialog | No Dialog | Works† |
| **KDE Plasma 6 (Portal v4)** | Portal+Token | Dialog | No Dialog | No Dialog | Works |
| **KDE Plasma 6 (Flatpak)** | Portal+Token | Dialog | No Dialog | No Dialog | Works† |
| **KDE Plasma 5 (Portal v3)** | Basic Portal | Dialog | Dialog | Dialog | Hangs |
| **Sway (Portal v4)** | Portal+Token | Dialog | No Dialog | No Dialog | Works |
| **Sway (Flatpak)** | Portal+Token | Dialog | No Dialog | No Dialog | Works† |
| **Sway (wlr-screencopy)** | wlr Direct | No Dialog | No Dialog | No Dialog | Works |
| **Hyprland (Portal v4)** | Portal+Token | Dialog | Varies* | Varies* | Varies* |
| **Hyprland (Flatpak)** | Portal+Token | Dialog | Varies* | Varies* | Works† |
| **Hyprland (wlr-screencopy)** | wlr Direct | No Dialog | No Dialog | No Dialog | Works |
| **Labwc (wlr-screencopy)** | wlr Direct | No Dialog | No Dialog | No Dialog | Works |
| **Cosmic (Portal TBD)** | Portal+Token | Dialog | TBD | TBD | TBD |
| **Any (Portal v3-)** | Basic Portal | Dialog | Dialog | Dialog | Hangs |

*Hyprland portal token support has known bugs (see Issues #123, #350)
†Flatpak requires user service, not system service - see Flatpak systemd integration below

### What Each Strategy Enables

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    STRATEGY CAPABILITY COMPARISON                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ STRATEGY 1: Portal + Restore Token                                  │ │
│  ├────────────────────────────────────────────────────────────────────┤ │
│  │ Requirements:                                                       │ │
│  │   • Portal version >= 4 (for restore token support)                │ │
│  │   • Credential storage (Secret Service, TPM, or encrypted file)    │ │
│  │                                                                     │ │
│  │ Enables:                                                            │ │
│  │   ✅ Session restoration after graceful disconnect                  │ │
│  │   ✅ Session restoration after server restart                       │ │
│  │   ✅ Session restoration after system reboot                        │ │
│  │   ✅ systemd service operation (after initial setup)                │ │
│  │                                                                     │ │
│  │ Does NOT Enable:                                                    │ │
│  │   ❌ First-time setup without dialog                                │ │
│  │   ❌ Restoration if user revoked permission                         │ │
│  │   ❌ Restoration if selected monitor no longer exists               │ │
│  │   ❌ Cross-machine token portability (tokens are machine-bound)     │ │
│  │                                                                     │ │
│  │ Token Invalidation Scenarios:                                       │ │
│  │   • User explicitly revokes in portal settings                      │ │
│  │   • Selected monitor disconnected                                   │ │
│  │   • Compositor/portal upgrade (sometimes)                           │ │
│  │   • Portal backend bug (Hyprland known issues)                      │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ STRATEGY 2: Mutter Direct D-Bus API                                 │ │
│  ├────────────────────────────────────────────────────────────────────┤ │
│  │ Requirements:                                                       │ │
│  │   • GNOME with Mutter compositor                                    │ │
│  │   • org.gnome.Mutter.ScreenCast D-Bus interface accessible          │ │
│  │   • org.gnome.Mutter.RemoteDesktop D-Bus interface accessible       │ │
│  │   • Non-sandboxed application (NOT Flatpak)                         │ │
│  │                                                                     │ │
│  │ Enables:                                                            │ │
│  │   ✅ ZERO dialog requirement (not even first time)                  │ │
│  │   ✅ Immediate session creation                                     │ │
│  │   ✅ Full systemd service operation                                 │ │
│  │   ✅ Virtual monitor creation (headless support)                    │ │
│  │                                                                     │ │
│  │ Does NOT Enable:                                                    │ │
│  │   ❌ Operation on KDE, Sway, or other compositors                   │ │
│  │   ❌ Flatpak deployment (sandboxing blocks D-Bus access)            │ │
│  │   ❌ API stability guarantee (may break between GNOME versions)     │ │
│  │                                                                     │ │
│  │ Risk Factors:                                                       │ │
│  │   • API not officially stable (though used by gnome-remote-desktop) │ │
│  │   • May require specific D-Bus permissions                          │ │
│  │   • GNOME 47+ may have breaking changes                             │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ STRATEGY 3: wlr-screencopy Direct Protocol                          │ │
│  ├────────────────────────────────────────────────────────────────────┤ │
│  │ Requirements:                                                       │ │
│  │   • wlroots-based compositor (Sway, Hyprland, Labwc, etc.)          │ │
│  │   • zwlr_screencopy_manager_v1 protocol available                   │ │
│  │   • zwlr_virtual_keyboard_manager_v1 for input                      │ │
│  │   • zwlr_virtual_pointer_manager_v1 for input                       │ │
│  │   • Privileged Wayland client access                                │ │
│  │                                                                     │ │
│  │ Enables:                                                            │ │
│  │   ✅ ZERO dialog requirement (protocol has no permission model)     │ │
│  │   ✅ Immediate session creation                                     │ │
│  │   ✅ Full systemd service operation                                 │ │
│  │   ✅ Direct DMA-BUF frame access (potentially more efficient)       │ │
│  │                                                                     │ │
│  │ Does NOT Enable:                                                    │ │
│  │   ❌ Operation on GNOME, KDE, or other non-wlroots compositors      │ │
│  │   ❌ PipeWire integration (different capture path)                  │ │
│  │   ❌ Standard portal clipboard integration                          │ │
│  │                                                                     │ │
│  │ Implementation Complexity:                                          │ │
│  │   • Requires separate capture backend (not PipeWire-based)          │ │
│  │   • Need to handle frame timing ourselves                           │ │
│  │   • Input injection via separate protocols                          │ │
│  │   • Clipboard would need wlr-data-control protocol                  │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ STRATEGY 4: SSH-Assisted Initial Grant                              │ │
│  ├────────────────────────────────────────────────────────────────────┤ │
│  │ Requirements:                                                       │ │
│  │   • SSH access to target machine                                    │ │
│  │   • X11 forwarding (-X) or waypipe for Wayland                      │ │
│  │   • User can view forwarded dialog                                  │ │
│  │                                                                     │ │
│  │ Enables:                                                            │ │
│  │   ✅ Initial permission grant on headless machines                  │ │
│  │   ✅ Token acquisition for future unattended operation              │ │
│  │   ✅ Admin setup without physical monitor                           │ │
│  │                                                                     │ │
│  │ Does NOT Enable:                                                    │ │
│  │   ❌ Fully automated deployment (still needs human once)            │ │
│  │   ❌ Operation without any SSH access                               │ │
│  │   ❌ Token renewal if revoked (needs SSH again)                     │ │
│  │                                                                     │ │
│  │ Usage:                                                              │ │
│  │   $ ssh -X user@server lamco-rdp-server --grant-permission          │ │
│  │   (Dialog appears on local X display, user clicks Allow)            │ │
│  │   Token stored, future starts work unattended                       │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### SSH-Like Authorization Key Concept

One of your questions was about SSH-like key-based authorization. Here's how we could conceptualize this:

```
┌─────────────────────────────────────────────────────────────────────────┐
│              SSH-INSPIRED AUTHORIZATION MODEL (CONCEPTUAL)               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  CURRENT SSH MODEL:                                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐│
│  │ Client                           Server                              ││
│  │ ──────                           ──────                              ││
│  │ ~/.ssh/id_rsa (private key) ───> ~/.ssh/authorized_keys             ││
│  │                                  (public key whitelist)              ││
│  │                                                                      ││
│  │ Trust established by:                                                ││
│  │ 1. Admin adds public key to authorized_keys                          ││
│  │ 2. Client proves possession of private key                           ││
│  │ 3. No interactive password needed after setup                        ││
│  └─────────────────────────────────────────────────────────────────────┘│
│                                                                          │
│  PORTAL RESTORE TOKEN (CURRENT):                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐│
│  │ Application                      Portal/Compositor                   ││
│  │ ───────────                      ─────────────────                   ││
│  │ Stored restore_token ──────────> Portal validates token              ││
│  │                                  (bound to app + selection)          ││
│  │                                                                      ││
│  │ Trust established by:                                                ││
│  │ 1. User clicks "Allow" in dialog                                     ││
│  │ 2. Portal issues opaque token                                        ││
│  │ 3. App stores token for future use                                   ││
│  │ 4. Token is single-use (new token issued each session)               ││
│  └─────────────────────────────────────────────────────────────────────┘│
│                                                                          │
│  HYPOTHETICAL KEY-BASED MODEL (WOULD REQUIRE CUSTOM PORTAL):             │
│  ┌─────────────────────────────────────────────────────────────────────┐│
│  │ Application                      Custom Portal Backend               ││
│  │ ───────────                      ─────────────────────               ││
│  │ lamco-rdp-server.key ──────────> /etc/lamco-portal/authorized_apps   ││
│  │ (app identity key)               (admin-maintained whitelist)        ││
│  │                                                                      ││
│  │ Trust established by:                                                ││
│  │ 1. Admin generates key pair for app                                  ││
│  │ 2. Admin adds public key to authorized_apps                          ││
│  │ 3. App proves identity via challenge-response                        ││
│  │ 4. Portal auto-approves without user dialog                          ││
│  │                                                                      ││
│  │ Requires:                                                            ││
│  │ • Custom portal backend (xdg-desktop-portal-lamco)                   ││
│  │ • Admin privilege for initial setup                                  ││
│  │ • Key management infrastructure                                      ││
│  └─────────────────────────────────────────────────────────────────────┘│
│                                                                          │
│  PRACTICAL APPROACH (OUR IMPLEMENTATION):                                │
│  ┌─────────────────────────────────────────────────────────────────────┐│
│  │ Strategy Selection:                                                  ││
│  │ ───────────────────                                                  ││
│  │ 1. Detect compositor type                                            ││
│  │ 2. Check available strategies (Mutter API, wlr-screencopy, Portal)   ││
│  │ 3. Use best strategy that avoids dialog                              ││
│  │ 4. Fall back to Portal + token (dialog on first run only)            ││
│  │ 5. Store credentials securely via detected storage method            ││
│  │                                                                      ││
│  │ This achieves "SSH-like" experience without custom portal:           ││
│  │ • One-time setup (like adding SSH key)                               ││
│  │ • Subsequent access without dialog (like SSH key auth)               ││
│  │ • Secure credential storage (like SSH agent)                         ││
│  └─────────────────────────────────────────────────────────────────────┘│
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Flatpak systemd Integration

Flatpak applications can integrate with systemd user services for background operation:

```xml
<!-- In org.lamco.RdpServer.yaml Flatpak manifest -->

<!-- Export systemd user service -->
<file type="systemd-user-service">org.lamco.RdpServer.service</file>
```

```ini
# files/org.lamco.RdpServer.service (included in Flatpak)

[Unit]
Description=lamco RDP Server (Flatpak)
Documentation=https://lamco.ai/docs/rdp-server

[Service]
Type=notify
ExecStart=/usr/bin/flatpak run --command=lamco-rdp-server org.lamco.RdpServer
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=default.target
```

**User enables the service:**

```bash
# Install Flatpak app
flatpak install flathub org.lamco.RdpServer

# Run once to grant portal permissions (interactive)
flatpak run org.lamco.RdpServer --grant-permission

# Enable systemd user service
systemctl --user enable org.lamco.RdpServer.service
systemctl --user start org.lamco.RdpServer.service

# Enable linger for headless operation
loginctl enable-linger $USER
```

**Token storage location in Flatpak:**

```
~/.var/app/org.lamco.RdpServer/
├── config/
│   └── lamco-rdp-server/
│       └── config.toml
└── data/
    └── lamco-rdp-server/
        └── sessions/
            ├── tokens/
            │   └── default.token      # Encrypted with app-bound key
            └── metadata/
                └── default.json       # Session metadata
```

**Credential storage decision tree for Flatpak:**

```
┌─────────────────────────────────────────────────────────────────┐
│          FLATPAK CREDENTIAL STORAGE DECISION                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Is org.freedesktop.portal.Secret available?                    │
│  ───────────────────────────────────────────                    │
│                                                                  │
│  YES ───> Use FlatpakSecretPortal                               │
│           ├─ Stores in host GNOME Keyring/KWallet               │
│           ├─ Encryption: Host system (AES-256)                   │
│           ├─ Security: Guaranteed                                │
│           └─ Persistence: Across app updates                     │
│                                                                  │
│  NO  ───> Use EncryptedFile                                     │
│           ├─ Stores in ~/.var/app/.../data/                     │
│           ├─ Encryption: AES-256-GCM (app-bound key)            │
│           ├─ Security: BestEffort                                │
│           └─ Persistence: Across app updates                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Deployment Method Decision Matrix

Comprehensive deployment recommendations based on use case:

| Use Case | Primary Method | Fallback | Rationale |
|----------|---------------|----------|-----------|
| **Personal workstation** | Native .deb/.rpm + systemd user | Flatpak | Full strategy access, simple setup |
| **Headless home server** | Native + systemd user + linger | - | Unattended operation, all strategies |
| **Multi-user VDI/terminal** | Native + systemd system (advanced) | Per-user systemd user | Complex but supports multi-user |
| **Maximum compatibility** | Flatpak | Native package | Works across all distros |
| **Security-first deployment** | Flatpak | Native with AppArmor/SELinux | Sandboxing limits attack surface |
| **Development machine** | Native binary (no systemd) | - | Full access, easy debugging |
| **Enterprise Fedora/RHEL** | Native RPM + systemd user + TPM | Flatpak | SELinux policies, TPM credential storage |
| **Enterprise Ubuntu/Debian** | Native DEB + systemd user | Flatpak | AppArmor, Secret Service |
| **Arch/Gentoo (rolling)** | Native package + systemd user | - | Latest portal features available |
| **Gentoo (OpenRC)** | Native + OpenRC script | - | Community-supported, manual D-Bus setup |
| **NixOS** | Nix package + systemd user | - | Declarative configuration |
| **Raspberry Pi OS** | Native DEB + systemd user | - | wlroots (labwc) with wlr-screencopy |

### Flatpak vs Native Feature Comparison

| Feature | Native Package | Flatpak |
|---------|---------------|---------|
| Portal + Token strategy | ✅ | ✅ |
| Mutter Direct API | ✅ | ❌ (sandboxed) |
| wlr-screencopy Direct | ✅ | ❌ (sandboxed) |
| Secret Service (direct) | ✅ | ❌ (sandboxed) |
| Flatpak Secret Portal | ❌ (not needed) | ✅ |
| TPM 2.0 credentials | ✅ | ❌ (no systemd-creds access) |
| Encrypted file storage | ✅ | ✅ |
| systemd user service | ✅ | ✅ (via export) |
| systemd system service | ✅ | ❌ (user-level only) |
| Auto-update | Manual/apt/dnf | ✅ Automatic |
| Distro portability | Limited | ✅ Universal |
| Installation size | Smaller | Larger (runtime included) |
| Security isolation | OS-level | ✅ Stronger (sandbox) |

**Recommendation hierarchy:**

1. **Best overall:** Native systemd user service (full feature access)
2. **Best portability:** Flatpak (works everywhere, portal-only)
3. **Specific needs:**
   - GNOME optimization → Native (enables Mutter API)
   - wlroots optimization → Native (enables wlr-screencopy)
   - Security focus → Flatpak (sandboxing)
   - Multi-user VDI → Native systemd system service (complex)

---

## Implementation Roadmap

### Phase 1: Portal Token Infrastructure (Weeks 1-2)

**Scope:** Implement token capture, storage, and restoration via existing portal path.

#### 1.1 Modify lamco-portal to Return Tokens

**File:** `lamco-wayland/crates/lamco-portal/src/remote_desktop.rs`

```rust
// Change return type to include token
pub async fn start_session(
    &self,
    session: &Session<'_>,
) -> Result<(OwnedFd, Vec<StreamInfo>, Option<String>)> {
    let response = /* ... existing code ... */;

    // Extract restore token from response
    let restore_token = response.restore_token().map(|s| s.to_string());

    Ok((pipewire_fd, streams, restore_token))
}
```

**File:** `lamco-wayland/crates/lamco-portal/src/lib.rs`

```rust
// Update create_session to return token
pub async fn create_session(
    &self,
    session_id: String,
    clipboard: Option<&ClipboardManager>,
) -> Result<(PortalSessionHandle, Option<String>)> {
    // ... existing code ...

    let (pipewire_fd, streams, restore_token) = self
        .remote_desktop
        .start_session(&remote_desktop_session)
        .await?;

    let handle = PortalSessionHandle::new(/* ... */);

    Ok((handle, restore_token))
}
```

#### 1.2 Create TokenManager

**New Files:**
- `wrd-server-specs/src/session/token_manager.rs` - Main TokenManager implementation
- `wrd-server-specs/src/session/flatpak_secret.rs` - Flatpak Secret Portal wrapper
- `wrd-server-specs/src/session/credentials.rs` - Credential storage detection

**Implementation:**
- Support all credential storage backends: Secret Service, Flatpak Secret Portal, TPM 2.0, encrypted file
- Implement deployment-aware storage path detection
- Add token metadata storage
- Implement machine-bound key derivation for encrypted file fallback

#### 1.3 Integrate into WrdServer

**File:** `wrd-server-specs/src/server/mod.rs`

```rust
impl WrdServer {
    pub async fn new(config: Config) -> Result<Self> {
        // ... existing capability probing ...

        // Detect deployment context (Flatpak, systemd, initd, etc.)
        let deployment = detect_deployment_context();
        info!("Deployment context: {:?}", deployment);

        // Detect credential storage (deployment-aware)
        let (storage_method, encryption, accessible) = detect_credential_storage(&deployment).await;
        info!("Credential storage: {:?} (encryption: {:?}, accessible: {})",
              storage_method, encryption, accessible);

        let token_manager = TokenManager::new(storage_method).await?;

        // Try to load existing token
        let restore_token = token_manager.load_token("default").await?;

        // Configure portal with token if available
        let portal_config = config.to_portal_config();
        let portal_config = PortalConfig {
            persist_mode: PersistMode::ExplicitlyRevoked,
            restore_token,
            ..portal_config
        };

        let portal_manager = PortalManager::new(portal_config).await?;

        // Create session (may or may not show dialog)
        let (session_handle, new_token) = portal_manager
            .create_session(session_id, portal_clipboard)
            .await?;

        // Store the new token
        if let Some(token) = new_token {
            token_manager.save_token("default", &token).await?;
            info!("Session token stored for future use");
        }

        // ... rest of initialization ...
    }
}
```

#### 1.4 Add CLI Options

```rust
// New CLI argument
#[derive(Parser)]
struct Args {
    /// Grant permission and exit (for initial setup)
    #[arg(long)]
    grant_permission: bool,

    /// Clear stored tokens
    #[arg(long)]
    clear_tokens: bool,

    /// Show session persistence status
    #[arg(long)]
    persistence_status: bool,
}
```

### Phase 2: Service Registry Extension (Weeks 2-3)

**Scope:** Add new ServiceIds and translation logic for session persistence capabilities.

#### 2.1 Extend ServiceId Enum

**File:** `wrd-server-specs/src/services/service.rs`

Add `SessionPersistence`, `DirectCompositorAPI`, `CredentialStorage`, `UnattendedAccess`, `WlrScreencopy` variants.

#### 2.2 Extend WaylandFeature Enum

**File:** `wrd-server-specs/src/services/wayland_features.rs`

Add corresponding feature variants with detailed capability information.

#### 2.3 Implement Translation Functions

**File:** `wrd-server-specs/src/services/translation.rs`

Implement `translate_session_persistence`, `translate_direct_compositor_api`, `translate_credential_storage`, `translate_wlr_screencopy`, `translate_unattended_access`.

#### 2.4 Update CompositorCapabilities

**File:** `wrd-server-specs/src/compositor/capabilities.rs`

```rust
pub struct CompositorCapabilities {
    // ... existing fields ...

    /// Deployment context (affects available strategies)
    pub deployment: DeploymentContext,

    /// D-Bus session access available
    pub has_session_dbus: bool,

    /// Can access system Secret Service directly
    pub has_secret_service_access: bool,
}
```

#### 2.5 Update Portal Capability Probing

**File:** `wrd-server-specs/src/compositor/portal_caps.rs`

```rust
pub struct PortalCapabilities {
    // ... existing fields ...

    /// Portal version supports restore tokens (v4+)
    pub supports_restore_token: bool,

    /// Maximum persist mode available
    pub max_persist_mode: u8,
}

impl PortalCapabilities {
    async fn probe_persistence_support(&mut self, connection: &Connection) {
        // Check portal version
        if self.version >= 4 {
            self.supports_restore_token = true;
            self.max_persist_mode = 2;
        } else {
            self.supports_restore_token = false;
            self.max_persist_mode = 0;
        }
    }
}
```

#### 2.6 Integrate Deployment Detection into Capability Probing

**File:** `wrd-server-specs/src/compositor/probing.rs`

```rust
pub async fn probe_capabilities() -> Result<CompositorCapabilities> {
    // ... existing probing ...

    // NEW: Detect deployment context
    let deployment = detect_deployment_context();

    // NEW: Check D-Bus session availability
    let has_session_dbus = check_session_dbus_available().await;

    // NEW: Check Secret Service accessibility
    let has_secret_service_access = match deployment {
        DeploymentContext::Flatpak => false, // Must use portal
        _ => check_secret_service_accessible().await,
    };

    // Build capabilities with deployment context
    let mut caps = CompositorCapabilities::new(compositor, portal, wayland_globals);
    caps.deployment = deployment;
    caps.has_session_dbus = has_session_dbus;
    caps.has_secret_service_access = has_secret_service_access;

    Ok(caps)
}
```

### Phase 3: Mutter Direct API (GNOME) (Weeks 3-4)

**Scope:** Implement alternative capture path for GNOME that bypasses portal dialogs entirely.

**⚠️ Deployment Constraint:** This phase is **NOT AVAILABLE for Flatpak deployments** due to sandboxing restrictions. Only applicable for native packages and systemd user services.

#### 3.1 Create Mutter D-Bus Wrappers

**New Files:**
- `wrd-server-specs/src/mutter/mod.rs`
- `wrd-server-specs/src/mutter/screencast.rs`
- `wrd-server-specs/src/mutter/remote_desktop.rs`

```rust
// Mutter ScreenCast D-Bus proxy
#[dbus_proxy(
    interface = "org.gnome.Mutter.ScreenCast",
    default_service = "org.gnome.Mutter.ScreenCast",
    default_path = "/org/gnome/Mutter/ScreenCast"
)]
trait MutterScreenCast {
    fn create_session(&self, properties: HashMap<String, Value>) -> Result<OwnedObjectPath>;
}

#[dbus_proxy(interface = "org.gnome.Mutter.ScreenCast.Session")]
trait MutterScreenCastSession {
    fn record_monitor(&self, connector: &str, properties: HashMap<String, Value>)
        -> Result<OwnedObjectPath>;
    fn record_virtual(&self, properties: HashMap<String, Value>)
        -> Result<OwnedObjectPath>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
}

#[dbus_proxy(interface = "org.gnome.Mutter.ScreenCast.Stream")]
trait MutterScreenCastStream {
    #[dbus_proxy(property)]
    fn pipewire_node_id(&self) -> Result<u32>;
}
```

#### 3.2 Create MutterSessionManager

**New File:** `wrd-server-specs/src/session/strategies/mutter_direct.rs`

Implement `MutterDirectStrategy` as shown in Multi-Strategy Session Management section.

#### 3.3 Integrate with Strategy Selector

Update `SessionStrategySelector` to prefer Mutter API when available on GNOME.

### Phase 4: wlr-screencopy Fallback (Weeks 4-6)

**Scope:** Document and optionally implement direct wlr-screencopy capture for wlroots compositors.

**⚠️ Deployment Constraint:** This phase is **NOT AVAILABLE for Flatpak deployments** due to Wayland socket access restrictions. Only applicable for native packages.

**Priority:** DEFERRED - Implement only if Phase 1 (Portal + Token) proves insufficient on Hyprland/Sway.

#### 4.1 Evaluate Necessity

After Phase 1-3 completion, evaluate if portal token support is sufficient for Sway/Hyprland users. If Hyprland token bugs persist, proceed with wlr-screencopy.

**Decision criteria:**
- Does Hyprland portal restore token work reliably in production?
- Is the one-time dialog on first run acceptable to users?
- What percentage of users run wlroots compositors?
- Is avoiding portal complexity worth maintaining a second capture backend?

#### 4.2 Design Capture Backend Abstraction

```rust
/// Capture backend abstraction
pub trait CaptureBackend: Send + Sync {
    /// Backend name for logging
    fn name(&self) -> &'static str;

    /// Does this backend require PipeWire?
    fn uses_pipewire(&self) -> bool;

    /// Start capture, returns frame receiver
    async fn start_capture(&self, output: &OutputInfo) -> Result<FrameReceiver>;

    /// Stop capture
    async fn stop_capture(&self) -> Result<()>;
}

/// Frame receiver abstraction
pub enum FrameReceiver {
    /// PipeWire-based (existing path)
    PipeWire(PipeWireReceiver),
    /// Direct DMA-BUF frames (wlr-screencopy)
    DirectDmaBuf(WlrFrameReceiver),
}
```

#### 4.3 Implement wlr-screencopy Backend

If needed, implement using `wayland-client` crate to connect to wlr-screencopy protocol directly.

---

### Implementation Phase Priority by Deployment

Understanding which phases are **mandatory** vs **optional** based on deployment target:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                 PHASE PRIORITY MATRIX BY DEPLOYMENT                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Deployment      │ Phase 1       │ Phase 2       │ Phase 3  │ Phase 4  │
│  ───────────────┼───────────────┼───────────────┼──────────┼──────────┤
│  Flatpak        │ ✅ MANDATORY   │ ✅ MANDATORY   │ ❌ N/A    │ ❌ N/A    │
│  Native + GNOME │ ✅ MANDATORY   │ ✅ MANDATORY   │ 🔶 OPTIONAL│ ❌ N/A    │
│  Native + KDE   │ ✅ MANDATORY   │ ✅ MANDATORY   │ ❌ N/A    │ ❌ N/A    │
│  Native + Sway  │ ✅ MANDATORY   │ ✅ MANDATORY   │ ❌ N/A    │ 🔶 OPTIONAL│
│  Native + Hypr  │ ✅ MANDATORY   │ ✅ MANDATORY   │ ❌ N/A    │ ⚠️  MAYBE  │
│  systemd user   │ ✅ MANDATORY   │ ✅ MANDATORY   │ 🔶 OPTIONAL│ 🔶 OPTIONAL│
│  systemd system │ ✅ MANDATORY   │ ✅ MANDATORY   │ ❌ N/A    │ ❌ N/A    │
│  initd/OpenRC   │ ✅ MANDATORY   │ ✅ MANDATORY   │ ⚠️  COMPLEX│ ⚠️  COMPLEX│
│                                                                          │
│  Legend:                                                                 │
│  ✅ MANDATORY = Required for basic unattended operation                  │
│  🔶 OPTIONAL = Optimization, avoids one-time dialog                      │
│  ⚠️  MAYBE = Depends on real-world testing                               │
│  ❌ N/A = Not applicable/blocked by deployment constraints               │
└─────────────────────────────────────────────────────────────────────────┘
```

### Key Implementation Insights

1. **Phase 1 + 2 are universal requirements**
   - Portal + Token strategy works in ALL deployment scenarios
   - Service Registry integration enables runtime adaptation
   - These phases deliver 90% of the value

2. **Phase 3 (Mutter API) is a GNOME-specific optimization**
   - Eliminates the one-time dialog
   - Only valuable for native packages on GNOME
   - Adds maintenance burden for marginal benefit
   - **Consider deferring** unless GNOME is primary target audience

3. **Phase 4 (wlr-screencopy) is a wlroots-specific fallback**
   - Only needed if Hyprland portal bugs are showstoppers
   - Requires separate capture pipeline (significant work)
   - wlr-screencopy avoids portal entirely (zero dialogs)
   - **Defer until proven necessary** by user feedback

### Minimum Viable Implementation

For maximum deployment compatibility with minimum effort:

```
PHASE 1 + PHASE 2 ONLY
─────────────────────

Delivers:
✅ Flatpak deployment support
✅ Native package support
✅ systemd user service support
✅ Portal restore token on all compositors
✅ Deployment-aware credential storage
✅ One-time permission grant
✅ Service Registry visibility

Skips:
❌ Zero-dialog GNOME (Mutter API)
❌ Zero-dialog wlroots (wlr-screencopy)

Result:
• Works everywhere with one manual grant
• Flatpak distribution ready
• All major distros supported
• Simpler codebase, less maintenance
```

**Recommendation:** Ship Phase 1 + 2, gather user feedback on whether Phase 3/4 optimizations are worth the complexity.

---

## Configuration Schema

### Extended Configuration

```toml
# config.toml additions

[session]
# Session persistence strategy: "auto", "portal_token", "mutter_direct", "wlr_screencopy"
# "auto" selects best available strategy
strategy = "auto"

# Credential storage method: "auto", "secret_service", "tpm2", "encrypted_file"
# "auto" selects most secure available method
credential_storage = "auto"

# Path for encrypted file storage (if using encrypted_file method)
credential_path = "~/.local/share/lamco-rdp-server/sessions"

# Portal persistence mode: "none", "transient", "permanent"
# "permanent" (mode 2) recommended for unattended operation
persist_mode = "permanent"

[session.recovery]
# Automatically attempt token restoration on startup
auto_restore = true

# Maximum token age before requiring re-grant (0 = never expire)
max_token_age_days = 0

# Clear tokens if monitor configuration changes
invalidate_on_monitor_change = false

[session.mutter]
# Enable Mutter direct API on GNOME (bypasses portal)
# WARNING: API not officially stable
enable_mutter_api = true

# Minimum GNOME version for Mutter API
min_gnome_version = "45.0"

[session.wlr]
# Enable wlr-screencopy fallback on wlroots compositors
# Requires separate capture backend implementation
enable_wlr_screencopy = false
```

---

## Failure Modes & Robustness Strategy

### Design Principle: Graceful Degradation

**No single detection failure is fatal.** The system uses a **defense-in-depth** approach where each component has conservative fallbacks:

```
Detection Success ───> Optimal Configuration (Guaranteed services)
        │
        └─ Detection Failure ───> Safe Mode (BestEffort/Degraded services)
                                    ↓
                              Still functional
```

### Critical Fallback Hierarchy

| Detection Failure | Fallback | Impact |
|-------------------|----------|--------|
| **Compositor unknown** | `CompositorType::Unknown` → Portal + MemFd + conservative quirks | ⚠️ Reduced performance, still works |
| **Portal probing fails** | `PortalCapabilities::default()` (all false) | ❌ **FATAL** (no capture) unless wlr-screencopy available |
| **Portal version unknown** | Assume v1 (no tokens) | ⚠️ Manual dialog each session |
| **Wayland globals fail** | Empty vector | ⚠️ Protocol-specific features disabled |
| **Secret Service unavailable** | Encrypted file storage | ⚠️ Weaker security (machine-bound key) |
| **TPM unavailable** | Secret Service or encrypted file | ⚠️ No TPM-bound encryption |
| **machine-id missing** | Hostname or static salt | ⚠️ Weak key derivation |
| **Deployment context unknown** | Assume `Native` | ⚠️ May attempt unavailable strategies |
| **Service not in registry** | Return `ServiceLevel::Unavailable` | ⚠️ Feature-specific degradation |

### Unknown Compositor Safe Mode

When compositor cannot be identified, the system uses **maximum compatibility defaults** (`profiles.rs:317-336`):

```rust
CompositorProfile {
    recommended_capture: CaptureBackend::Portal,     // Always available
    recommended_buffer_type: BufferType::MemFd,      // Most compatible
    supports_damage_hints: false,                    // Assume no
    supports_explicit_sync: false,                   // Assume no
    quirks: [PoorDmaBufSupport, NeedsExplicitCursorComposite],
    recommended_fps_cap: 30,                         // Conservative
    portal_timeout_ms: 60000,                        // Extra time
}
```

**Services in Unknown mode:**

| ServiceId | Level | Reason |
|-----------|-------|--------|
| VideoCapture | Guaranteed | Portal provides |
| RemoteInput | Guaranteed | Portal provides |
| DamageTracking | BestEffort | Frame diff fallback |
| MetadataCursor | Varies | Depends on portal |
| DmaBufZeroCopy | Unavailable | Quirk blocks it |
| DirectCompositorAPI | Unavailable | Unknown compositor |
| WlrScreencopy | Unavailable | Unknown compositor |
| SessionPersistence | Varies | Depends on portal version |

### Portal Failure → Fatal Error

Portal is **required** for most deployments. If portal probing completely fails:

```rust
// In WrdServer::new()
if !service_registry.has_service(ServiceId::VideoCapture) {
    // Check if wlr-screencopy is available (wlroots only)
    if service_registry.has_service(ServiceId::WlrScreencopy) {
        warn!("Portal unavailable, using wlr-screencopy fallback");
        // Use WlrScreencopyStrategy (Phase 4)
    } else {
        return Err(anyhow!(
            "FATAL: No screen capture capability detected.\n\
             \n\
             This could be caused by:\n\
             1. xdg-desktop-portal not running\n\
             2. No portal backend installed for your compositor\n\
             3. Not running in Wayland session\n\
             \n\
             To fix, install the appropriate portal backend:\n\
             • GNOME: xdg-desktop-portal-gnome\n\
             • KDE Plasma: xdg-desktop-portal-kde\n\
             • Sway/wlroots: xdg-desktop-portal-wlr\n\
             • Hyprland: xdg-desktop-portal-hyprland\n\
             \n\
             Then restart your session or run:\n\
             systemctl --user restart xdg-desktop-portal.service"
        ));
    }
}
```

### Credential Storage Failure → Encrypted File

Credential storage **never fails completely**. Worst case is encrypted file with weak key:

```rust
fn derive_machine_key() -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();

    // Try machine-id
    if let Ok(machine_id) = fs::read_to_string("/etc/machine-id") {
        hasher.update(machine_id.trim().as_bytes());
    } else if let Ok(machine_id) = fs::read_to_string("/var/lib/dbus/machine-id") {
        hasher.update(machine_id.trim().as_bytes());
    } else if let Ok(hostname) = hostname::get() {
        warn!("No machine-id, using hostname for key derivation");
        hasher.update(hostname.to_string_lossy().as_bytes());
    } else {
        warn!("No machine-id or hostname - using static salt (WEAK SECURITY)");
        hasher.update(b"lamco-fallback-key");
    }

    hasher.update(b"lamco-rdp-server-token-encryption-v1");

    Ok(hasher.finalize().into())
}
```

**Security degradation:**

| Available | Key Source | Security |
|-----------|------------|----------|
| machine-id | Unique per machine | Strong |
| hostname | Network hostname | Medium |
| Static | Same for all instances | Weak |

**Even in worst case:** Tokens are encrypted (just not uniquely per machine).

### Session Persistence Failure → Basic Portal

If all persistence mechanisms fail:

```rust
// Portal v3 or below
SessionPersistence: Unavailable
  └─> Falls back to BasicPortalStrategy
      ├─ Dialog appears on every server start
      ├─ Session works after grant
      └─ No token storage
```

**User experience:**
- Server starts
- Dialog appears: "Allow lamco-rdp-server to share your screen?"
- User clicks "Allow"
- Session works normally
- **BUT:** Dialog repeats on next restart

**This is acceptable for:**
- Interactive workstations (user present anyway)
- Development/testing environments
- Rare-restart scenarios

---

## Diagnostic CLI Commands

To help debug detection failures:

```bash
# Full capability report
lamco-rdp-server --show-capabilities

# Test each component individually
lamco-rdp-server --test-compositor-detection
lamco-rdp-server --test-portal-connection
lamco-rdp-server --test-credential-storage
lamco-rdp-server --test-deployment-detection

# Safe mode (skip detection, use defaults)
lamco-rdp-server --safe-mode

# Verbose failure diagnostics
lamco-rdp-server --diagnose
```

**Example diagnostic output:**

```
🔍 Diagnosing lamco-rdp-server environment...

[✅] Wayland session detected
[✅] D-Bus session bus accessible
[⚠️ ] Compositor identification: Unknown
     └─ Fallback: Generic portal support active
[✅] Portal connection successful
[✅] ScreenCast portal v5 available
[✅] RemoteDesktop portal v2 available
[✅] Restore token support: Available
[⚠️ ] Secret Service: Not detected
     └─ Fallback: Using encrypted file storage
[✅] machine-id available for key derivation
[✅] Session persistence: Possible via portal tokens

SUMMARY: System is operational
         Persistence: ✅ (Portal v5 tokens)
         Performance: ⚠️  (Generic compositor settings)

RECOMMENDATIONS:
  • Set XDG_CURRENT_DESKTOP to identify compositor
  • Install libsecret for improved credential security
```

---

## Summary: Failure Never Cascades

### Isolation Principle

Each service detection is **isolated**:

```rust
// services/translation.rs
pub fn translate_capabilities(caps: &CompositorCapabilities) -> Vec<AdvertisedService> {
    let mut services = Vec::new();

    // Each translation is independent
    services.push(translate_damage_tracking(caps));  // Can't affect others
    services.push(translate_dmabuf(caps));           // Can't affect others
    services.push(translate_metadata_cursor(caps));  // Can't affect others
    // ... etc

    services  // Always returns full list (some may be Unavailable)
}
```

**Key guarantee:** `translate_capabilities()` **always returns 11 services**, even if all are Unavailable.

### Answer to Your Question

**"What if service discovery fails or returns unfamiliar results?"**

```
┌─────────────────────────────────────────────────────────────────┐
│                  FAILURE → FALLBACK → OUTCOME                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Compositor unknown                                              │
│  ───────────────────                                             │
│  → CompositorType::Unknown                                       │
│  → unknown_profile() with safe defaults                          │
│  → Portal-only, MemFd buffers, conservative quirks               │
│  → ✅ System works, just not optimized                           │
│                                                                  │
│  Portal unavailable                                              │
│  ──────────────────                                              │
│  → VideoCapture: Unavailable                                     │
│  → Check for wlr-screencopy (wlroots only)                       │
│  → If none: ❌ FATAL ERROR with clear user message               │
│  → ✅ Error explains how to fix (install portal backend)         │
│                                                                  │
│  Portal version unknown                                          │
│  ──────────────────────                                          │
│  → Assume version 1 (no tokens)                                  │
│  → SessionPersistence: Degraded                                  │
│  → ✅ System works, dialog required each time                    │
│                                                                  │
│  Credential storage unavailable                                  │
│  ──────────────────────────────                                  │
│  → Try: TPM → Secret Service → Encrypted File → Static Key      │
│  → ✅ Encrypted file ALWAYS works (worst case: weak static key) │
│                                                                  │
│  Service returns unfamiliar data                                 │
│  ────────────────────────────                                    │
│  → Service marked as Unavailable                                 │
│  → Code checks level before using                                │
│  → Falls back to alternative (e.g., painted cursor not metadata)│
│  → ✅ System continues without that feature                      │
│                                                                  │
│  ALL detection fails                                             │
│  ──────────────────                                              │
│  → Can run with --safe-mode flag                                 │
│  → Uses hardcoded safe defaults                                  │
│  → ✅ Basic RDP functionality guaranteed                         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Bottom line:** The system **always** has a fallback path, and the only truly fatal error is "no screen capture capability whatsoever" (which means portal AND wlr-screencopy both unavailable).

---

*For detailed failure mode analysis, see: FAILURE-MODES-AND-FALLBACKS.md*

---

## Security Considerations

### Token Security

| Concern | Mitigation |
|---------|------------|
| Token theft | Encrypted storage, machine-bound keys |
| Token replay | Single-use tokens, compositor validates |
| Cross-machine attack | Machine ID bound encryption |
| Memory exposure | Zeroize tokens after use |
| Privilege escalation | Tokens only grant screen capture, not system access |

### Credential Storage Security

| Method | Security Level | Unlock Requirement |
|--------|---------------|-------------------|
| TPM 2.0 | Highest | Machine boot + optional PIN |
| Secret Service (locked) | High | User login or explicit unlock |
| Secret Service (auto-unlock) | Medium | User login |
| Encrypted File | Medium | Machine ID (auto) or password |
| Plain File | None | Not implemented |

### Mutter API Security

| Concern | Mitigation |
|---------|------------|
| Any app can use API | Application must run as user, not sandboxed |
| No permission dialog | User chooses to run lamco-rdp-server |
| API stability | Version checks, fallback to portal |

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_storage_detection() {
        // Mock D-Bus responses to test detection logic
    }

    #[test]
    fn test_token_encryption_roundtrip() {
        let token = "test-token-12345";
        let manager = TokenManager::new(CredentialStorageMethod::EncryptedFile)?;
        manager.save_token("test", token)?;
        let loaded = manager.load_token("test")?;
        assert_eq!(loaded, Some(token.to_string()));
    }

    #[test]
    fn test_strategy_selection() {
        // Test strategy selection for different capability combinations
    }
}
```

### Integration Tests

```bash
# Test token persistence across restart
./lamco-rdp-server --grant-permission
./lamco-rdp-server --persistence-status  # Should show "Token available"
systemctl restart lamco-rdp-server       # Should start without dialog
```

### Compositor-Specific Tests

| Compositor | Test Scenario | Expected Result |
|------------|--------------|-----------------|
| GNOME 46 | Token persistence | No dialog on restart |
| GNOME 46 | Mutter direct | No dialog ever |
| KDE Plasma 6 | Token persistence | No dialog on restart |
| Sway | Token persistence | No dialog on restart |
| Hyprland | Token persistence | Document any bugs |

---

## References

### XDG Desktop Portal

- [ScreenCast Portal Documentation](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.ScreenCast.html)
- [RemoteDesktop Portal Documentation](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html)
- [OBS Studio Restore Token PR](https://github.com/obsproject/obs-studio/pull/5559)
- [ashpd Rust Crate](https://docs.rs/ashpd/latest/ashpd/)

### Compositor-Specific

- [GNOME Mutter RemoteDesktop Wiki](https://wiki.gnome.org/Projects/Mutter/RemoteDesktop)
- [gnome-remote-desktop Source](https://github.com/GNOME/gnome-remote-desktop)
- [wlr-screencopy Protocol](https://wayland.app/protocols/wlr-screencopy-unstable-v1)
- [Hyprland Portal Issues](https://github.com/hyprwm/xdg-desktop-portal-hyprland/issues)

### Credential Storage

- [Secret Service API](https://specifications.freedesktop.org/secret-service/latest/)
- [GNOME Keyring](https://wiki.archlinux.org/title/GNOME/Keyring)
- [KDE Wallet](https://wiki.archlinux.org/title/KDE_Wallet)
- [systemd-creds](https://smallstep.com/blog/systemd-creds-hardware-protected-secrets/)
- [TPM 2.0 Linux](https://linbit.com/blog/securing-the-linstor-encryption-passphrase-by-using-systemd-creds-and-tpm-2-0/)

### systemd Integration

- [User Services](https://wiki.archlinux.org/title/Systemd/User)
- [loginctl enable-linger](https://www.freedesktop.org/software/systemd/man/loginctl.html)

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.2.0 | 2025-12-31 | lamco-rdp-server team | Added comprehensive failure modes & robustness strategy section, diagnostic CLI commands, safe mode documentation, credential storage fallback chain with weak-key scenarios |
| 1.1.0 | 2025-12-31 | lamco-rdp-server team | Added deployment constraints section (Flatpak, systemd, initd), Flatpak Secret Portal support, deployment-aware credential storage, phase priority matrix |
| 1.0.0 | 2025-12-31 | lamco-rdp-server team | Initial architecture document |

---

*End of Document*
