# Service Registry - Technical Documentation

## Overview

The Service Registry system translates Wayland compositor capabilities into RDP-compatible service advertisements. This enables runtime decisions based on what's actually available on the host system.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Service Registry Flow                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────────┐     ┌───────────────────┐     ┌────────────────┐  │
│  │  Compositor      │────▶│  ServiceRegistry  │────▶│  Runtime       │  │
│  │  Probing         │     │                   │     │  Decisions     │  │
│  │                  │     │  • 11 services    │     │                │  │
│  │  • Portal caps   │     │  • 4 levels       │     │  • Codec sel.  │  │
│  │  • Wayland globs │     │  • RDP mappings   │     │  • FPS config  │  │
│  │  • Quirks        │     │  • Perf hints     │     │  • Cursor mode │  │
│  └──────────────────┘     └───────────────────┘     └────────────────┘  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
src/services/
├── mod.rs                 # Module exports, documentation
├── service.rs             # ServiceId, ServiceLevel, AdvertisedService
├── wayland_features.rs    # WaylandFeature enum
├── rdp_capabilities.rs    # RdpCapability enum
├── registry.rs            # ServiceRegistry implementation
└── translation.rs         # Compositor → Service translation
```

## Core Types

### ServiceId (18 services)

```rust
pub enum ServiceId {
    // Display Services (8)
    DamageTracking,      // Bandwidth optimization via dirty region detection
    DmaBufZeroCopy,      // GPU buffer zero-copy path
    ExplicitSync,        // Tear-free display synchronization
    FractionalScaling,   // HiDPI support
    MetadataCursor,      // Client-side cursor rendering
    MultiMonitor,        // Multiple display support
    WindowCapture,       // Per-window capture capability
    HdrColorSpace,       // HDR passthrough (future)

    // I/O Services (3)
    Clipboard,           // Bidirectional clipboard
    RemoteInput,         // Keyboard/mouse injection
    VideoCapture,        // PipeWire video stream

    // Session Persistence Services (7) - Phase 2
    SessionPersistence,  // Portal restore token support
    DirectCompositorAPI, // Mutter/compositor direct APIs
    CredentialStorage,   // Token encryption backends
    UnattendedAccess,    // Zero-dialog capability
    WlrScreencopy,       // wlroots direct capture
    WlrDirectInput,      // wlroots virtual keyboard/pointer
    LibeiInput,          // Portal + EIS/libei protocol
}
```

### ServiceLevel (4 levels, ordered)

```rust
pub enum ServiceLevel {
    Unavailable = 0,  // Not supported on this compositor
    Degraded = 1,     // Known issues, fallbacks in use
    BestEffort = 2,   // Works but may have limitations
    Guaranteed = 3,   // Fully supported and tested
}
```

**Key Property**: Implements `Ord`, so comparisons work:
```rust
if registry.service_level(ServiceId::DamageTracking) >= ServiceLevel::BestEffort {
    // Enable adaptive FPS
}
```

### WaylandFeature

Represents the detected Wayland-side feature with metadata:

```rust
pub enum WaylandFeature {
    DamageTracking { method: DamageMethod, compositor_hints: bool },
    DmaBufZeroCopy { formats: Vec<DrmFormat>, supports_modifiers: bool },
    ExplicitSync { version: u32 },
    FractionalScaling { max_scale: f32 },
    MetadataCursor { has_hotspot: bool, has_shape_updates: bool },
    MultiMonitor { max_monitors: u32, virtual_source: bool },
    WindowCapture { has_toplevel_export: bool },
    HdrColorSpace { transfer: HdrTransfer, gamut: String },
    Clipboard { portal_version: u32 },
    RemoteInput { uses_libei: bool, keyboard: bool, pointer: bool, touch: bool },
    PipeWireStream { node_id: Option<u32>, buffer_type: String },
}
```

### RdpCapability

Represents the RDP-side capability this maps to:

```rust
pub enum RdpCapability {
    EgfxCodec { avc444: bool, avc420: bool, remotefx: bool, progressive: bool },
    DesktopComposition { multi_mon: bool, max_monitors: u32, scaling: bool },
    CursorChannel { metadata: bool, large_cursor: bool, color_depth: u8 },
    ClipboardExtended { file_copy: bool, max_size_bytes: u64, formats: Vec<String> },
    InputCapability { keyboard: bool, mouse: bool, touch: bool, unicode: bool },
    SurfaceManagement { max_surfaces: u32, surface_commands_version: u32 },
    FrameAcknowledge { enabled: bool, max_unacked: u32 },
    Custom { name: String, version: u32, properties: Vec<(String, String)> },
}
```

### AdvertisedService

Complete service advertisement:

```rust
pub struct AdvertisedService {
    pub id: ServiceId,
    pub name: String,
    pub wayland_source: Option<WaylandFeature>,
    pub rdp_capability: Option<RdpCapability>,
    pub level: ServiceLevel,
    pub performance: PerformanceHints,
    pub notes: Option<String>,
}
```

### PerformanceHints

Optimization hints for runtime:

```rust
pub struct PerformanceHints {
    pub recommended_fps: Option<u32>,
    pub latency_overhead_ms: Option<u32>,
    pub zero_copy_available: bool,
    pub buffer_count: Option<u32>,
    pub simd_available: bool,
}
```

## Translation Logic

### Per-Compositor Profiles

The translation layer (`translation.rs`) maps compositor capabilities to service levels:

#### GNOME (Mutter)

| Service | Level | Reason |
|---------|-------|--------|
| DamageTracking | Guaranteed (45+) / BestEffort | Portal-based, GNOME 45+ has better hints |
| DmaBufZeroCopy | Unavailable | GNOME prefers MemFd buffers |
| ExplicitSync | Unavailable | Not yet in GNOME |
| MetadataCursor | Guaranteed | Full metadata cursor support |
| MultiMonitor | BestEffort | RestartCaptureOnResize quirk |
| Clipboard | Guaranteed | Portal v2+ |

#### KDE (KWin)

| Service | Level | Reason |
|---------|-------|--------|
| DamageTracking | Guaranteed (Plasma 6+) | Native damage hints |
| DmaBufZeroCopy | Guaranteed | Excellent DMA-BUF support |
| ExplicitSync | Guaranteed (Plasma 6+) | Full explicit sync |
| MetadataCursor | Guaranteed | Full support |
| MultiMonitor | Guaranteed (Plasma 6+) | No quirks in Plasma 6 |

#### Sway/wlroots

| Service | Level | Reason |
|---------|-------|--------|
| DamageTracking | Guaranteed | Native screencopy with damage |
| DmaBufZeroCopy | Guaranteed | Excellent DMA-BUF |
| ExplicitSync | Guaranteed | Full support |
| MetadataCursor | Degraded | NeedsExplicitCursorComposite quirk |
| MultiMonitor | Guaranteed | No quirks |

### Quirk-Aware Level Assignment

Quirks detected during probing affect service levels:

```rust
// Example: NeedsExplicitCursorComposite quirk
if has_metadata && !needs_composite {
    AdvertisedService::guaranteed(ServiceId::MetadataCursor, feature)
} else if has_metadata {
    AdvertisedService::degraded(
        ServiceId::MetadataCursor,
        feature,
        "Requires explicit cursor compositing",
    )
}
```

## ServiceRegistry API

### Creation

```rust
let caps = probe_capabilities().await?;
let registry = ServiceRegistry::from_compositor(caps);
```

### Query Methods

```rust
// Check if service exists at any level
registry.has_service(ServiceId::DamageTracking) -> bool

// Get service level (returns Unavailable if not found)
registry.service_level(ServiceId::DamageTracking) -> ServiceLevel

// Get full service details
registry.get_service(ServiceId::DamageTracking) -> Option<&AdvertisedService>

// Get all services
registry.all_services() -> &[AdvertisedService]

// Filter by level
registry.services_at_level(ServiceLevel::Guaranteed) -> Vec<&AdvertisedService>
registry.guaranteed_services() -> Vec<&AdvertisedService>
registry.usable_services() -> Vec<&AdvertisedService>
```

### Decision Methods

```rust
// Codec selection
registry.recommended_codecs() -> Vec<&'static str>
registry.should_enable_avc444() -> bool

// Feature enablement
registry.should_enable_adaptive_fps() -> bool
registry.should_use_predictive_cursor() -> bool

// Performance
registry.recommended_fps() -> u32
```

### Logging

```rust
// Full table output
registry.log_summary();

// Compact status line
registry.status_line() -> String
// Output: "Services: ✅6 🔶2 ⚠️0 ❌3"
```

## Server Integration

### Startup Sequence

In `WrdServer::new()`:

```rust
// 1. Probe compositor capabilities
let capabilities = probe_capabilities().await?;

// 2. Create service registry
let service_registry = ServiceRegistry::from_compositor(capabilities.clone());
service_registry.log_summary();

// 3. Make service-aware decisions
let damage_level = service_registry.service_level(ServiceId::DamageTracking);
if damage_level >= ServiceLevel::BestEffort {
    info!("✅ Damage tracking: {} - enabling adaptive FPS", damage_level);
}
```

### Log Output Example

```
╔════════════════════════════════════════════════════════════╗
║              Service Advertisement Registry                ║
╚════════════════════════════════════════════════════════════╝
  Compositor: GNOME 46.0
  Services: 6 guaranteed, 2 best-effort, 0 degraded, 3 unavailable
  ─────────────────────────────────────────────────────────
  ✅ Damage Tracking       [Guaranteed]
  ❌ DMA-BUF Zero-Copy     [Unavailable]
      ↳ Compositor prefers MemFd buffers
  ❌ Explicit Sync         [Unavailable]
  ❌ Fractional Scaling    [Unavailable]
  ✅ Metadata Cursor       [Guaranteed]  → Cursor[metadata]
  🔶 Multi-Monitor         [BestEffort]  → Desktop[multi-mon, max=4]
      ↳ Capture restarts on resolution change
  🔶 Window Capture        [BestEffort]
  ❌ HDR Color Space       [Unavailable]
  ✅ Clipboard             [Guaranteed]  → Clipboard[text, 10MB]
  ✅ Remote Input          [Guaranteed]  → Input[kbd,mouse]
  ✅ Video Capture         [Guaranteed]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🎛️ Service-based feature configuration:
   ✅ Damage tracking: Guaranteed - enabling adaptive FPS
   ✅ Metadata cursor: Guaranteed - client-side rendering
   ⚠️ DMA-BUF: Unavailable - using memory copy path
```

## Test Coverage

16 unit tests covering:
- Service level ordering and comparison
- Service ID enumeration
- Registry creation and queries
- GNOME profile translation
- DRM format properties
- Feature display formatting

```bash
cargo test --release services::
# 16 passed; 0 failed
```

## Future Enhancements (Phase 3+)

1. **Runtime Negotiation**: Pass registry to display handler for adaptive behavior
2. **IronRDP Integration**: Modify capability negotiation to use registry
3. **Dynamic Updates**: Handle compositor changes during session
4. **Client Protocol**: Custom DVC channel for service discovery

## Files Reference

| File | Lines | Purpose |
|------|-------|---------|
| `src/services/mod.rs` | 45 | Module exports and documentation |
| `src/services/service.rs` | 220 | Core types: ServiceId, ServiceLevel, AdvertisedService |
| `src/services/wayland_features.rs` | 175 | WaylandFeature enum with all variants |
| `src/services/rdp_capabilities.rs` | 180 | RdpCapability enum with presets |
| `src/services/registry.rs` | 280 | ServiceRegistry implementation |
| `src/services/translation.rs` | 395 | Compositor → Service translation |
| **Total** | **~1,295** | Complete service advertisement system |
