# Service Advertisement Architecture

## Overview

The Service Advertisement system bridges Wayland compositor capabilities to RDP clients through a unified registry that translates detected features into RDP-compatible service advertisements.

## Problem Statement

Currently, the compositor probing system detects extensive information about the running Wayland environment:
- Compositor type and version (GNOME 46, KDE 6, Sway 1.9, etc.)
- Portal capabilities (ScreenCast, RemoteDesktop, Clipboard)
- Available Wayland protocols
- Known quirks requiring workarounds

However, this information is **not translated** into RDP capabilities or communicated to clients. The detected features are used only for internal workarounds rather than enabling optimized client experiences.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Service Advertisement Flow                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌──────────────────┐    ┌────────────────┐ │
│  │   Wayland   │───►│     Service      │───►│   RDP Client   │ │
│  │  Compositor │    │     Registry     │    │   (mstsc/      │ │
│  └─────────────┘    └──────────────────┘    │   FreeRDP)     │ │
│        │                    │               └────────────────┘ │
│        ▼                    ▼                       │          │
│  ┌─────────────┐    ┌──────────────────┐    ┌────────────────┐ │
│  │ Compositor  │    │ WaylandFeature   │    │  Capability    │ │
│  │ Probing     │───►│ → RdpCapability  │───►│  Negotiation   │ │
│  │             │    │   Translation    │    │                │ │
│  │ • Portal    │    │                  │    │ • EGFX codec   │ │
│  │ • Globals   │    │ • Service levels │    │ • Input caps   │ │
│  │ • Quirks    │    │ • Performance    │    │ • Clipboard    │ │
│  └─────────────┘    └──────────────────┘    └────────────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Feature Mapping Matrix

| Wayland Feature | Detection Method | RDP Capability | Client Benefit |
|----------------|------------------|----------------|----------------|
| Damage Tracking | `supports_damage_hints` | EGFX optimization | 70% bandwidth savings |
| DMA-BUF Support | `recommended_buffer_type` | Quality preset | Zero-copy, lower latency |
| Explicit Sync | `supports_explicit_sync` | Frame pacing | Tear-free display |
| Fractional Scale | `has_fractional_scale()` | Desktop scaling | HiDPI support |
| Metadata Cursor | Portal `CursorMode::Metadata` | Cursor channel | Smooth cursor |
| Multi-Monitor | Portal `SourceType::Virtual` | EGFX multi-surface | True multi-mon |
| HDR Color Space | Portal version 5+ | HDR capability | HDR passthrough |
| Window Capture | Portal `SourceType::Window` | App remoting | Per-window access |

## Core Components

### ServiceRegistry

Central registry that holds all advertised services and translates compositor capabilities:

```rust
pub struct ServiceRegistry {
    compositor_caps: CompositorCapabilities,
    services: Vec<AdvertisedService>,
    compatibility: FeatureCompatibility,
}
```

### AdvertisedService

Individual service advertisement with level guarantees:

```rust
pub struct AdvertisedService {
    pub id: ServiceId,
    pub name: String,
    pub wayland_source: WaylandFeature,
    pub rdp_capability: Option<RdpCapability>,
    pub level: ServiceLevel,
    pub performance: PerformanceHints,
}
```

### ServiceLevel

Guarantees about service availability:

```rust
pub enum ServiceLevel {
    /// Feature is fully supported and tested
    Guaranteed,
    /// Feature works but may have limitations
    BestEffort,
    /// Feature detected but known issues exist
    Degraded,
    /// Feature not available on this compositor
    Unavailable,
}
```

### WaylandFeature

Enumeration of detectable Wayland features:

```rust
pub enum WaylandFeature {
    DamageTracking { method: DamageMethod },
    DmaBufZeroCopy { formats: Vec<DrmFormat> },
    ExplicitSync,
    FractionalScaling { max_scale: f32 },
    MetadataCursor,
    MultiMonitor { max_monitors: u32 },
    WindowCapture,
    HdrColorSpace { transfer: HdrTransfer },
}
```

### RdpCapability

RDP-side capability representations:

```rust
pub enum RdpCapability {
    EgfxCodec { avc444: bool, avc420: bool },
    DesktopComposition { multi_mon: bool },
    CursorChannel { metadata: bool },
    ClipboardExtended { file_copy: bool },
    Custom { name: String, version: u32 },
}
```

## Compositor Profiles

### GNOME (Mutter)

```
Services Advertised:
├── DamageTracking: Guaranteed (Portal-based, GNOME 45+)
├── MetadataCursor: Guaranteed
├── MultiMonitor: BestEffort (RestartCaptureOnResize quirk)
├── DmaBufZeroCopy: Unavailable (prefers MemFd)
├── ExplicitSync: Unavailable
└── WindowCapture: Guaranteed
```

### KDE (KWin)

```
Services Advertised:
├── DamageTracking: Guaranteed (Plasma 6+)
├── MetadataCursor: Guaranteed
├── MultiMonitor: Guaranteed
├── DmaBufZeroCopy: Guaranteed
├── ExplicitSync: Guaranteed (Plasma 6+)
└── WindowCapture: Guaranteed
```

### Sway/wlroots

```
Services Advertised:
├── DamageTracking: Guaranteed (native screencopy)
├── MetadataCursor: Degraded (needs explicit composite)
├── MultiMonitor: Guaranteed
├── DmaBufZeroCopy: Guaranteed
├── ExplicitSync: Guaranteed
└── WindowCapture: BestEffort (layer-shell only)
```

## Integration Points

### 1. Session Startup

```rust
// In session initialization
let compositor_caps = probe_capabilities().await?;
let registry = ServiceRegistry::from_compositor(compositor_caps);

// Inject into RDP capability exchange
let rdp_caps = registry.generate_rdp_capabilities();
```

### 2. Frame Processing

```rust
// Adaptive behavior based on services
if registry.service_level(ServiceId::DamageTracking) >= ServiceLevel::BestEffort {
    adaptive_fps.enable_activity_detection();
}

if registry.service_level(ServiceId::DmaBufZeroCopy) == ServiceLevel::Guaranteed {
    encoder.prefer_zero_copy();
}
```

### 3. Feature Negotiation

```rust
// Client requests feature → check registry
fn handle_client_request(&self, feature: ClientFeature) -> FeatureResponse {
    match self.registry.can_provide(feature) {
        ServiceLevel::Guaranteed => FeatureResponse::Granted,
        ServiceLevel::BestEffort => FeatureResponse::GrantedWithWarning,
        ServiceLevel::Degraded => FeatureResponse::GrantedDegraded,
        ServiceLevel::Unavailable => FeatureResponse::Denied,
    }
}
```

## Module Structure

```
src/services/
├── mod.rs                 # Module exports
├── registry.rs            # ServiceRegistry implementation
├── service.rs             # AdvertisedService, ServiceLevel
├── wayland_features.rs    # WaylandFeature enum
├── rdp_capabilities.rs    # RdpCapability enum
├── translation.rs         # Compositor → Services mapping
├── performance.rs         # PerformanceHints
└── compatibility.rs       # FeatureCompatibility matrix
```

## Benefits

1. **Zero Configuration**: Auto-detect and advertise optimal settings
2. **Client Intelligence**: Smart clients can query available features
3. **Graceful Degradation**: Service levels guide fallback behavior
4. **Future-Proof**: New Wayland protocols become new services
5. **Performance Optimization**: Clients select optimal code paths
6. **Debugging**: Clear visibility into what's available and why

## Related Documents

- [Implementation Phases](./SERVICE-ADVERTISEMENT-PHASES.md)
- [Compositor Probing](../src/compositor/mod.rs)
- [Premium Features](./PREMIUM-FEATURES.md)
