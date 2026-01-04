# lamco-rdp-server Codebase Architecture Audit

**Date:** 2026-01-04
**Auditor:** Deep codebase analysis (systematic review)
**Scope:** Service Registry, Strategy Selection, Mutter/Portal implementations
**Code Size:** 33,961 lines of Rust (src/ only)
**Test Status:** 296/296 passing (100%)
**Purpose:** Pre-publication audit for code quality, architectural soundness, and production readiness

---

## Executive Summary

### Overall Assessment: **STRONG ARCHITECTURE with CRITICAL BUGS**

**Strengths:**
- ✅ Excellent abstraction layers (SessionStrategy trait, SessionHandle trait)
- ✅ Service Registry design is sophisticated and well-implemented
- ✅ Graceful fallback mechanisms designed correctly
- ✅ Clean separation of concerns
- ✅ Comprehensive testing (296 tests, all passing)
- ✅ Well-documented architectural decisions

**Critical Issues Found:**
- 🔴 **CRITICAL BUG:** Variable scope error in portal_token.rs (portal_manager used after out of scope)
- 🔴 **ARCHITECTURAL CONCERN:** Duplicate Portal session creation in hybrid mode
- 🔴 **UNTESTED PATH:** Mutter strategy completely untested on target platforms (GNOME 40-45)
- 🟡 **GNOME 46:** Mutter explicitly broken and disabled (documented)
- 🟡 **ERROR HANDLING:** Persistence rejection handling works but has scope bug

**Production Readiness:**
- ✅ **GNOME 46 + Portal:** Production-ready (tested, working)
- ⏳ **GNOME 40-45 + Mutter:** Architecture ready, but UNTESTED (critical gap)
- ⏳ **KDE/Sway + Portal:** Architecture ready, but UNTESTED
- 🔴 **Critical Bug Must Fix:** portal_manager scope issue before any publication

---

## Architecture Overview

### Three-Layer Design

```
┌──────────────────────────────────────────────────────────────┐
│ Layer 1: Capability Detection & Service Translation         │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  probe_capabilities() → CompositorCapabilities              │
│         ↓                                                    │
│  translate_capabilities() → Vec<AdvertisedService>          │
│         ↓                                                    │
│  ServiceRegistry (query layer)                              │
│                                                              │
│  Services:                                                   │
│  • VideoCapture, DamageTracking, MetadataCursor            │
│  • SessionPersistence, DirectCompositorAPI                  │
│  • CredentialStorage, UnattendedAccess                      │
│                                                              │
│  Each service has:                                           │
│  • ServiceLevel (Guaranteed/BestEffort/Degraded/Unavailable)│
│  • WaylandFeature (what provides it)                        │
│  • RdpCapability (what it maps to)                          │
│  • PerformanceHints (optimization guidance)                 │
│                                                              │
└──────────────────────────────────────────────────────────────┘
                            ↓
┌──────────────────────────────────────────────────────────────┐
│ Layer 2: Strategy Selection & Implementation                │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  SessionStrategySelector::select_strategy()                 │
│         ↓                                                    │
│  Decision tree:                                              │
│  1. Check deployment constraints (Flatpak → Portal only)    │
│  2. Check DirectCompositorAPI service level                 │
│  3. If BestEffort AND is_available() → Mutter              │
│  4. Check SessionPersistence service level                  │
│  5. If BestEffort → Portal + Token                         │
│  6. Fallback → Portal without tokens                        │
│         ↓                                                    │
│  Returns: Box<dyn SessionStrategy>                          │
│                                                              │
│  Implementations:                                            │
│  • MutterDirectStrategy                                     │
│    └─> MutterSessionHandleImpl (implements SessionHandle)  │
│  • PortalTokenStrategy                                      │
│    └─> PortalSessionHandleImpl (implements SessionHandle)  │
│                                                              │
└──────────────────────────────────────────────────────────────┘
                            ↓
┌──────────────────────────────────────────────────────────────┐
│ Layer 3: Server Integration & Hybrid Modes                  │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  WrdServer::new()                                            │
│  1. Probes capabilities → builds ServiceRegistry            │
│  2. Creates SessionStrategySelector                          │
│  3. Calls select_strategy() → gets Box<dyn SessionStrategy> │
│  4. Calls strategy.create_session() → gets Arc<SessionHandle>│
│  5. Extracts PipeWireAccess (FD or NodeID)                  │
│  6. HYBRID MODE: Creates additional Portal session if needed │
│  7. Creates WrdDisplayHandler (video)                        │
│  8. Creates WrdInputHandler (input)                          │
│  9. Creates ClipboardManager (clipboard)                     │
│                                                              │
│  SessionHandle provides:                                     │
│  • pipewire_access() → FD or NodeID                         │
│  • notify_keyboard_keycode()                                │
│  • notify_pointer_*()                                       │
│  • portal_clipboard() → Option<ClipboardComponents>        │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

**Architectural Strength:** Clean abstractions at each layer, zero coupling between layers.

---

## Critical Bugs Found

### BUG #1: Variable Scope Error in Portal Retry Path 🔴 CRITICAL

**Location:** `src/session/strategies/portal_token.rs:286`

**The Bug:**
```rust
// Line 180: portal_manager created
let portal_manager = Arc::new(
    PortalManager::new(portal_config).await?
);

// Lines 191-234: Match with retry path
let (portal_handle, new_token, pre_created_clipboard_mgr) = match portal_manager
    .create_session(session_id.clone(), None)
    .await
{
    Ok(result) => (result.0, result.1, None),
    Err(e) => {
        // RETRY PATH: Creates NEW manager
        let no_persist_manager = Arc::new(
            PortalManager::new(no_persist_config).await?  // ← Different manager!
        );

        let result = no_persist_manager
            .create_session(session_id.clone(), Some(clipboard_mgr.as_ref()))
            .await?;

        (result.0, result.1, Some(clipboard_mgr))
    }
};

// ... later at line 286:
let handle = PortalSessionHandleImpl {
    // ...
    remote_desktop: portal_manager.remote_desktop().clone(),  // ← BUG!
    // ...
};
```

**Problem:**
- When retry path is taken, the actual manager is `no_persist_manager`
- But line 286 uses `portal_manager`
- This means we're cloning remote_desktop from the WRONG manager
- The manager that was used (`no_persist_manager`) is dropped
- The remote_desktop instance from the WRONG manager is stored

**Impact:**
- **Severity:** CRITICAL - Wrong object used for input injection
- **Trigger:** When portal rejects persistence (happens on some portals)
- **Result:** Input injection will likely fail or use wrong session
- **Observed:** May explain the "crash-service-registry.log" error

**Root Cause:**
Variable lifetime/scope issue - different managers in different branches

**Fix Required:**
```rust
// Track which manager was actually successful
let (portal_handle, new_token, pre_created_clipboard_mgr, active_manager) = match portal_manager
    .create_session(session_id.clone(), None)
    .await
{
    Ok(result) => (result.0, result.1, None, portal_manager.clone()),
    Err(e) => {
        if error_msg.contains("cannot persist") || error_msg.contains("InvalidArgument") {
            // ...
            let no_persist_manager = Arc::new(
                PortalManager::new(no_persist_config).await?
            );

            let result = no_persist_manager
                .create_session(session_id.clone(), Some(clipboard_mgr.as_ref()))
                .await?;

            (result.0, result.1, Some(clipboard_mgr), no_persist_manager)
        } else {
            return Err(e).context("Failed to create portal session");
        }
    }
};

// ... later:
let handle = PortalSessionHandleImpl {
    // ...
    remote_desktop: active_manager.remote_desktop().clone(),  // ← Use correct manager
    // ...
};
```

**Testing Gap:**
- ⚠️ Bug not caught by tests (retry path not exercised in test suite)
- Need integration test that triggers persistence rejection
- Need to verify fix works end-to-end

---

### BUG #2: Duplicate Portal Session in Hybrid Mode 🟡 MEDIUM

**Location:** `src/server/mod.rs:304-358`

**The Issue:**
When Mutter strategy is used, the hybrid code creates a SECOND Portal session for input+clipboard:

```rust
// Line 242-254: Strategy creates session (Portal OR Mutter)
let session_handle = strategy
    .create_session()
    .await?;

// Lines 304-312: ALWAYS creates another Portal manager
let portal_manager = Arc::new(
    PortalManager::new(portal_config).await?  // ← Second manager!
);

// Lines 316-358: If Mutter, creates THIRD Portal session
if let Some(clipboard) = session_handle.portal_clipboard() {
    // Portal strategy: Uses existing session ✅
} else {
    // Mutter strategy: Creates NEW session ❌
    let (portal_handle, _) = portal_manager
        .create_session(session_id, None)  // ← Second session!
        .await?;
}
```

**Problem:**
- Portal strategy: 1 Portal session (correct) ✅
- Mutter strategy: 2 Portal sessions (one from never-used manager on line 308, one created on line 335)

**Impact:**
- **Severity:** MEDIUM - Wasteful but functional
- **Resource leak:** Extra D-Bus objects, portal sessions
- **User experience:** Shows 2 permission dialogs instead of 1
- **Confusion:** Logs show "Portal session created" twice

**Root Cause:**
The code creates a PortalManager (lines 308-312) for "input+clipboard" but then:
- Portal strategy: Doesn't use it (returns existing clipboard)
- Mutter strategy: Creates ANOTHER session from it

The initial PortalManager on line 308 appears to be dead code.

**Fix Required:**
```rust
// Option 1: Only create PortalManager if Mutter strategy
let (portal_clipboard_manager, portal_clipboard_session, portal_input_handle) = if let Some(clipboard) = session_handle.portal_clipboard() {
    // Portal strategy: Use existing session
    (/* ... existing code ... */)
} else {
    // Mutter strategy: Create portal manager AND session
    let mut portal_config = config.to_portal_config();
    portal_config.persist_mode = ashpd::desktop::PersistMode::DoNot;
    portal_config.restore_token = None;

    let portal_manager = Arc::new(
        PortalManager::new(portal_config).await?
    );

    let session_id = format!("lamco-rdp-input-clipboard-{}", uuid::Uuid::new_v4());
    let (portal_handle, _) = portal_manager
        .create_session(session_id, None)
        .await?;

    // ... create components ...
};

// Option 2: Remove lines 308-312 entirely (dead code)
```

**Testing Gap:**
- Mutter strategy path needs integration test
- Verify only ONE Portal session created in hybrid mode
- Verify no duplicate dialogs

---

## Architectural Analysis

### Service Registry System (EXCELLENT ✅)

**Design:** `src/services/`
- `service.rs` - Core types (ServiceId, ServiceLevel, AdvertisedService)
- `translation.rs` - Capability → Service translation logic (733 lines)
- `registry.rs` - Query layer (395 lines)
- `wayland_features.rs` - Wayland capability types
- `rdp_capabilities.rs` - RDP capability mapping

**Code Size:** ~2,800 lines

**Strengths:**
1. **Clear separation of concerns:**
   - Detection (compositor module)
   - Translation (services/translation.rs)
   - Query (services/registry.rs)
   - Application (server/mod.rs, strategies)

2. **ServiceLevel enum is brilliant:**
   ```rust
   pub enum ServiceLevel {
       Unavailable = 0,  // Not available
       Degraded = 1,     // Works but has issues
       BestEffort = 2,   // Works but may have limitations
       Guaranteed = 3,   // Fully supported and tested
   }
   ```
   - Implements `Ord` for comparison
   - Clear semantics for decision-making
   - Allows graceful degradation

3. **Translation functions are comprehensive:**
   - Each service has dedicated translation function
   - Version-aware logic (GNOME 46 vs 40-45)
   - Quirk-aware (handles compositor-specific bugs)
   - Evidence-based (actual testing results inform levels)

4. **Version Detection Intelligence:**
   ```rust
   // GNOME 46+: Known broken - session linkage incomplete
   Some(v) if v >= 46.0 => {
       AdvertisedService::unavailable(ServiceId::DirectCompositorAPI)
           .with_note("Mutter API incomplete on GNOME 46+ (session linkage broken)")
   }

   // GNOME 40-45: Should work (critical for RHEL 9, Ubuntu 22.04 LTS)
   Some(v) if v >= 40.0 => {
       AdvertisedService::best_effort(ServiceId::DirectCompositorAPI, feature)
           .with_note("Mutter D-Bus API (critical for Portal v3 systems)")
   }
   ```
   This is evidence-based engineering - version 46 was tested, found broken, and explicitly disabled.

**Weaknesses:**
1. **Synchronous D-Bus check in async context:**
   ```rust
   // translation.rs:647-668
   fn check_dbus_interface_sync(interface: &str) -> bool {
       std::thread::scope(|s| {
           let handle = s.spawn(move || {
               let rt = tokio::runtime::Runtime::new().ok()?;  // ← Creates new runtime!
               // ...
           });
       })
   }
   ```

   **Issue:** Creates new Tokio runtime to avoid nesting issues
   **Impact:** Overhead during capability probing (~20-50ms)
   **Justification:** Necessary evil to avoid "cannot start runtime from within runtime"
   **Rating:** Acceptable workaround, properly documented

2. **parse_gnome_version() is fragile:**
   ```rust
   fn parse_gnome_version(version_str: &str) -> Option<f32> {
       version_str.split('.').take(2).collect::<Vec<_>>().join(".").parse::<f32>().ok()
   }
   ```

   **Issue:** "46.0.1" → 46.0 ✅, "46.10" → 46.1 ❌ (should be 46.10)
   **Impact:** Version 46.10+ would be parsed as 46.1 (still >= 46.0, so works correctly)
   **Rating:** Works but fragile

**Overall Rating: 9/10** - Excellent design with minor implementation issues

---

### Strategy Selection System (VERY GOOD ✅)

**Design:** `src/session/strategies/`
- `selector.rs` - Strategy selection logic (335 lines)
- `mutter_direct.rs` - Mutter implementation (254 lines)
- `portal_token.rs` - Portal implementation (315 lines)

**Code Size:** ~900 lines

**Strengths:**

1. **Clean trait abstraction:**
   ```rust
   #[async_trait]
   pub trait SessionStrategy: Send + Sync {
       fn name(&self) -> &'static str;
       fn requires_initial_setup(&self) -> bool;
       fn supports_unattended_restore(&self) -> bool;
       async fn create_session(&self) -> Result<Arc<dyn SessionHandle>>;
       async fn cleanup(&self, session: &dyn SessionHandle) -> Result<()>;
   }
   ```
   - Minimal surface area
   - Clear semantics
   - Easy to add new strategies (wlr-screencopy)

2. **SessionHandle trait unifies different backends:**
   ```rust
   pub enum PipeWireAccess {
       FileDescriptor(RawFd),  // Portal provides FD
       NodeId(u32),            // Mutter provides node ID
   }

   pub trait SessionHandle: Send + Sync {
       fn pipewire_access(&self) -> PipeWireAccess;
       fn streams(&self) -> Vec<StreamInfo>;
       fn session_type(&self) -> SessionType;
       // Input methods
       async fn notify_keyboard_keycode(&self, keycode: i32, pressed: bool) -> Result<()>;
       async fn notify_pointer_motion_absolute(&self, stream_id: u32, x: f64, y: f64) -> Result<()>;
       // ...
       fn portal_clipboard(&self) -> Option<ClipboardComponents>;
   }
   ```
   - Abstracts FD vs NodeID difference
   - Unified input injection interface
   - Clipboard availability as Option<> (Mutter doesn't have it)

3. **Deployment-aware selection:**
   ```rust
   match caps.deployment {
       DeploymentContext::Flatpak => {
           // Flatpak: ONLY portal strategy (sandbox blocks direct APIs)
           return Ok(Box::new(PortalTokenStrategy::new(...)));
       }
       DeploymentContext::SystemdSystem => {
           // System service: Limited to portal (D-Bus session complexity)
           return Ok(Box::new(PortalTokenStrategy::new(...)));
       }
       _ => {
           // Native, SystemdUser, InitD - full strategy access
       }
   }
   ```
   - Respects sandbox constraints
   - Prevents impossible configurations
   - Clear error messages

4. **Graceful fallback on Mutter unavailability:**
   ```rust
   if self.service_registry.service_level(ServiceId::DirectCompositorAPI) >= ServiceLevel::BestEffort {
       if MutterDirectStrategy::is_available().await {
           return Ok(Box::new(MutterDirectStrategy::new(monitor_connector)));
       } else {
           warn!("Service Registry reports Mutter API available, but connection failed");
           warn!("Falling back to Portal + Token strategy");
       }
   }
   ```
   - Double-checks availability at runtime
   - Logs reason for fallback
   - Continues gracefully

**Weaknesses:**

1. **Runtime check duplicates service registry:**
   - Service Registry checks D-Bus interfaces (translation.rs:433-434)
   - Strategy selector checks again (selector.rs:113: `is_available()`)
   - Redundant but defensive (acceptable)

2. **Monitor detection always runs for Mutter:**
   ```rust
   let monitor_connector = self.detect_primary_monitor().await;
   return Ok(Box::new(MutterDirectStrategy::new(monitor_connector)));
   ```
   - Enumerates /sys/class/drm every time
   - Could cache result
   - Minor performance issue (~10-20ms)

**Overall Rating: 8.5/10** - Very solid with one critical scope bug

---

### Mutter Direct Implementation (INCOMPLETE ⚠️)

**Location:** `src/mutter/`
- `mod.rs` - Public API and availability checks (74 lines)
- `screencast.rs` - D-Bus proxy for ScreenCast (321 lines)
- `remote_desktop.rs` - D-Bus proxy for RemoteDesktop (257 lines)
- `session_manager.rs` - High-level session management (321 lines)
- `pipewire_helper.rs` - PipeWire daemon connection (90 lines)

**Code Size:** ~1,100 lines

**Architectural Analysis:**

1. **D-Bus Proxy Pattern (Clean):**
   ```rust
   pub struct MutterScreenCast<'a> {
       proxy: zbus::Proxy<'a>,
   }

   impl MutterScreenCast<'_> {
       pub async fn create_session(&self, properties: HashMap<String, Value>) -> Result<OwnedObjectPath> {
           self.proxy.call("CreateSession", &(properties,)).await
       }
   }
   ```
   - Thin wrapper around zbus
   - Simple, maintainable
   - Follows Rust conventions

2. **Signal Handling (Correct After Many Fixes):**
   ```rust
   // Subscribe to PipeWireStreamAdded signal BEFORE calling Start()
   let mut signal_stream = stream_proxy
       .subscribe_for_node_id()
       .await?;

   // Start the session (triggers signal)
   session_proxy.start().await?;

   // Wait for signal with timeout
   let node_id = tokio::time::timeout(
       Duration::from_secs(5),
       signal_stream.next()
   ).await?;
   ```
   - Subscribe before triggering
   - Timeout protection
   - Error handling
   - **This was Bug #2 from MUTTER-GNOME-46-ISSUES.md** - correctly fixed

3. **Session Structure (Well-Designed):**
   ```rust
   pub struct MutterSessionHandle {
       pub screencast_session: OwnedObjectPath,
       pub remote_desktop_session: OwnedObjectPath,
       pub streams: Vec<OwnedObjectPath>,
       pub stream_info: Vec<MutterStreamInfo>,
       pub connection: zbus::Connection,  // Kept alive for proxies
   }
   ```
   - All session state in one place
   - Connection kept alive (prevents premature cleanup)
   - Stream info readily available

**Known Issues (Documented):**

1. **Session Linkage Broken on GNOME 46:**
   ```rust
   // From MUTTER-GNOME-46-ISSUES.md:
   // RemoteDesktop and ScreenCast sessions can't be linked
   // NotifyPointerMotionAbsolute fails: "No screen cast active"
   // RemoteDesktop.CreateSession() takes no arguments (can't pass session-id)
   // ScreenCast sessions don't expose SessionId property
   ```
   - **Root cause:** GNOME 46 API regression or undocumented change
   - **Evidence:** 10+ bugs fixed, still doesn't work
   - **Solution:** Marked as Unavailable in Service Registry

2. **PipeWire Node Connection Doesn't Work:**
   ```rust
   // Mutter provides node ID (e.g., 59)
   // Our code connects to PipeWire daemon and targets node
   // Stream connects but never receives frames (black screen)
   ```
   - **Root cause:** Unknown (different auth? missing init step?)
   - **Contrast:** Portal FD works perfectly (pre-configured by portal)

3. **Input Injection Fails:**
   ```rust
   // Keyboard: "Invalid key event"
   // Pointer: "No screen cast active"
   ```
   - Consequence of session linkage failure
   - Would work if sessions were properly linked

**Status on GNOME 46:**
- ❌ Completely broken (video, input, all failed)
- ✅ Correctly identified and disabled in Service Registry
- ✅ Graceful fallback to Portal works perfectly

**Status on GNOME 40-45:**
- ❓ **COMPLETELY UNTESTED** (critical gap)
- 🎯 This is THE REASON Mutter strategy exists (Portal v3 systems)
- 🔴 Must test on RHEL 9 and Ubuntu 22.04 LTS before claiming support

**Code Quality:**
- Clean implementation
- Well-structured
- Good error handling
- Properly documented issues

**Overall Rating: 7/10** - Good code that doesn't work on GNOME 46, unknown status on 40-45

---

### Portal Token Implementation (GOOD with CRITICAL BUG ⚠️)

**Location:** `src/session/strategies/portal_token.rs`

**Code Size:** 315 lines

**Strengths:**

1. **Persistence Rejection Handling (Excellent Design):**
   ```rust
   let (portal_handle, new_token, pre_created_clipboard_mgr) = match portal_manager
       .create_session(session_id.clone(), None)
       .await
   {
       Ok(result) => (result.0, result.1, None),
       Err(e) => {
           if error_msg.contains("cannot persist") || error_msg.contains("InvalidArgument") {
               // Gracefully retry without persistence
               // ...
           }
       }
   };
   ```
   - Detects specific error ("cannot persist")
   - Retries with `PersistMode::DoNot`
   - Continues operation (session still works, just won't persist)
   - **This is production-quality error handling**

2. **Clipboard Manager Lifecycle (Smart):**
   ```rust
   // In retry path:
   let clipboard_mgr = Arc::new(ClipboardManager::new().await?);
   let result = no_persist_manager
       .create_session(session_id, Some(clipboard_mgr.as_ref()))  // ← Enables it
       .await?;

   // Later, reuse the SAME manager:
   let clipboard_manager = if let Some(clipboard_mgr) = pre_created_clipboard_mgr {
       clipboard_mgr  // ← Reuse enabled manager
   } else {
       Arc::new(ClipboardManager::new().await?)  // ← Normal path
   };
   ```
   - Reuses clipboard manager that was enabled in Portal session
   - Prevents "clipboard not enabled" errors
   - **This was Bug #12 from MUTTER-GNOME-46-ISSUES.md** - correctly fixed

3. **Token Lifecycle:**
   - Load before session creation
   - Save if new token received
   - Handle missing tokens gracefully
   - Clear logging at each step

**Weaknesses:**

1. **CRITICAL:** portal_manager scope bug (see BUG #1 above)

2. **Error detection is string-based:**
   ```rust
   if error_msg.contains("cannot persist") || error_msg.contains("InvalidArgument") {
   ```
   - Fragile (error messages could change)
   - Better: Check error type/code if available
   - Works in practice but not ideal

3. **Clipboard manager created twice in some paths:**
   - Normal path: Creates once (line 273-279)
   - Retry path: Creates once, reuses (line 217-220, then 269-271)
   - Could be simplified

**Overall Rating: 7.5/10** - Excellent design with one critical implementation bug

---

## Strategy Switching: How It Actually Works

### Runtime Flow (As Implemented)

```
1. STARTUP (main.rs)
   └─> WrdServer::new(config)

2. CAPABILITY DETECTION (server/mod.rs:129)
   └─> probe_capabilities()
       ├─> identify_compositor() → CompositorType::Gnome { version: "46.0" }
       ├─> probe_portal_caps() → PortalCapabilities { version: 5, ... }
       ├─> detect_credential_storage() → GnomeKeyring
       └─> Returns CompositorCapabilities

3. SERVICE TRANSLATION (server/mod.rs:205)
   └─> ServiceRegistry::from_compositor(capabilities)
       └─> translate_capabilities()
           ├─> translate_damage_tracking() → Guaranteed
           ├─> translate_dmabuf() → Guaranteed (if DmaBuf) / BestEffort / Unavailable
           ├─> translate_metadata_cursor() → Guaranteed / Degraded / Unavailable
           ├─> translate_session_persistence() → Guaranteed (if Portal v4+ AND storage)
           ├─> translate_direct_compositor_api()
           │   └─> GNOME 46: Unavailable ("session linkage broken")
           │   └─> GNOME 40-45: BestEffort ("critical for Portal v3 systems")
           ├─> translate_credential_storage() → Guaranteed / BestEffort / Degraded
           └─> translate_unattended_access() → Guaranteed / BestEffort / Degraded

       Results in ServiceRegistry with all services indexed

4. STRATEGY SELECTION (server/mod.rs:236-244)
   └─> SessionStrategySelector::select_strategy()

       Decision tree:
       ├─> IF deployment == Flatpak
       │   └─> RETURN PortalTokenStrategy (no choice, sandbox constraint)
       │
       ├─> IF deployment == SystemdSystem
       │   └─> RETURN PortalTokenStrategy (D-Bus complexity)
       │
       ├─> IF service_level(DirectCompositorAPI) >= BestEffort
       │   └─> IF MutterDirectStrategy::is_available().await
       │       ├─> detect_primary_monitor() → Option<String>
       │       └─> RETURN MutterDirectStrategy::new(monitor)
       │   ELSE
       │       └─> WARN "Service Registry reports available but connection failed"
       │       └─> Fall through to Portal...
       │
       ├─> IF supports_session_persistence()
       │   └─> RETURN PortalTokenStrategy (with tokens)
       │
       └─> FALLBACK
           └─> RETURN PortalTokenStrategy (without tokens, dialog every time)

5. SESSION CREATION (server/mod.rs:250-254)
   └─> strategy.create_session()

       Portal path:
       ├─> Load restore token from TokenManager
       ├─> Create PortalManager with token
       ├─> Try: create_session()
       │   └─> OK → save new token if received
       │   └─> ERR "cannot persist" → Retry without persistence
       └─> Return PortalSessionHandleImpl

       Mutter path:
       ├─> Verify compositor is GNOME
       ├─> Check not in Flatpak
       ├─> Create MutterSessionManager
       ├─> Create ScreenCast session
       │   ├─> RecordMonitor(connector) OR RecordVirtual()
       │   ├─> Subscribe to PipeWireStreamAdded signal
       │   ├─> Start() session
       │   └─> Wait for signal → node_id
       ├─> Create RemoteDesktop session
       │   ├─> CreateSession() (no args)
       │   └─> Start() session
       └─> Return MutterSessionHandleImpl

6. HYBRID MODE (server/mod.rs:304-358)
   └─> IF session_handle.portal_clipboard().is_none()
       ├─> INFO "HYBRID MODE: Mutter for video, Portal for input+clipboard"
       ├─> Create PortalManager (AGAIN - see BUG #2)
       ├─> Create Portal session (one dialog)
       ├─> Create ClipboardManager
       └─> Create PortalSessionHandleImpl for input

   Portal returns clipboard, so:
       └─> Use strategy's existing session (no extra dialog)
```

**What's Brilliant:**

1. **Service Registry as Truth Source:**
   - Capability detection happens ONCE
   - Service translation happens ONCE
   - Strategy selector TRUSTS the registry
   - No runtime re-probing or guessing

2. **Evidence-Based Disabling:**
   - GNOME 46 Mutter tested → found broken → marked Unavailable
   - Strategy selector sees Unavailable → skips Mutter
   - Falls back to Portal (which was tested and works)
   - **No crashes, no user-visible errors, clean fallback**

3. **Deployment Constraints Respected:**
   - Flatpak → Portal only (correct)
   - systemd system service → Portal only (pragmatic)
   - Native → All strategies (maximum flexibility)

**What's Concerning:**

1. **GNOME 40-45 Marked BestEffort but NEVER TESTED:**
   ```rust
   // GNOME 40-45: Should work (critical for RHEL 9, Ubuntu 22.04 LTS)
   Some(v) if v >= 40.0 => {
       AdvertisedService::best_effort(ServiceId::DirectCompositorAPI, feature)
           .with_note("Mutter D-Bus API (critical for Portal v3 systems)")
   }
   ```
   - Assumption: API regressed in 46, so 40-45 should work
   - **Not validated with actual testing**
   - High risk if 40-45 also broken
   - **Critical: Must test on RHEL 9 before publishing**

2. **Mutter PipeWire Path Untested:**
   - Mutter provides node ID
   - Code connects to PipeWire daemon (pipewire_helper.rs)
   - **This failed on GNOME 46** (black screen)
   - Unknown if it works on 40-45
   - Different from Portal's pre-configured FD

3. **Hybrid Mode Duplicates Sessions** (see BUG #2)

**Overall Rating: 6/10** - Good architecture, but primary use case (GNOME 40-45) is completely untested

---

## Code Quality Assessment

### Testing Coverage: EXCELLENT ✅

**Unit Tests:** 296/296 passing (100%)

**Module Breakdown:**
```
✅ Session persistence:   13/13 passing
✅ Service registry:      24/24 passing
✅ Service translation:    4/4 passing
✅ Strategy selection:     2/2 passing
✅ Mutter availability:    1/1 passing (just check, not full functionality)
✅ Other modules:        252/252 passing

⏭️ Ignored (require environment): 15 tests
   - TPM 2.0 tests (require /dev/tpmrm0)
   - Secret Service tests (require running daemon)
   - Mutter tests (require GNOME session)
   - Portal tests (require portal services)
```

**Testing Strengths:**
1. All translation logic tested
2. Service registry queries tested
3. Strategy selection logic tested
4. Token encryption tested
5. Credential detection tested

**Testing Gaps:**
1. ❌ Retry path (persistence rejection) not tested
2. ❌ Hybrid mode not tested
3. ❌ Mutter full functionality not tested (only `is_available()`)
4. ❌ Portal actual session creation not tested (ignored)
5. ❌ Integration testing limited (manual only)

**Recommendation:** Add integration tests for retry path and hybrid mode

---

### Error Handling: VERY GOOD ✅

**Pattern:**
```rust
.await
.context("Failed to <what we were trying to do>")?;
```

**Strengths:**
- Consistent use of `anyhow::Context`
- Clear error messages
- Error chain preserved
- User-friendly formatting in main.rs

**Examples:**
```rust
// Good context chaining:
portal_manager
    .create_session(session_id.clone(), None)
    .await
    .context("Failed to create portal session")?;

// Nested for specificity:
MutterSessionManager::new()
    .await
    .context("Failed to create Mutter session manager")?;

let mutter_handle = manager
    .create_session(self.monitor_connector.as_deref())
    .await
    .context("Failed to create Mutter session")?;
```

**Weakness:**
- Some error messages still say "wrd-server" instead of "lamco-rdp-server" (from PHASE-3-COMPLETE.md)
- Need comprehensive error message audit

**Overall Rating: 9/10** - Excellent error handling with minor naming inconsistencies

---

### Code Organization: EXCELLENT ✅

**Module Structure:**
```
src/
├── compositor/         Capability detection
├── services/          Service Registry
├── session/           Session persistence & strategies
│   └── strategies/    Strategy implementations
├── mutter/            Mutter D-Bus API
├── server/            Main server orchestration
├── egfx/              H.264 video streaming
├── clipboard/         Clipboard integration
├── cursor/            Cursor handling
├── damage/            Damage detection
├── performance/       Adaptive FPS, Latency Governor
└── utils/             Diagnostics, metrics, errors
```

**Strengths:**
1. Clear module boundaries
2. Logical grouping by concern
3. Minimal cross-module dependencies
4. Re-exports for convenience (lib.rs)

**Trait Hierarchy:**
```
SessionStrategy (strategy.rs)
   ├── implemented by MutterDirectStrategy
   └── implemented by PortalTokenStrategy

SessionHandle (strategy.rs)
   ├── implemented by MutterSessionHandleImpl
   └── implemented by PortalSessionHandleImpl
```

**Dependency Flow:**
```
server/mod.rs
   → session/strategies/selector.rs (chooses strategy)
      → session/strategies/mutter_direct.rs (implements SessionStrategy)
      → session/strategies/portal_token.rs (implements SessionStrategy)
         → mutter/session_manager.rs (creates Mutter session)
         → portal/ (from lamco-portal crate)
   → services/registry.rs (queries capabilities)
      → services/translation.rs (translates compositor caps)
         → compositor/ (detects capabilities)
```

**No circular dependencies, clean DAG.**

**Overall Rating: 10/10** - Exemplary organization

---

### Documentation: COMPREHENSIVE ✅

**Code Comments:**
```rust
//! Module-level docs explaining:
//! - Purpose
//! - Architecture
//! - Usage examples
//! - Security model
//! - Compatibility notes
```

**Every public item documented:**
- All public functions have doc comments
- Complex logic has inline comments
- Design decisions explained

**External Documentation:**
- 54 canonical docs in docs/
- Architecture guides
- Implementation guides
- Testing procedures
- Known issues documented (MUTTER-GNOME-46-ISSUES.md)

**Examples:**
```rust
/// Portal + Token strategy
///
/// This strategy uses the XDG Portal with restore tokens for session persistence.
/// Works across all desktop environments with portal v4+.
pub struct PortalTokenStrategy {
    service_registry: Arc<ServiceRegistry>,
    token_manager: Arc<TokenManager>,
}
```

**Overall Rating: 9.5/10** - Excellent documentation

---

## Strategy Switching Solidity Assessment

### Current State: PARTIALLY TESTED

**What Works (Verified via Testing):**

1. **Portal Strategy on GNOME 46:** ✅ SOLID
   - Tested extensively (see crash-*.log files)
   - Persistence rejection handled
   - Clipboard works both directions
   - Input injection works
   - Video works
   - **Status:** Production-ready

2. **Fallback from Mutter to Portal:** ✅ SOLID
   - Service Registry marks GNOME 46 Mutter as Unavailable
   - Strategy selector sees Unavailable, skips to Portal
   - No crashes, clean fallback
   - Logs explain reasoning
   - **Status:** Works as designed

3. **Deployment Constraints:** ✅ SOLID
   - Flatpak → Portal only (correct)
   - Tested via unit tests
   - **Status:** Correct

### What's UNTESTED (Critical Gaps):

1. **Mutter Strategy on GNOME 40-45:** ❌ COMPLETELY UNTESTED
   - **Platforms:** RHEL 9 (GNOME 40), Ubuntu 22.04 LTS (GNOME 42)
   - **Status in Code:** Marked as BestEffort
   - **Reality:** Never executed successfully
   - **Risk:** Could be just as broken as GNOME 46
   - **Impact:** This is THE REASON Mutter exists (Portal v3 has no tokens)

2. **Hybrid Mode (Mutter video + Portal input):** ⚠️ DESIGN OK, BUG PRESENT
   - **Code:** Lines 316-358 in server/mod.rs
   - **Testing:** Not tested (Mutter never worked)
   - **Bug:** Duplicate Portal session (see BUG #2)
   - **Risk:** Two permission dialogs instead of one

3. **Portal Retry Path:** ⚠️ LOGIC OK, BUG PRESENT
   - **Code:** Lines 196-234 in portal_token.rs
   - **Testing:** Hit in crash logs (worked!)
   - **Bug:** portal_manager scope issue (see BUG #1)
   - **Status:** Logic proven via actual error, but bug present

4. **Token Restoration:** ⚠️ PARTIALLY TESTED
   - **First run:** Tested (no token, dialog appears)
   - **Second run:** Needs verification (does token actually restore?)
   - **Token save:** Tested via logs
   - **Token load:** Tested via logs
   - **Actual restore:** Unverified

### Strategy Switching Confidence Levels

**High Confidence (8-10/10):**
- ✅ Portal strategy on GNOME 46: **10/10** (extensively tested)
- ✅ Service Registry logic: **9/10** (well-tested, one fragile parse)
- ✅ Fallback from Mutter to Portal: **9/10** (works, verified)
- ✅ Deployment constraint enforcement: **10/10** (correct logic)

**Medium Confidence (5-7/10):**
- ⚠️ Portal retry path: **6/10** (logic works, scope bug present)
- ⚠️ Hybrid mode: **5/10** (untested, has bug, design sound)
- ⚠️ Token restoration: **6/10** (first run works, second run unverified)

**Low Confidence (0-4/10):**
- ❌ Mutter on GNOME 40-45: **2/10** (completely untested, might not work)
- ❌ Mutter PipeWire connection: **1/10** (failed on 46, unknown on 40-45)
- ❌ Mutter input injection: **1/10** (failed on 46, unknown on 40-45)

---

## Specific Code Issues (Beyond Critical Bugs)

### Issue #3: Unused Code in Mutter GNOME 46 Path

**Location:** `src/server/mod.rs:304-312`

**The Code:**
```rust
// Create Portal manager for input+clipboard (needed for both strategies)
let mut portal_config = config.to_portal_config();
portal_config.persist_mode = ashpd::desktop::PersistMode::DoNot;  // Don't persist (causes errors)
portal_config.restore_token = None;

let portal_manager = Arc::new(
    PortalManager::new(portal_config)
        .await
        .context("Failed to create Portal manager for input+clipboard")?,
);
```

**Issue:**
- Created for "both strategies"
- Portal strategy: Doesn't use it (has own manager from strategy)
- Mutter strategy: Creates YET ANOTHER session from it (line 335)
- **This PortalManager is never directly used**

**Impact:**
- Minor resource waste (~20ms + memory)
- Confusing code flow

**Fix:**
Move this inside the `else` branch (Mutter only):
```rust
let (portal_clipboard_manager, portal_clipboard_session, portal_input_handle) = if let Some(clipboard) = session_handle.portal_clipboard() {
    // Portal strategy path
    // ...
} else {
    // Mutter strategy path

    // Create Portal manager HERE (not before the if)
    let mut portal_config = config.to_portal_config();
    portal_config.persist_mode = ashpd::desktop::PersistMode::DoNot;
    portal_config.restore_token = None;

    let portal_manager = Arc::new(
        PortalManager::new(portal_config).await?
    );

    // ... rest of Mutter hybrid code
};
```

---

### Issue #4: RemoteDesktop Session Created But Unused on GNOME 46

**From:** PHASE-3-COMPLETE.md:401-406

**The Problem:**
When Mutter strategy runs on GNOME 46:
1. Create ScreenCast session ✅
2. Create RemoteDesktop session ✅
3. Discover Mutter input doesn't work ❌
4. Fall back to Portal for input ✅
5. Mutter RemoteDesktop session just sits there (wasted)

**Current State:**
- On GNOME 46, Service Registry marks Mutter as Unavailable
- Mutter strategy is never selected
- **This issue doesn't manifest**

**But:**
- Code still exists
- If Mutter were enabled on 46, would create unused session
- Architectural waste

**Fix (If Mutter Re-enabled):**
```rust
// In MutterSessionManager::create_session():
// Add option to create ScreenCast-only session (skip RemoteDesktop)

pub async fn create_session_video_only(
    &self,
    monitor_connector: Option<&str>,
) -> Result<MutterSessionHandle> {
    // Create ScreenCast session
    // DO NOT create RemoteDesktop session
    // Return handle without RemoteDesktop path
}
```

**Priority:** LOW (doesn't manifest currently)

---

### Issue #5: Clipboard Manager Creation Complexity

**Observation:** Clipboard manager gets created in multiple places with different logic

**Paths:**

1. **Portal strategy, normal path:**
   ```rust
   // portal_token.rs:273-279
   let clipboard_manager = Arc::new(
       lamco_portal::ClipboardManager::new().await?
   );
   ```

2. **Portal strategy, retry path:**
   ```rust
   // portal_token.rs:217-220 (in Err branch)
   let clipboard_mgr = Arc::new(
       lamco_portal::ClipboardManager::new().await?
   );

   // Enable it in session
   let result = no_persist_manager
       .create_session(session_id, Some(clipboard_mgr.as_ref()))
       .await?;

   // ... later, reuse it (lines 269-271)
   let clipboard_manager = if let Some(clipboard_mgr) = pre_created_clipboard_mgr {
       clipboard_mgr
   } else {
       // Create new (shouldn't happen in retry path)
   };
   ```

3. **Mutter hybrid mode:**
   ```rust
   // server/mod.rs:340-344
   let clipboard_mgr = Arc::new(
       lamco_portal::ClipboardManager::new().await?
   );
   ```

**Issue:**
- Three different creation points
- Retry path correctly reuses
- Normal path creates new
- Hybrid mode creates new
- **Risk:** Could theoretically create multiple managers

**Reality Check:**
- Each code path is mutually exclusive
- Only ONE manager ever created per execution
- Not actually a bug, just complex

**Improvement:**
Extract to helper function:
```rust
async fn ensure_clipboard_manager(
    pre_created: Option<Arc<ClipboardManager>>,
) -> Result<Arc<ClipboardManager>> {
    if let Some(mgr) = pre_created {
        debug!("Reusing pre-created clipboard manager");
        Ok(mgr)
    } else {
        debug!("Creating new clipboard manager");
        Arc::new(ClipboardManager::new().await?)
    }
}
```

**Priority:** LOW (works correctly, just hard to follow)

---

## Mutter Implementation Deep Dive

### What Was Fixed (From MUTTER-GNOME-46-ISSUES.md)

**12 Bugs Fixed During Investigation:**

1. ✅ Tokio runtime nesting → Separate thread for D-Bus checks
2. ✅ PipeWireNodeId property → Use signal instead
3. ✅ RemoteDesktop CreateSession signature → No arguments
4. ✅ Portal persistence rejection → Graceful retry
5. ✅ EIS connection required → Call ConnectToEIS()
6. ✅ Session handle lifetime → Store in WrdServer
7. ✅ RemoteDesktop proxy reuse → Store started proxy
8. ✅ D-Bus type mismatch → Convert ObjectPath to string
9. ✅ Method name → NotifyPointerMotionRelative
10. ✅ Error messages → Update repo URLs, paths
11. ✅ Clipboard manager lifecycle → Reuse enabled manager
12. ✅ Service Registry version detection → GNOME 46+ = Unavailable

**Assessment:**
- **Effort:** Significant (10+ iterations, 20+ hours estimated)
- **Quality:** Professional debugging (systematic, documented)
- **Outcome:** Determined API is fundamentally broken on GNOME 46
- **Decision:** Correctly disabled in Service Registry

### What's Still Broken on GNOME 46 (Cannot Fix)

**Core Issues (API Limitations):**

1. **Session Linkage:**
   - RemoteDesktop.CreateSession() takes NO arguments
   - ScreenCast sessions don't expose SessionId
   - No way to link RemoteDesktop → ScreenCast
   - NotifyPointerMotionAbsolute fails: "No screen cast active"

2. **PipeWire Node Access:**
   - Mutter provides node ID (e.g., 59)
   - Connecting to PipeWire daemon works
   - Creating stream works
   - Stream never receives frames (black screen)
   - Portal FD works perfectly (pre-configured)

**Conclusion:** GNOME 46 Mutter API incomplete or changed incompatibly

### What MIGHT Work on GNOME 40-45 (Speculation)

**Hypothesis:**
- Session linkage may have worked in GNOME 40-45
- PipeWire node access may have worked
- API may have changed in GNOME 46

**Evidence:**
- gnome-remote-desktop uses same API
- gnome-remote-desktop presumably works on earlier GNOME
- API existed and functioned historically

**Counter-Evidence:**
- No actual testing on 40-45
- Could be broken on all versions
- Could have different bugs on each version

**Critical Action Required:**
- **MUST TEST on RHEL 9 (GNOME 40.10)**
- **MUST TEST on Ubuntu 22.04 LTS (GNOME 42)**
- This is not optional for enterprise readiness

---

## Service Registry Intelligence Assessment

### Design Philosophy: RUNTIME ADAPTATION

**Concept:**
```
Detect → Translate → Query → Adapt
```

**Not:**
```
Hardcode → Configure → Hope
```

**Implementation:**

1. **Detection Layer** (compositor/):
   - Environment variables (DESKTOP_SESSION, XDG_CURRENT_DESKTOP)
   - D-Bus introspection (compositor-specific interfaces)
   - Portal version probing
   - Wayland global enumeration

2. **Translation Layer** (services/translation.rs):
   - CompositorCapabilities → Vec<AdvertisedService>
   - Each service gets ServiceLevel based on detection
   - Version-specific logic (GNOME 40 vs 45 vs 46)
   - Quirk-aware adjustments

3. **Query Layer** (services/registry.rs):
   - `has_service(id)` - Boolean availability
   - `service_level(id)` - Quality guarantee
   - `services_at_level(min)` - Filter by quality
   - `recommended_codecs()` - Codec selection
   - `can_avoid_permission_dialog()` - UX queries

4. **Application Layer** (server/, strategies/):
   - Query registry for decisions
   - No hardcoded logic
   - Adapt based on actual capabilities

**Example Decision:**
```rust
// From server/mod.rs:214-230
let damage_level = service_registry.service_level(ServiceId::DamageTracking);
if damage_level >= ServiceLevel::BestEffort {
    info!("✅ Damage tracking: {} - enabling adaptive FPS", damage_level);
} else {
    info!("⚠️ Damage tracking: {} - using frame diff fallback", damage_level);
}
```

**Assessment:**
- ✅ Decouples detection from decision-making
- ✅ Easy to add new services
- ✅ Easy to adjust levels based on testing
- ✅ Runtime information for user (logged)
- ✅ Future-proof (new compositors just need translation functions)

**Potential Over-Engineering?**
- For 2-3 strategies, this is complex
- But: Scales to many strategies (wlr-screencopy, Mir, Cosmic)
- Justified for long-term product

**Overall Rating: 9.5/10** - Sophisticated and appropriate for the problem space

---

## Portal vs Mutter: Architectural Comparison

### Portal Strategy

**API Used:**
- org.freedesktop.portal.ScreenCast (xdg-desktop-portal)
- org.freedesktop.portal.RemoteDesktop (xdg-desktop-portal)
- org.freedesktop.portal.Clipboard (xdg-desktop-portal)

**What It Provides:**
```rust
PipeWireAccess::FileDescriptor(fd)  // Pre-configured PipeWire FD
streams: Vec<StreamInfo>             // Stream metadata
notify_keyboard_keycode()            // Input injection
notify_pointer_motion_absolute()     // Pointer injection
portal_clipboard()                   // Clipboard via same session
```

**Advantages:**
- ✅ Universal (works on GNOME, KDE, Sway, Hyprland, etc.)
- ✅ Secure (user grants explicit permission)
- ✅ Restore tokens (Portal v4+) avoid re-granting
- ✅ Single session for video+input+clipboard (shared)
- ✅ PipeWire FD pre-configured (just works)
- ✅ Well-documented, stable API

**Disadvantages:**
- ⚠️ First-run dialog required (one-time)
- ⚠️ Portal v3: No tokens → dialog every restart
- ⚠️ Compositor-specific bugs (GNOME SelectionOwnerChanged broken)

**Production Status:**
- ✅ GNOME 46: Fully working (tested)
- ⏳ GNOME 40-45: Should work (untested)
- ⏳ KDE: Should work (untested)
- ⏳ Sway: Should work (untested)

---

### Mutter Direct API Strategy

**API Used:**
- org.gnome.Mutter.ScreenCast (gnome-shell)
- org.gnome.Mutter.RemoteDesktop (gnome-shell)

**What It Provides:**
```rust
PipeWireAccess::NodeId(node_id)     // PipeWire node ID (need to connect)
streams: Vec<StreamInfo>             // Stream metadata
notify_keyboard_keycode()            // Input injection (BROKEN on 46)
notify_pointer_motion_absolute()     // Pointer injection (BROKEN on 46)
portal_clipboard()                   // None (must use Portal separately)
```

**Advantages:**
- ✅ Zero permission dialogs (not even first time)
- ✅ GNOME-specific optimization
- ✅ Critical for Portal v3 systems (RHEL 9, Ubuntu 22.04 LTS)
- ✅ Headless-friendly (RecordVirtual)

**Disadvantages:**
- ❌ GNOME-only (not portable)
- ❌ Broken on GNOME 46 (session linkage issue)
- ❌ Requires separate Portal session for clipboard (hybrid mode)
- ❌ PipeWire node connection more complex (daemon socket vs pre-configured FD)
- ❌ Completely untested on target platforms (40-45)

**Production Status:**
- ❌ GNOME 46: Disabled (broken, tested)
- ❓ GNOME 40-45: **UNKNOWN** - This is the critical gap
- ❌ Non-GNOME: Not applicable

---

### Hybrid Mode (Mutter + Portal)

**Concept:**
- Mutter: Video capture (zero dialogs)
- Portal: Input + Clipboard (one dialog)
- **Total:** One dialog (better than Portal-only first run on Portal v3)

**Implementation:**
```rust
// server/mod.rs:316-358
if let Some(clipboard) = session_handle.portal_clipboard() {
    // Portal strategy: Everything in one session
} else {
    // Mutter strategy: Create separate Portal session
    info!("HYBRID MODE: Mutter for video (zero dialogs), Portal for input+clipboard (one dialog)");

    let (portal_handle, _) = portal_manager
        .create_session(session_id, None)  // ← Creates session
        .await?;

    // Create clipboard manager
    // Create input handle
}
```

**Design Assessment:**

**Strengths:**
- ✅ Logical approach (use Mutter for what works, Portal for what doesn't)
- ✅ Reduces dialogs vs. pure Portal on Portal v3 systems
- ✅ Maximizes unattended operation

**Weaknesses:**
- ⚠️ Complexity (two sessions to manage)
- 🔴 Duplicate session bug (BUG #2)
- ⚠️ Untested (Mutter never worked to trigger this path)

**Reality Check:**
- On GNOME 46: Never runs (Mutter disabled)
- On GNOME 40-45: Would run IF Mutter works
- **If Mutter works on 40-45:** This is valuable
- **If Mutter broken on 40-45:** This is dead code

**Decision Tree:**
```
IF Mutter works on GNOME 40-45:
  → Keep hybrid mode
  → Fix duplicate session bug (BUG #2)
  → Add integration tests

IF Mutter broken on all GNOME versions:
  → Remove hybrid mode code
  → Remove Mutter strategy entirely
  → Simplify to Portal-only
  → Remove ~1,100 lines of Mutter code
```

---

## Testing Matrix Analysis

### What's Been Tested

| Platform | Version | Portal Ver | Mutter | Strategy Used | Status | Dialogs |
|----------|---------|------------|--------|---------------|--------|---------|
| **Ubuntu 24.04** | GNOME 46.0 | v5 | Disabled | Portal + Token | ✅ Works | 1 first run, 0 after |

**Testing Evidence:**
- crash-*.log files (12/31/2025)
- test-run-latest.log
- PHASE-3-COMPLETE.md
- SESSION-END-STATUS-2025-12-31.md

**What Was Verified:**
- ✅ Service Registry detects GNOME 46
- ✅ Marks Mutter as Unavailable
- ✅ Selects Portal strategy
- ✅ Handles persistence rejection (retry path triggered)
- ✅ Clipboard both directions
- ✅ Input injection
- ✅ Video works
- ✅ Token saved to GNOME Keyring
- ✅ No crashes after fixes

### What's UNTESTED (Critical Gaps)

| Platform | Version | Portal Ver | Mutter | Expected Strategy | Status | Priority |
|----------|---------|------------|--------|-------------------|--------|----------|
| **RHEL 9** | GNOME 40.10 | **v4** | **Unknown** | **Mutter OR Portal** | ❌ **Untested** | **🔴 CRITICAL** |
| **Ubuntu 22.04** | GNOME 42.x | **v3** | **Unknown** | **Mutter OR Portal** | ❌ **Untested** | **🔴 CRITICAL** |
| **Fedora 39** | GNOME 45.x | v5 | Unknown | Mutter OR Portal | ❌ Untested | 🟡 High |
| **KDE Plasma** | KDE 6.x | v5 | N/A | Portal + Token | ❌ Untested | 🟡 High |
| **Sway** | wlroots | v4+ | N/A | Portal + Token | ❌ Untested | 🟡 Medium |

**The RHEL 9 / Ubuntu 22.04 Gap is CRITICAL:**

**Why:**
1. These are THE platforms where Mutter matters (Portal v3 = no tokens)
2. Mutter is marked as BestEffort on these platforms
3. **Entire Mutter strategy exists for these platforms**
4. Never tested on them
5. Could be just as broken as GNOME 46

**Consequences if Mutter Broken on 40-45:**
- Portal v3 systems: Dialog every restart (acceptable but not ideal)
- Enterprise value proposition weakened
- ~1,100 lines of Mutter code is dead code
- Service Registry logic for version detection is overly complex
- Should simplify to Portal-only

**Consequences if Mutter Works on 40-45:**
- Zero dialogs on enterprise Linux (huge win)
- Validates the architectural complexity
- Keep all Mutter code
- Just need to fix bugs #1 and #2

---

## Integration Points Analysis

### How Strategies Integrate with WrdServer

**Integration Point 1: PipeWire Connection**

```rust
// server/mod.rs:258-301
match session_handle.pipewire_access() {
    PipeWireAccess::FileDescriptor(fd) => {
        // Portal path: Direct FD
        info!("Using Portal-provided PipeWire file descriptor: {}", fd);
        (fd, streams)
    }
    PipeWireAccess::NodeId(node_id) => {
        // Mutter path: Connect to daemon, then target node
        info!("Using Mutter-provided PipeWire node ID: {}", node_id);
        let fd = crate::mutter::get_pipewire_fd_for_mutter()?;
        (fd, streams)
    }
}
```

**Assessment:**
- ✅ Clean abstraction (enum handles difference)
- ✅ Each path properly handled
- ⚠️ Mutter path (`get_pipewire_fd_for_mutter()`) untested in production
- ⚠️ **RISK:** Node targeting may not work (failed on GNOME 46)

**Integration Point 2: Input Injection**

```rust
// Portal strategy returns PortalSessionHandleImpl
impl SessionHandle for PortalSessionHandleImpl {
    async fn notify_keyboard_keycode(&self, keycode: i32, pressed: bool) -> Result<()> {
        let session = self.session.lock().await;
        self.remote_desktop
            .notify_keyboard_keycode(&session, keycode, pressed)
            .await
    }
}

// Mutter strategy returns MutterSessionHandleImpl
impl SessionHandle for MutterSessionHandleImpl {
    async fn notify_keyboard_keycode(&self, keycode: i32, pressed: bool) -> Result<()> {
        let rd_session = MutterRemoteDesktopSession::new(
            &self.mutter_handle.connection,
            self.mutter_handle.remote_desktop_session.clone(),
        ).await?;

        rd_session.notify_keyboard_keycode(keycode, pressed).await
    }
}
```

**Assessment:**
- ✅ Both implement same trait
- ✅ WrdInputHandler doesn't know which is used
- ✅ Perfect abstraction
- ⚠️ Mutter version creates new proxy for every call (less efficient but works)
- ❌ Mutter version never tested

**Integration Point 3: Clipboard**

```rust
// Portal strategy:
fn portal_clipboard(&self) -> Option<ClipboardComponents> {
    Some(ClipboardComponents {
        manager: Arc::clone(&self.clipboard_manager),
        session: Arc::clone(&self.session),
    })
}

// Mutter strategy:
fn portal_clipboard(&self) -> Option<ClipboardComponents> {
    None  // Mutter has no clipboard API
}
```

**Assessment:**
- ✅ Clean: Some means "shares session", None means "need separate"
- ✅ Server code handles both cases (server/mod.rs:316-358)
- ⚠️ Server code has duplicate session bug in None path

**Overall Integration Rating: 8/10** - Clean abstractions, some bugs in edge cases

---

## Recommendations

### CRITICAL: Fix Bugs Before Publication

**Priority 1: Fix portal_manager Scope Bug** 🔴
- **File:** `src/session/strategies/portal_token.rs:286`
- **Fix:** Track active_manager variable through match
- **Test:** Add integration test for retry path
- **Verification:** Run on system that rejects persistence
- **Timeline:** **Before any publication**

**Priority 2: Fix Duplicate Portal Session** 🟡
- **File:** `src/server/mod.rs:304-358`
- **Fix:** Move PortalManager creation into else branch
- **Test:** Verify only one session created in hybrid mode
- **Verification:** Check logs show one session, not two
- **Timeline:** **Before any publication**

**Priority 3: Remove Unused PortalManager** 🟡
- **File:** `src/server/mod.rs:308-312`
- **Fix:** Delete or move into Mutter path
- **Impact:** Cleaner code, 20ms faster
- **Timeline:** **Before publication**

---

### CRITICAL: Test Mutter on Target Platforms

**Why This Is Critical:**

The **entire justification** for Mutter strategy is Portal v3 systems (RHEL 9, Ubuntu 22.04 LTS).

**Current State:**
- Mutter marked as BestEffort on GNOME 40-45
- Strategy selector will TRY Mutter on these platforms
- **Never tested = unknown if works**

**Possible Outcomes:**

**Outcome A: Mutter Works on 40-45**
- ✅ Validates the architecture
- ✅ Zero dialogs on enterprise Linux
- ✅ Keep all Mutter code
- ✅ Fix bugs #1, #2, #3
- ✅ Write deployment guides
- ✅ Marketing: "Zero-dialog operation"

**Outcome B: Mutter Broken on 40-45 Too**
- ⚠️ Entire Mutter strategy is useless
- ⚠️ ~1,100 lines of dead code
- ⚠️ Remove Mutter entirely
- ⚠️ Simplify to Portal-only
- ⚠️ Portal v3: Dialog every restart (acceptable)
- ⚠️ Marketing: "One-time permission grant (Portal v4+)"

**Testing Plan:**

**Week 1 (This Week):**
1. Deploy to RHEL 9 VM (192.168.10.6)
2. Test Mutter strategy
3. If works: Celebrate, document, fix bugs
4. If broken: Document, decide on removal

**Week 2:**
1. Deploy to Ubuntu 22.04 LTS VM (need to acquire)
2. Verify RHEL 9 findings
3. Make final decision on Mutter

**Go/No-Go Decision Criteria:**

**Keep Mutter If:**
- Works on RHEL 9 OR Ubuntu 22.04 (at least one)
- Video works (frames received)
- Input works (mouse/keyboard functional)
- Worth the complexity (~1,100 lines)

**Remove Mutter If:**
- Broken on both RHEL 9 AND Ubuntu 22.04
- Same issues as GNOME 46
- Not worth maintaining

---

### Publication Strategy Options

Given the current state, you have three options:

#### Option 1: Test First, Then Publish (RECOMMENDED)

**Process:**
1. Fix bugs #1, #2, #3 (critical fixes)
2. Test on RHEL 9 (verify Mutter status)
3. Make Mutter keep/remove decision
4. Clean up based on decision
5. Publish with accurate documentation

**Timeline:** 1-2 weeks
**Risk:** LOW (know exactly what works)
**Quality:** HIGH (tested, verified)

**Recommendation:** ✅ **Do this** - Don't publish with untested critical code path

---

#### Option 2: Publish with Mutter Disabled (SAFE)

**Process:**
1. Fix bugs #1, #2, #3
2. Mark ALL Mutter as Unavailable in translation.rs
3. Remove hybrid mode code
4. Publish Portal-only version
5. Test Mutter after publication
6. Add Mutter in v0.2.0 if it works

**Timeline:** 1 week
**Risk:** LOW (only shipping tested code)
**Downside:** Lose potential enterprise feature

**Recommendation:** ⚠️ Conservative approach if time-constrained

---

#### Option 3: Publish As-Is (NOT RECOMMENDED)

**Process:**
1. Fix only bug #1 (critical)
2. Leave Mutter enabled on 40-45 (untested)
3. Publish
4. Hope it works

**Timeline:** 2-3 days
**Risk:** **HIGH** (shipping untested code marked as BestEffort)
**Problems:**
- Users on RHEL 9 might get broken Mutter
- Strategy selector would try it and fail
- Fallback would work but users see errors
- Bad first impression

**Recommendation:** ❌ **Don't do this** - Damages credibility

---

## Dependency Analysis

### External Crates (Published)

```toml
lamco-wayland = "0.2.2"
lamco-rdp = "0.4.0"
lamco-video = "0.1.2"
lamco-rdp-input = "0.1.1"
```

**Status:** ✅ Published, stable

### Path Dependencies (Blockers)

```toml
lamco-portal = { path = "../lamco-wayland/crates/lamco-portal" }  # v0.3.0 unpublished
lamco-pipewire = { path = "../lamco-rdp-workspace/crates/lamco-pipewire" }  # Unreleased fixes
lamco-clipboard-core = { path = "../lamco-rdp-workspace/crates/lamco-clipboard-core" }
lamco-rdp-clipboard = { path = "../lamco-rdp-workspace/crates/lamco-rdp-clipboard" }
```

**Blockers for Publication:**
1. ❌ Can't publish to crates.io with path dependencies
2. ❌ Can't build without these repos present

### IronRDP Fork (Major Blocker)

```toml
ironrdp = { path = "/home/greg/wayland/IronRDP/crates/ironrdp" }
ironrdp-server = { path = "/home/greg/wayland/IronRDP/crates/ironrdp-server" }
# ... 9 more ironrdp crates
```

**Your Fork Changes:**
- EGFX support (PR #1057 - pending)
- Clipboard file transfer (PRs #1063-1066 - pending)
- ZGFX compression (not yet submitted)
- Server integration (not yet submitted)

**Blocker:** Can't publish lamco-rdp-server until IronRDP fork is published or PRs merge

---

## Publication Path Analysis

### Path A: Wait for Upstream PRs to Merge

**Process:**
1. Wait for IronRDP PRs #1057, #1063-1066 to merge
2. Wait for IronRDP v0.14.0 publication (estimated Jan 2026)
3. Publish lamco-portal v0.3.0
4. Publish lamco-pipewire with fixes
5. Publish lamco-rdp-server with crates.io dependencies

**Pros:**
- ✅ Clean crates.io dependencies
- ✅ Easy for users to build
- ✅ No fork maintenance

**Cons:**
- ⏳ Dependent on upstream timeline (uncertain)
- ⏳ Could be weeks or months
- ⏳ Blocks your launch

**Timeline:** Unknown (upstream-dependent)

---

### Path B: Publish IronRDP Fork

**Process:**
1. Clean up IronRDP fork (remove experimental code)
2. Publish as `lamco-ironrdp` to crates.io OR GitHub registry
3. Update dependencies:
   ```toml
   [dependencies]
   ironrdp = { package = "lamco-ironrdp", version = "0.13.1" }
   # OR
   ironrdp = { git = "https://github.com/glamberson/IronRDP", branch = "combined-egfx-file-transfer" }
   ```
4. Publish lamco-portal v0.3.0
5. Publish lamco-pipewire with fixes
6. Publish lamco-rdp-server

**Pros:**
- ✅ Full control over timeline
- ✅ Can publish immediately
- ✅ Include all needed features

**Cons:**
- ⚠️ Fork maintenance burden
- ⚠️ Users might be confused (lamco-ironrdp vs ironrdp)
- ⚠️ Need to keep fork in sync with upstream
- ⚠️ Git dependency makes crates.io publication harder

**Timeline:** 1-2 weeks (your control)

---

### Path C: Vendor Dependencies (cargo vendor)

**Process:**
1. Run `cargo vendor` to create vendored dependencies
2. Include vendor/ directory in repository
3. Configure `.cargo/config.toml`:
   ```toml
   [source.crates-io]
   replace-with = "vendored-sources"

   [source.vendored-sources]
   directory = "vendor"
   ```
4. Build from vendored sources

**Pros:**
- ✅ Self-contained build
- ✅ No internet required
- ✅ Reproducible

**Cons:**
- ❌ Can't publish to crates.io (vendor/ too large)
- ❌ Only works for distribution packages (.deb, .rpm, Flatpak)
- ❌ Still need to solve publication separately

**Use Case:** Good for OBS builds, not for crates.io

**Timeline:** Immediate for packages, doesn't solve crates.io

---

### Path D: Git Submodules

**Process:**
1. Add IronRDP as submodule: `git submodule add https://github.com/glamberson/IronRDP deps/IronRDP`
2. Add lamco-wayland: `git submodule add https://github.com/lamco-admin/lamco-wayland deps/lamco-wayland`
3. Add lamco-rdp-workspace: `git submodule add ... deps/lamco-rdp-workspace`
4. Update Cargo.toml paths:
   ```toml
   lamco-portal = { path = "deps/lamco-wayland/crates/lamco-portal" }
   ironrdp = { path = "deps/IronRDP/crates/ironrdp" }
   ```

**Pros:**
- ✅ Self-contained repository
- ✅ Version-locked dependencies
- ✅ Easy to clone and build

**Cons:**
- ❌ Can't publish to crates.io
- ❌ Submodules are confusing for users
- ❌ Need to manage submodule updates

**Use Case:** Good for development, not for distribution

---

### Recommended Publication Approach

**Two-Phase Publication:**

**Phase 1: Early Access (v0.1.0-alpha) - THIS MONTH**
1. Fix bugs #1, #2, #3 (critical fixes)
2. Test on RHEL 9 (verify Mutter status)
3. Publish IronRDP fork as `lamco-ironrdp` v0.13.1
4. Publish lamco-portal v0.3.0
5. Publish lamco-pipewire v0.4.1 (with fixes)
6. Publish lamco-rdp-server v0.1.0-alpha
   ```toml
   [dependencies]
   ironrdp = { package = "lamco-ironrdp", version = "0.13.1" }
   lamco-portal = "0.3.0"
   lamco-pipewire = "0.4.1"
   ```
7. Document: "Alpha release using forked IronRDP until upstream PRs merge"

**Phase 2: Stable Release (v0.1.0) - WHEN UPSTREAM MERGES**
1. Wait for IronRDP v0.14.0 with your PRs
2. Update dependencies to upstream IronRDP
3. Publish lamco-rdp-server v0.1.0 stable
4. Archive `lamco-ironrdp` (no longer needed)

**Benefits:**
- ✅ Can publish immediately
- ✅ Users can use it now
- ✅ Clean migration path to upstream
- ✅ No long-term fork maintenance
- ✅ Clear versioning (alpha → stable)

---

## Code Freeze Readiness Assessment

### Can Code Be Frozen Now?

**Answer: NO** - Critical bugs must be fixed first

**Blockers:**
1. 🔴 Bug #1 (portal_manager scope) - MUST FIX
2. 🔴 Bug #2 (duplicate Portal session) - MUST FIX
3. 🔴 Mutter testing on RHEL 9 - MUST TEST before claiming support
4. 🟡 Bug #3 (unused PortalManager) - SHOULD FIX

**After Fixes:**

**Freeze-Ready:**
- ✅ Portal strategy (tested, working)
- ✅ Service Registry (excellent architecture)
- ✅ Strategy selection (works correctly)
- ✅ Error handling (comprehensive)
- ✅ Tests (296/296 passing)

**Not Freeze-Ready:**
- ❌ Mutter strategy (untested on target platforms)
- ❌ Hybrid mode (has bugs, untested)

**Recommendation:**

**Option 1: Freeze with Mutter Disabled**
- Mark ALL Mutter as Unavailable (not just 46+)
- Remove hybrid mode code
- Freeze Portal-only version
- Test Mutter separately
- Add in v0.2.0 if it works

**Option 2: Test Then Freeze**
- Test Mutter on RHEL 9 FIRST
- If works: Fix bugs, then freeze
- If broken: Disable Mutter, then freeze

**Both are acceptable.** Option 2 gives you maximum features on day 1, Option 1 gets you to market faster.

---

## Summary and Verdict

### Architecture Grade: A (9/10)

**Strengths:**
- Excellent abstraction layers
- Clean separation of concerns
- Service Registry is sophisticated and well-executed
- Strategy pattern perfectly applied
- Graceful degradation throughout

**Weaknesses:**
- Critical bug in portal_token.rs (scope issue)
- Duplicate session in hybrid mode
- Untested critical code path (Mutter on 40-45)

### Code Quality Grade: A- (8.5/10)

**Strengths:**
- 296/296 tests passing
- Comprehensive error handling
- Well-documented
- Clean organization
- Professional debugging (Mutter investigation)

**Weaknesses:**
- One critical scope bug
- Some dead code (unused PortalManager)
- Testing gaps (retry path, hybrid mode)
- String-based error detection

### Production Readiness Grade: B+ (7.5/10)

**Production-Ready:**
- ✅ Portal strategy on GNOME 46: **YES**
- ✅ Fallback mechanisms: **YES**
- ✅ Error handling: **YES**

**Not Production-Ready:**
- ❌ Mutter strategy: **NO** (untested)
- ❌ Hybrid mode: **NO** (bugs, untested)
- ❌ Critical bugs unfixed: **NO**

**After Bug Fixes + Mutter Testing:**
- Overall grade would be: **A (9/10) - Production-Ready**

---

## Final Recommendations

### Immediate Actions (Before Any Publication)

1. **Fix Bug #1** (portal_manager scope)
   - Change portal_token.rs line 286
   - Track active_manager through match
   - Test retry path

2. **Fix Bug #2** (duplicate Portal session)
   - Move PortalManager creation into Mutter branch
   - Verify one session only

3. **Fix Bug #3** (unused PortalManager)
   - Remove lines 308-312 from server/mod.rs
   - Or move into Mutter branch

4. **Test on RHEL 9**
   - Deploy binary (use OBS or build on RHEL 9)
   - Test Mutter strategy
   - Document results

5. **Make Mutter Decision**
   - If works: Keep, document
   - If broken: Disable, simplify

### Code Freeze Decision

**Freeze After:**
- ✅ Bugs #1, #2, #3 fixed
- ✅ RHEL 9 tested
- ✅ Mutter decision made
- ✅ Dead code removed (if Mutter disabled)

**Estimated Timeline:** 3-7 days

### Publication Readiness

**Current State: NOT READY**
- Critical bugs present
- Untested critical code path
- Unclear feature set (keep or remove Mutter?)

**After Fixes: READY**
- All critical bugs fixed
- Tested code only
- Clear documentation
- Known limitations documented

---

**VERDICT: Strong architecture with critical bugs that MUST be fixed before publication. Test Mutter on RHEL 9 to determine final feature set, then freeze.**

*End of Audit - 2026-01-04*
