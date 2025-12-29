# Implementation Plan: Priority 2 - Capability Probing

**Effort**: 16-20 hours
**Impact**: Auto-adapt to different DEs, "just works" quality
**Dependencies**: None
**Files**: New module src/compositor/, updates to src/server/mod.rs

---

## OVERVIEW

**Goal**: Automatically detect compositor capabilities and adapt behavior

**Why**: Avoid maintaining DE-specific code, handle version differences gracefully

**What**: Runtime probing of Wayland globals, Portal features, system capabilities

---

## ARCHITECTURE

```
src/compositor/
├── mod.rs              # Public API
├── probing.rs          # Wayland global enumeration
├── capabilities.rs     # Capability structures
└── profiles.rs         # Known DE profiles
```

---

## PHASE 1: Wayland Global Probing (6-8 hours)

### File: `src/compositor/probing.rs`

```rust
use wayland_client::{Connection, protocol::wl_registry};

pub struct WaylandProber {
    connection: Connection,
}

impl WaylandProber {
    pub fn new() -> Result<Self> {
        let connection = Connection::connect_to_env()?;
        Ok(Self { connection })
    }

    /// Enumerate all Wayland globals
    pub fn enumerate_globals(&self) -> Result<Vec<Global>> {
        let mut globals = Vec::new();

        // Connect to registry
        let display = self.connection.display();
        let registry = display.get_registry();

        // Bind registry and listen for globals
        registry.quick_assign(|_, event, _| {
            if let wl_registry::Event::Global { name, interface, version } = event {
                globals.push(Global {
                    name,
                    interface: interface.to_string(),
                    version,
                });
            }
        });

        // Roundtrip to get all globals
        self.connection.roundtrip()?;

        Ok(globals)
    }

    /// Check if specific protocol is available
    pub fn has_protocol(&self, interface: &str, min_version: u32) -> bool {
        let globals = self.enumerate_globals().ok()?;
        globals.iter().any(|g| g.interface == interface && g.version >= min_version)
    }
}

pub struct Global {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}
```

**Check for**:
- `zwlr_screencopy_manager_v1` (wlroots capture)
- `ext_image_copy_capture_v1` (modern capture)
- `wp_viewporter` (scaling support)
- `wp_fractional_scale_manager_v1` (fractional DPI)
- `xx_color_manager_v1` (color management)
- `org_kde_kwin_*` (KDE-specific protocols)

---

## PHASE 2: Portal Capability Detection (4-6 hours)

### File: `src/compositor/portal_caps.rs`

```rust
use ashpd::desktop::{screencast, remote_desktop};

pub struct PortalCapabilities {
    pub version: u32,
    pub supports_screencast: bool,
    pub supports_remote_desktop: bool,
    pub supports_clipboard: bool,
    pub available_cursor_modes: Vec<CursorMode>,
    pub available_source_types: Vec<SourceType>,
}

pub async fn probe_portal_capabilities() -> Result<PortalCapabilities> {
    // Query Portal version
    let version = query_portal_version().await?;

    // Check ScreenCast availability
    let supports_screencast = test_screencast_available().await;

    // Check RemoteDesktop availability
    let supports_remote_desktop = test_remote_desktop_available().await;

    // Query supported cursor modes
    let cursor_modes = query_cursor_modes().await;

    // Query supported source types (monitor, window, region)
    let source_types = query_source_types().await;

    Ok(PortalCapabilities {
        version,
        supports_screencast,
        supports_remote_desktop,
        supports_clipboard,
        available_cursor_modes: cursor_modes,
        available_source_types: source_types,
    })
}
```

---

## PHASE 3: Compositor Profiles (2-3 hours)

### File: `src/compositor/profiles.rs`

```rust
pub enum CompositorType {
    Gnome { version: String },
    Kde { version: String },
    Sway { version: String },
    Hyprland { version: String },
    Weston,
    Unknown,
}

pub struct CompositorProfile {
    pub compositor: CompositorType,
    pub wayland_protocols: Vec<String>,
    pub portal_backend: Option<String>,
    pub recommended_capture: CaptureBackend,
    pub recommended_buffer_type: BufferType,
    pub supports_damage_hints: bool,
    pub supports_explicit_sync: bool,
    pub quirks: Vec<Quirk>,
}

pub fn identify_compositor() -> Result<CompositorType> {
    // Check environment variables
    if let Ok(session) = std::env::var("DESKTOP_SESSION") {
        if session.contains("gnome") {
            return Ok(CompositorType::Gnome { version: detect_gnome_version()? });
        }
        // ... other DEs
    }

    // Check Wayland globals for compositor-specific protocols
    let globals = probe_wayland_globals()?;

    if globals.iter().any(|g| g.interface.contains("org_kde")) {
        return Ok(CompositorType::Kde { version: detect_kde_version()? });
    }

    // ... other detection methods

    Ok(CompositorType::Unknown)
}

pub fn create_profile(compositor: CompositorType) -> CompositorProfile {
    match compositor {
        CompositorType::Gnome { version } => {
            CompositorProfile {
                compositor,
                wayland_protocols: vec![/* GNOME protocols */],
                portal_backend: Some("gnome".to_string()),
                recommended_capture: CaptureBackend::Portal,
                recommended_buffer_type: BufferType::MemFd,  // GNOME prefers MemFd
                supports_damage_hints: true,
                supports_explicit_sync: false,  // Not yet in GNOME
                quirks: vec![Quirk::RequiresWaylandSession],
            }
        }
        CompositorType::Kde { version } => {
            CompositorProfile {
                recommended_buffer_type: BufferType::DmaBuf,  // KDE better with DMA-BUF
                // ... KDE-specific settings
            }
        }
        // ... other compositors
    }
}

pub enum Quirk {
    RequiresWaylandSession,
    SlowPortalPermissions,
    PoorDmaBufSupport,
    NeedsExplicitCursorComposite,
}
```

---

## PHASE 4: Integration (4-5 hours)

### File: `src/server/mod.rs`

**Add capability detection on startup**:

```rust
pub async fn new(config: Config) -> Result<Self> {
    // ... existing initialization ...

    // === CAPABILITY PROBING ===
    info!("Probing compositor capabilities...");

    let compositor = crate::compositor::identify_compositor()?;
    info!("Detected compositor: {:?}", compositor);

    let profile = crate::compositor::create_profile(compositor);
    info!("Compositor profile: {:?}", profile);

    let portal_caps = crate::compositor::probe_portal_capabilities().await?;
    info!("Portal capabilities: {:?}", portal_caps);

    // Apply quirks and optimizations
    for quirk in &profile.quirks {
        match quirk {
            Quirk::RequiresWaylandSession => {
                // Verify we're in Wayland session
                if std::env::var("WAYLAND_DISPLAY").is_err() {
                    warn!("Not in Wayland session - may have issues");
                }
            }
            Quirk::SlowPortalPermissions => {
                // Increase timeout for permission dialogs
                increase_portal_timeout();
            }
            // ... handle other quirks
        }
    }

    // Configure based on capabilities
    let capture_config = if profile.supports_damage_hints {
        CaptureConfig::with_compositor_damage()
    } else {
        CaptureConfig::with_software_damage()
    };

    // ... rest of initialization with capability-aware config ...
}
```

---

## TESTING

### Test Matrix

**Test on each DE**:
- [ ] GNOME 46 (Ubuntu 24.04)
- [ ] KDE Plasma 6 (if available)
- [ ] Sway (if available)
- [ ] Unknown compositor (fallback behavior)

**For each**:
- [ ] Correct compositor detected
- [ ] Appropriate profile loaded
- [ ] Quirks applied correctly
- [ ] Features work as expected
- [ ] Logs show capability info

---

## SUCCESS CRITERIA

- [ ] Auto-detects GNOME, KDE, Sway correctly
- [ ] Gracefully handles unknown compositors
- [ ] Applies appropriate optimizations per DE
- [ ] Logs clear capability information
- [ ] Works on all tested DEs without manual config

---

**Estimated**: 16-20 hours
**Priority**: #2
**Dependencies**: None
