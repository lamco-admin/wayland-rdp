# Failure Modes & Fallback Strategy

**Document Version:** 1.0.0
**Date:** 2025-12-31
**Related:** SESSION-PERSISTENCE-ARCHITECTURE.md
**Classification:** Robustness & Reliability Design

---

## Executive Summary

This document defines the **defensive programming strategy** for service discovery and session persistence when detection fails or returns unexpected results. The core principle is **graceful degradation**: the system should work with reduced functionality rather than failing completely.

---

## Design Principles

### 1. Fail-Safe Defaults

Every detection step has a **conservative fallback** that ensures the system remains operational:

```
Detection Success ───> Optimal Configuration
        │
        └─ Detection Failure ───> Conservative Safe Mode
                                    (still functional)
```

### 2. Partial Degradation

Individual service failures **do not cascade**. Each service is independent:

```
VideoCapture: Guaranteed    ✅
MetadataCursor: Unavailable ❌  ← Cursor falls back to painted mode
DamageTracking: BestEffort  🔶
ClipBoard: Guaranteed       ✅
```

System continues with 3/4 services working.

### 3. Explicit Communication

When fallbacks activate, the system **logs clearly** what failed and what fallback is being used.

### 4. Runtime Recovery

If environment changes (compositor upgrade, portal installation), the system can **re-detect on restart** without code changes.

---

## Failure Scenarios & Fallbacks

### Scenario 1: Compositor Detection Fails

**When this happens:**
- `XDG_CURRENT_DESKTOP` not set
- `DESKTOP_SESSION` not set
- No compositor-specific env vars
- Process detection fails

**Existing fallback** (`probing.rs:182`):

```rust
CompositorType::Unknown {
    session_info: Some(session_env) // Capture any available info
}
```

**What Unknown compositor gets** (`profiles.rs:317-336`):

```rust
CompositorProfile {
    compositor: CompositorType::Unknown,
    recommended_capture: CaptureBackend::Portal,  // ← SAFEST
    recommended_buffer_type: BufferType::MemFd,   // ← MOST COMPATIBLE
    supports_damage_hints: false,                 // ← ASSUME NO
    supports_explicit_sync: false,                // ← ASSUME NO
    quirks: vec![
        Quirk::PoorDmaBufSupport,                 // ← DON'T TRUST DMA-BUF
        Quirk::NeedsExplicitCursorComposite,
    ],
    recommended_fps_cap: 30,                      // ← CONSERVATIVE
    portal_timeout_ms: 60000,                     // ← EXTRA TIME
}
```

**Result:**
- ✅ System still works via Portal
- ❌ No compositor-specific optimizations (Mutter API, wlr-screencopy)
- ❌ DMA-BUF zero-copy disabled
- ⚠️ Frame differencing for damage tracking

**Service Registry impact:**

| ServiceId | Level | Note |
|-----------|-------|------|
| VideoCapture | Guaranteed | Portal always provides this |
| RemoteInput | Guaranteed | Portal always provides this |
| DamageTracking | BestEffort | Falls back to frame diff |
| DmaBufZeroCopy | Unavailable | Quirk blocks it |
| MetadataCursor | Varies | Depends on portal caps |
| DirectCompositorAPI | Unavailable | Unknown compositor |
| WlrScreencopy | Unavailable | Unknown compositor |

---

### Scenario 2: Portal Probing Fails

**When this happens:**
- D-Bus session bus not available
- xdg-desktop-portal not running
- Portal D-Bus call times out

**Existing fallback** (`probing.rs:44-45`):

```rust
let portal = match PortalCapabilities::probe().await {
    Ok(caps) => caps,
    Err(e) => {
        warn!("Failed to probe Portal capabilities: {}", e);
        PortalCapabilities::default()  // ← FALLBACK
    }
};
```

**What PortalCapabilities::default() returns** (`portal_caps.rs:81-91`):

```rust
PortalCapabilities {
    version: 0,                         // ← NO PORTAL
    supports_screencast: false,
    supports_remote_desktop: false,
    supports_clipboard: false,
    available_cursor_modes: vec![],
    available_source_types: vec![],
    backend: None,
}
```

**Result:**
- ❌ System **CANNOT START** - no way to capture screen
- This is a **fatal error** for portal-dependent deployments (Flatpak, KDE, etc.)

**Service Registry impact:**

| ServiceId | Level | Consequence |
|-----------|-------|-------------|
| VideoCapture | Unavailable | **FATAL** - cannot stream |
| RemoteInput | Unavailable | **FATAL** - cannot control |
| Clipboard | Unavailable | Non-critical |
| All others | Unavailable | Degraded experience |

**Mitigation required:**

```rust
// In WrdServer::new() after capability probing
let portal_usable = service_registry.service_level(ServiceId::VideoCapture)
    >= ServiceLevel::Degraded;

if !portal_usable {
    // Try alternative capture methods if available
    if service_registry.has_service(ServiceId::WlrScreencopy) {
        warn!("Portal unavailable, attempting wlr-screencopy fallback");
        // Use wlr-screencopy strategy
    } else {
        return Err(anyhow!(
            "Portal not available and no alternative capture backend found.\n\
             Please install xdg-desktop-portal and a backend for your compositor:\n\
             - GNOME: xdg-desktop-portal-gnome\n\
             - KDE: xdg-desktop-portal-kde\n\
             - Sway/wlroots: xdg-desktop-portal-wlr"
        ));
    }
}
```

---

### Scenario 3: Portal Version Unknown/Old

**When this happens:**
- Portal installed but version property missing
- Portal version < 4 (no restore tokens)

**Fallback:**

```rust
// Portal v0 = version detection failed
if portal.version == 0 {
    warn!("Portal version could not be determined, assuming v1 (no tokens)");
}

// Portal v1-3 = no restore token support
if portal.version < 4 {
    warn!("Portal v{} does not support restore tokens", portal.version);
    warn!("Permission dialog will appear on every server start");
}
```

**Service Registry impact:**

```rust
SessionPersistence: Degraded  // or Unavailable if v0
  ↳ "Portal version < 4, no restore token support"
```

**Result:**
- ✅ System works but requires manual dialog each time
- ❌ No unattended operation possible
- ⚠️ Flatpak still viable, just with dialog

---

### Scenario 4: Credential Storage Detection Fails

**When this happens:**
- No Secret Service on D-Bus
- systemd-creds not available
- File encryption key derivation fails

**Fallback chain:**

```rust
pub async fn detect_credential_storage(
    deployment: &DeploymentContext
) -> (CredentialStorageMethod, EncryptionType, bool) {
    // Try TPM → Try Secret Service → Try Encrypted File → None

    // If ALL detection fails:
    (
        CredentialStorageMethod::EncryptedFile,  // ← ALWAYS AVAILABLE
        EncryptionType::Aes256Gcm,
        true,  // Can always create encrypted files
    )
}
```

**Machine key derivation fallback:**

```rust
fn derive_machine_key() -> Result<[u8; 32]> {
    let mut hasher = Sha256::new();

    // Try /etc/machine-id
    if let Ok(machine_id) = fs::read_to_string("/etc/machine-id") {
        hasher.update(machine_id.trim().as_bytes());
    } else if let Ok(machine_id) = fs::read_to_string("/var/lib/dbus/machine-id") {
        // Fallback location
        hasher.update(machine_id.trim().as_bytes());
    } else {
        // WORST CASE: No machine-id available
        warn!("No machine-id found, using hostname for key derivation");
        if let Ok(hostname) = hostname::get() {
            hasher.update(hostname.to_string_lossy().as_bytes());
        } else {
            // ABSOLUTE WORST CASE: Static salt only
            warn!("No machine-id or hostname, using static key (WEAK SECURITY)");
            hasher.update(b"lamco-rdp-server-static-fallback");
        }
    }

    // Application-specific salt (always available)
    hasher.update(b"lamco-rdp-server-token-encryption-v1");

    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);

    Ok(key)
}
```

**Security degradation:**

| Scenario | Key Source | Security Level |
|----------|-----------|----------------|
| Normal | machine-id + salt | Strong (machine-bound) |
| No machine-id | hostname + salt | Medium (hostname predictable) |
| No hostname | salt only | Weak (static key) |

**Critical:** Even in worst case, tokens are still encrypted. Just not machine-bound.

---

### Scenario 5: Deployment Context Detection Fails

**When this happens:**
- Not in Flatpak (no `/.flatpak-info`)
- Not systemd (no `/run/systemd/system`)
- Not OpenRC (no `/run/openrc`)
- Unknown init system

**Fallback:**

```rust
pub fn detect_deployment_context() -> DeploymentContext {
    // ... all detection attempts ...

    // FINAL FALLBACK
    info!("Could not determine deployment context, assuming Native");
    DeploymentContext::Native
}
```

**Why Native is safe:**
- Assumes **most permissive** environment
- All strategies will be attempted
- If they fail, they fail gracefully (strategies have their own error handling)

**Result:**
- ✅ Attempts all strategies (may fail at strategy level)
- ⚠️ May log warnings if strategies fail (e.g., can't access Mutter API)
- ✅ Will fall back to portal if advanced strategies unavailable

---

### Scenario 6: Wayland Global Enumeration Fails

**When this happens:**
- Not connected to Wayland display
- wayland-client connection fails
- Environment variables missing

**Existing fallback** (`probing.rs:50`):

```rust
let wayland_globals = enumerate_wayland_globals().unwrap_or_default();
```

**Result:**
- `wayland_globals` = empty vector
- Protocol checks return `false`
- Service translation marks protocol-dependent services as Unavailable

**Service Registry impact:**

```rust
// Example: Fractional scaling depends on protocol
if caps.has_fractional_scale() {  // ← Returns false if no globals
    // Guaranteed
} else {
    AdvertisedService::unavailable(ServiceId::FractionalScaling)
}
```

**System behavior:**
- ✅ Still works via Portal (doesn't need Wayland globals)
- ❌ No protocol-specific optimizations (fractional scale, etc.)

---

### Scenario 7: Service Registry Returns Unexpected Data

**When this happens:**
- New compositor added but translation not updated
- Portal returns unexpected version number
- D-Bus returns malformed data

**Defensive pattern in translation** (`translation.rs`):

```rust
fn translate_damage_tracking(caps: &CompositorCapabilities) -> AdvertisedService {
    // Always returns an AdvertisedService, never panics
    // Worst case: returns Unavailable
}
```

**Service lookup safety** (`registry.rs:71`):

```rust
pub fn service_level(&self, id: ServiceId) -> ServiceLevel {
    self.services
        .get(&id)
        .map(|s| s.level)
        .unwrap_or(ServiceLevel::Unavailable)  // ← SAFE DEFAULT
}
```

**Result:**
- ✅ Unknown services → Unavailable
- ✅ System queries service, gets Unavailable, uses fallback code path
- ✅ No crashes or panics

---

## Fallback Chain by Component

### Compositor Identification

```
┌─────────────────────────────────────────────────────────────────┐
│                  COMPOSITOR DETECTION FALLBACK                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. XDG_CURRENT_DESKTOP ──> Success: GNOME/KDE/Sway/etc.        │
│        │                                                         │
│        └─ FAIL                                                   │
│           │                                                      │
│  2. DESKTOP_SESSION ───────> Success: GNOME/KDE/etc.            │
│        │                                                         │
│        └─ FAIL                                                   │
│           │                                                      │
│  3. Compositor env vars ───> Success: Sway/Hyprland             │
│     (SWAYSOCK, etc.)                                             │
│        │                                                         │
│        └─ FAIL                                                   │
│           │                                                      │
│  4. Process detection ─────> Success: gnome-shell/kwin/sway     │
│     (pgrep)                                                      │
│        │                                                         │
│        └─ FAIL                                                   │
│           │                                                      │
│  5. FALLBACK: Unknown ─────> CompositorType::Unknown            │
│                              ├─ Portal-only capture              │
│                              ├─ MemFd buffers                    │
│                              ├─ Conservative quirks              │
│                              └─ 30 FPS cap                       │
│                                                                  │
│  RESULT: System works with generic Portal support               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Portal Capability Probing

```
┌─────────────────────────────────────────────────────────────────┐
│                  PORTAL PROBING FALLBACK                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. D-Bus session connection                                     │
│        │                                                         │
│        ├─ SUCCESS ──> Query portal properties                    │
│        │                                                         │
│        └─ FAIL ─────> PortalCapabilities::default()             │
│                       ├─ version: 0                              │
│                       ├─ supports_screencast: false              │
│                       ├─ supports_remote_desktop: false          │
│                       └─ All services: Unavailable               │
│                                                                  │
│  2. Query ScreenCast portal                                      │
│        │                                                         │
│        ├─ SUCCESS ──> Get version, source types, cursor modes    │
│        │                                                         │
│        └─ FAIL ─────> supports_screencast: false                 │
│                       (but continue with other probes)           │
│                                                                  │
│  3. Query RemoteDesktop portal                                   │
│        │                                                         │
│        ├─ SUCCESS ──> Get version, device types                  │
│        │                                                         │
│        └─ FAIL ─────> supports_remote_desktop: false             │
│                       (but continue)                             │
│                                                                  │
│  RESULT: Partial portal support possible                         │
│          (e.g., screen capture but no input injection)           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Critical check in WrdServer::new():**

```rust
// After service registry creation
if !service_registry.has_service(ServiceId::VideoCapture) {
    return Err(anyhow!(
        "No screen capture capability detected.\n\
         \n\
         Possible causes:\n\
         1. xdg-desktop-portal not running\n\
         2. No portal backend installed\n\
         3. Not in Wayland session\n\
         \n\
         Install the portal backend for your compositor:\n\
         - GNOME: sudo apt install xdg-desktop-portal-gnome\n\
         - KDE: sudo apt install xdg-desktop-portal-kde\n\
         - Sway: sudo apt install xdg-desktop-portal-wlr\n\
         \n\
         Then restart your session or run:\n\
         systemctl --user restart xdg-desktop-portal.service"
    ));
}
```

---

### Session Persistence Detection

```
┌─────────────────────────────────────────────────────────────────┐
│            SESSION PERSISTENCE FALLBACK CHAIN                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Portal v4+ detected?                                            │
│  ─────────────────────                                           │
│        │                                                         │
│        ├─ YES ──> SessionPersistence: Guaranteed/BestEffort      │
│        │          (depends on credential storage)                │
│        │                                                         │
│        └─ NO                                                     │
│           │                                                      │
│           ├─ Portal v1-3 ──> SessionPersistence: Degraded        │
│           │                  "No restore token support"          │
│           │                  User must grant permission each time│
│           │                                                      │
│           └─ Portal v0 (unknown) ──> SessionPersistence: Degraded│
│                                      "Could not determine portal │
│                                       version, assuming < v4"    │
│                                                                  │
│  RESULT: System always has a session strategy                    │
│          (worst case: BasicPortalStrategy with dialog each time) │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Credential Storage Detection

```
┌─────────────────────────────────────────────────────────────────┐
│            CREDENTIAL STORAGE FALLBACK CHAIN                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Deployment: Flatpak?                                            │
│  ───────────────────                                             │
│        │                                                         │
│        └─ YES ──> Flatpak Secret Portal available?              │
│                   │                                              │
│                   ├─ YES ──> FlatpakSecretPortal                 │
│                   │                                              │
│                   └─ NO ───> EncryptedFile                       │
│                              (app data dir)                      │
│                                                                  │
│  Deployment: Native/systemd?                                     │
│  ──────────────────────────                                      │
│        │                                                         │
│        ├─ systemd-creds has-tpm2?                                │
│        │   │                                                     │
│        │   ├─ YES ──> Tpm2                                       │
│        │   │                                                     │
│        │   └─ NO ───> Secret Service available?                  │
│        │              │                                          │
│        │              ├─ YES ──> GnomeKeyring/KWallet/KeePassXC  │
│        │              │                                          │
│        │              └─ NO ───> EncryptedFile                   │
│        │                         │                               │
│        │                         └─ machine-id available?        │
│        │                            │                            │
│        │                            ├─ YES ──> Machine-bound key │
│        │                            │                            │
│        │                            └─ NO ───> hostname?         │
│        │                                       │                 │
│        │                                       ├─ YES ──> Host key│
│        │                                       │                 │
│        │                                       └─ NO ──> Static  │
│        │                                                (WEAK)    │
│                                                                  │
│  FINAL FALLBACK: EncryptedFile with static salt                  │
│  (Weak security but system still functional)                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**None is never returned** - EncryptedFile is always available as last resort.

---

### Scenario 5: Unknown Service Levels

**When this happens:**
- New `ServiceId` added but translation not implemented
- HashMap lookup returns None

**Existing safety** (`registry.rs:71`):

```rust
pub fn service_level(&self, id: ServiceId) -> ServiceLevel {
    self.services
        .get(&id)
        .map(|s| s.level)
        .unwrap_or(ServiceLevel::Unavailable)  // ← SAFE DEFAULT
}
```

**Usage in code:**

```rust
// Consumer code always checks level before using
if service_registry.service_level(ServiceId::NewFeature) >= ServiceLevel::BestEffort {
    // Use feature
} else {
    // Graceful degradation path (always exists)
}
```

**Result:**
- ✅ Unknown services treated as Unavailable
- ✅ Code continues with fallback path
- ✅ No panics or crashes

---

## Safe Mode Configuration

If all detection fails catastrophically, the system can operate in **Safe Mode**:

```rust
/// Safe mode configuration (guaranteed to work on any compositor with portal)
pub fn safe_mode_capabilities() -> CompositorCapabilities {
    CompositorCapabilities {
        compositor: CompositorType::Unknown { session_info: None },
        portal: PortalCapabilities {
            version: 1,  // Assume basic portal
            supports_screencast: true,
            supports_remote_desktop: true,
            supports_clipboard: false,
            available_cursor_modes: vec![CursorMode::Embedded],
            available_source_types: vec![SourceType::Monitor],
            backend: None,
        },
        wayland_globals: vec![],
        profile: CompositorProfile {
            recommended_capture: CaptureBackend::Portal,
            recommended_buffer_type: BufferType::MemFd,
            supports_damage_hints: false,
            supports_explicit_sync: false,
            quirks: vec![Quirk::PoorDmaBufSupport],
            recommended_fps_cap: 30,
            portal_timeout_ms: 60000,
        },
        deployment: DeploymentContext::Native,  // Assume most permissive
        has_session_dbus: true,
        has_secret_service_access: false,
    }
}
```

**What Safe Mode provides:**

| Feature | Status |
|---------|--------|
| Screen capture | ✅ Works (Portal + MemFd) |
| Input injection | ✅ Works (Portal) |
| Video encoding | ✅ Works (software H.264) |
| Clipboard | ❌ Disabled |
| Multi-monitor | ⚠️ Limited |
| DMA-BUF zero-copy | ❌ Disabled |
| Hardware encoding | ⚠️ May work (attempted) |
| Session persistence | ❌ Dialog every time |

**User experience in Safe Mode:**
- Dialog on every server start
- Software encoding only (slower)
- Memory copy path (no zero-copy)
- Painted cursor mode
- Works but not optimal

---

## Error Communication Strategy

### Log Levels for Failures

```rust
// FATAL: System cannot start
error!("No screen capture backend available - cannot start");

// WARNING: Degraded functionality
warn!("Portal version < 4, tokens not supported - manual dialog required");
warn!("DMA-BUF support unreliable, using MemFd buffers");

// INFO: Fallback activated
info!("Compositor not identified, using generic Portal support");
info!("Secret Service unavailable, using encrypted file storage");

// DEBUG: Minor degradation
debug!("Fractional scaling not available");
debug!("HDR color space not supported");
```

### User-Facing Messages

```rust
// On startup with degraded capabilities
╔════════════════════════════════════════════════════════╗
║              Service Advertisement Registry             ║
╚════════════════════════════════════════════════════════╝
  Compositor: Unknown
  Services: 3 guaranteed, 2 best-effort, 2 degraded, 4 unavailable

  ⚠️  DEGRADED MODE ACTIVE

  Working:
  ✅ Video Capture        [Guaranteed] → Portal MemFd
  ✅ Remote Input         [Guaranteed] → Portal libei
  ✅ Damage Tracking      [BestEffort] → Frame diff

  Limited:
  ⚠️  Metadata Cursor     [Degraded]   → Painted cursor fallback
  ⚠️  Session Persistence [Degraded]   → Manual dialog required

  Unavailable:
  ❌ DMA-BUF Zero-Copy    [Unavailable] → Compositor unknown
  ❌ Multi-Monitor        [Unavailable] → Portal lacks sources
  ❌ DirectCompositorAPI  [Unavailable] → Unknown compositor
  ❌ HDR Color Space      [Unavailable] → Not implemented

  The server will work but with reduced performance.
  For optimal operation, install compositor-specific tools.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## Minimum Required Services

The **absolute minimum** for basic operation:

| ServiceId | Required Level | Fallback if Unavailable |
|-----------|---------------|-------------------------|
| VideoCapture | Degraded | **FATAL ERROR** - cannot start |
| RemoteInput | Degraded | **FATAL ERROR** - cannot control |
| DamageTracking | Degraded | Use full frame updates (high bandwidth) |
| Clipboard | N/A | Disable clipboard feature |
| All others | N/A | Reduced functionality, not fatal |

**Fatal error check:**

```rust
// In WrdServer::new() after service registry creation
let video_level = service_registry.service_level(ServiceId::VideoCapture);
let input_level = service_registry.service_level(ServiceId::RemoteInput);

if video_level == ServiceLevel::Unavailable {
    return Err(anyhow!("FATAL: No screen capture capability available"));
}

if input_level == ServiceLevel::Unavailable {
    return Err(anyhow!("FATAL: No input injection capability available"));
}

// Everything else is optional/degradable
```

---

## Session Strategy Selection Fallback

When strategy selection encounters failures:

```rust
impl SessionStrategySelector {
    pub fn select_strategy(&self) -> Box<dyn SessionStrategy> {
        // Attempt strategies in priority order

        // All advanced strategies check service levels
        // If level < required, they're skipped

        // FINAL FALLBACK (always works if portal available)
        warn!("No advanced session strategy available");
        warn!("Using basic portal - manual dialog required each session");
        Box::new(BasicPortalStrategy::new())
    }
}
```

**BasicPortalStrategy characteristics:**

- No token support
- No persistence
- Dialog every time
- But **ALWAYS WORKS** if portal is running

---

## Flatpak-Specific Failure Handling

### Flatpak Secret Portal Unavailable

```rust
async fn detect_flatpak_credential_storage() -> (CredentialStorageMethod, EncryptionType, bool) {
    if check_flatpak_secret_portal_available().await {
        (CredentialStorageMethod::FlatpakSecretPortal, EncryptionType::HostKeyring, true)
    } else {
        // FLATPAK FALLBACK: Always have app data dir
        warn!("Flatpak Secret Portal not available, using encrypted file");
        info!("Tokens will be stored in: ~/.var/app/org.lamco.RdpServer/data/");
        (CredentialStorageMethod::EncryptedFile, EncryptionType::Aes256Gcm, true)
    }
}
```

**Key insight:** In Flatpak, we **always** have write access to app data directory. Encrypted file storage is the guaranteed fallback.

---

## Testing Fallback Paths

### Simulated Failure Testing

```rust
#[cfg(test)]
mod fallback_tests {
    use super::*;

    #[test]
    fn test_unknown_compositor_fallback() {
        let compositor = CompositorType::Unknown { session_info: None };
        let profile = CompositorProfile::for_compositor(&compositor);

        // Unknown compositor should use safe defaults
        assert_eq!(profile.recommended_capture, CaptureBackend::Portal);
        assert_eq!(profile.recommended_buffer_type, BufferType::MemFd);
        assert!(!profile.supports_damage_hints);
    }

    #[test]
    fn test_portal_failure_fallback() {
        let caps = PortalCapabilities::default();

        // Default caps should indicate no support
        assert!(!caps.supports_screencast);
        assert!(!caps.supports_remote_desktop);
        assert_eq!(caps.version, 0);
    }

    #[test]
    fn test_service_not_in_registry() {
        let registry = ServiceRegistry::from_compositor(/* ... */);

        // Querying unknown service should return Unavailable
        // Not panic or crash
        let level = registry.service_level(ServiceId::VideoCapture);
        assert_eq!(level, ServiceLevel::Unavailable);
    }

    #[test]
    fn test_credential_storage_total_failure() {
        // Simulate no keyring, no TPM, no machine-id
        let (method, encryption, accessible) = detect_credential_storage_worst_case();

        // Should still return EncryptedFile
        assert_eq!(method, CredentialStorageMethod::EncryptedFile);
        assert!(accessible); // File storage always accessible
    }
}
```

---

## Deployment-Specific Fallback Matrix

| Failure Type | Native | Flatpak | systemd user | systemd system | initd |
|--------------|--------|---------|--------------|----------------|-------|
| Compositor unknown | Unknown profile | Unknown profile | Unknown profile | Unknown profile | Unknown profile |
| Portal v0 | Try wlr-screencopy† | **FATAL** | Try wlr-screencopy† | **FATAL** | Try wlr-screencopy† |
| No Secret Service | TPM or File | Secret Portal or File | TPM or File | File only | File only |
| No TPM | Secret Service or File | N/A | Secret Service or File | File only | File only |
| No machine-id | Hostname key | App-bound key | Hostname key | Hostname key | Hostname key |
| D-Bus failure | **FATAL**‡ | **FATAL** | **FATAL** | **FATAL** | **FATAL** |

†wlr-screencopy only available if compositor is wlroots-based
‡Could be recoverable with direct Wayland protocol on wlroots

---

## Recommended CLI for Diagnostics

To help users understand what failed:

```bash
# Show detected capabilities
lamco-rdp-server --show-capabilities

# Output:
╔════════════════════════════════════════════════════════╗
║            Capability Detection Report                  ║
╚════════════════════════════════════════════════════════╝

Compositor: Unknown (could not identify)
  └─ Fallback: Generic Portal support

Portal: version 5
  ├─ ScreenCast: ✅ Available
  ├─ RemoteDesktop: ✅ Available
  ├─ Clipboard: ✅ Available (v2+)
  └─ Restore tokens: ✅ Supported

Deployment: Native
  └─ Full strategy access

Credential Storage: GNOME Keyring
  ├─ Encryption: AES-256-GCM via libsecret
  └─ Status: ✅ Unlocked

Session Persistence: Guaranteed
  └─ Strategy: Portal + Token

Warnings:
  ⚠️  Compositor not identified (using generic support)
  ⚠️  DMA-BUF zero-copy disabled (unknown compositor)

Recommendation:
  System is fully functional with Portal.
  For optimal performance, ensure compositor is identified.
```

---

## Architecture Document Addition

I should add this as a new section to SESSION-PERSISTENCE-ARCHITECTURE.md. Let me do that now.
