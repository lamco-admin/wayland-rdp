# Wayland Innovations Ultrathink - Strategic Enhancement Plan

**Date**: 2025-12-29 22:15 UTC
**Context**: Post-AVC444 success (0.93 MB/s), planning next enhancements
**Strategy**: Two-product approach (RDP server + headless compositor)
**Focus**: Wayland-native innovations, not just RDP feature parity

---

## TWO-PRODUCT STRATEGY (Clear Separation)

### Product A: lamco-rdp-server (Portal-based, Existing Compositors)

**What it is**:
- RDP server that works with GNOME, KDE, Sway, etc.
- Uses XDG Desktop Portal + PipeWire (compositor-agnostic)
- Can't implement new Wayland protocols (not the compositor)
- **Innovation space**: Capture pipeline, encoding, latency, user experience

**Constraints**:
- Limited to what Portal/PipeWire expose
- Can't force compositor changes
- Must work across multiple DEs

**Current status**: ✅ Production-ready (0.93 MB/s AVC444)

---

### Product B: Headless Compositor (Future, Full Control)

**What it would be**:
- Custom Wayland compositor optimized for remote desktop
- Can implement ANY Wayland protocol
- Not bound by Portal limitations
- **Innovation space**: Everything - protocols, GPU pipelines, novel features

**Status**: Not yet started (future product)

**Value**: Technical innovation sandbox, competitive moat

---

## ENHANCEMENT CATEGORY 1: CLIPBOARD (Immediate, Product A)

### DIB/DIBV5 Support (HIGH PRIORITY)

**Current state**: You have text, images (PNG/JPEG), file transfer ✅
**Missing**: Windows DIB/DIBV5 bitmap formats

**Why this matters**:
- Windows clipboard screenshots use DIB format
- Windows Paint, Photoshop use DIBV5
- **Gap**: Windows screenshot → Linux paste doesn't work!

**Research findings**:

**Crate 1**: `clipboard-win` ([docs.rs](https://docs.rs/clipboard-win/latest/clipboard_win/formats/index.html))
- Defines CF_DIB and CF_DIBV5 constants
- Windows-only (won't run on Linux)
- Useful for understanding format specs

**Crate 2**: `dib` ([crates.io](https://crates.io/crates/dib))
- DIB/BMP decoder/encoder
- 137 downloads/month
- MIT license
- **Can use on Linux!**

**Implementation approach**:

**File**: `src/clipboard/image_conversion.rs` (NEW or extend existing)

```rust
use dib::{BitmapInfo, BitmapV5Header};
use image::{DynamicImage, RgbaImage};

/// Convert Windows DIB to PNG
pub fn dib_to_png(dib_data: &[u8]) -> Result<Vec<u8>> {
    // Parse DIB header
    let (header, pixel_data) = parse_dib_header(dib_data)?;

    // Convert BGR(A) to RGBA
    let rgba_data = bgr_to_rgba(pixel_data, header.width, header.height);

    // Create image
    let img = RgbaImage::from_raw(header.width, header.height, rgba_data)?;

    // Encode as PNG
    let mut png_data = Vec::new();
    img.write_to(&mut png_data, image::ImageOutputFormat::Png)?;

    Ok(png_data)
}

/// Convert PNG to Windows DIBV5
pub fn png_to_dibv5(png_data: &[u8]) -> Result<Vec<u8>> {
    // Decode PNG
    let img = image::load_from_memory(png_data)?;

    // Convert to BGRA
    let rgba = img.to_rgba8();
    let bgra = rgba_to_bgra(rgba.as_raw());

    // Build DIBV5 header
    let header = create_dibv5_header(
        img.width(),
        img.height(),
        32, // bits per pixel
    );

    // Combine header + pixel data
    let mut dib = Vec::new();
    dib.extend_from_slice(&header.to_bytes());
    dib.extend_from_slice(&bgra);

    Ok(dib)
}
```

**Effort**: 4-6 hours
**Impact**: Complete clipboard image support (Windows ↔ Linux)
**Priority**: **HIGH** (user-visible gap)

**Action**: Implement this for clipboard completeness

---

## ENHANCEMENT CATEGORY 2: CAPTURE PIPELINE (High ROI, Product A)

### 2.1 Damage-Informed Scheduling (HIGHEST ROI)

**Current**: Damage tracking detects regions, but scheduling is basic
**Enhancement**: Make damage drive the entire capture pipeline

**What we have** (lamco-video):
- ✅ Damage detection (SIMD tile comparison)
- ✅ Backpressure handling
- ✅ Buffer pooling
- 🟡 Basic rate limiting

**What to add**:

**A. Adaptive FPS based on damage**:
```rust
pub struct AdaptiveFpsController {
    target_fps: u32,
    current_fps: u32,
    damage_history: VecDeque<f32>,  // Last N frames damage ratios
}

impl AdaptiveFpsController {
    pub fn adjust_fps(&mut self, damage_ratio: f32) -> u32 {
        // High damage (video, movement) → increase FPS
        if damage_ratio > 0.5 {
            self.current_fps = self.target_fps;  // Full rate
        }
        // Medium damage (scrolling) → moderate FPS
        else if damage_ratio > 0.1 {
            self.current_fps = self.target_fps * 2 / 3;  // 20 fps @ 30 target
        }
        // Low damage (typing, cursor) → low FPS
        else if damage_ratio > 0.01 {
            self.current_fps = self.target_fps / 2;  // 15 fps
        }
        // Static → minimal FPS
        else {
            self.current_fps = 5;  // Just keepalive
        }

        self.current_fps
    }
}
```

**Benefit**: Save CPU/bandwidth when screen static, responsive when active
**Effort**: 8-12 hours
**Impact**: 30-50% CPU reduction for typical desktop use

**B. Damage-Aware Region Priority**:
```rust
pub enum RegionPriority {
    Critical,   // Cursor area, focused window
    High,       // Active windows
    Medium,     // Background windows
    Low,        // Desktop wallpaper, static areas
}

pub struct DamageScheduler {
    priority_map: HashMap<DamageRegion, RegionPriority>,
}

impl DamageScheduler {
    pub fn prioritize_regions(&self, regions: Vec<DamageRegion>) -> Vec<PrioritizedRegion> {
        // Sort by priority + age
        // Encode critical regions first (cursor, text input)
        // Defer low-priority regions if backpressure
    }
}
```

**Benefit**: Cursor/text feels instant, background catches up later
**Effort**: 12-16 hours
**Impact**: Perceived latency reduction (user-visible!)

**C. Latency Governor**:
```rust
pub struct LatencyGovernor {
    mode: LatencyMode,
    budget_ms: f32,
}

pub enum LatencyMode {
    Interactive,  // Minimize latency (gaming, CAD)
    Balanced,     // Balance latency vs quality
    Quality,      // Maximize quality, latency OK
}

impl LatencyGovernor {
    pub fn should_encode_frame(&self, damage: &[DamageRegion], elapsed: f32) -> bool {
        match self.mode {
            LatencyMode::Interactive => {
                // Always encode if ANY damage (low latency)
                !damage.is_empty()
            }
            LatencyMode::Balanced => {
                // Wait for budget or significant damage
                elapsed >= self.budget_ms || total_damage(damage) > 0.05
            }
            LatencyMode::Quality => {
                // Batch multiple changes for better compression
                elapsed >= self.budget_ms * 2.0 || total_damage(damage) > 0.2
            }
        }
    }
}
```

**Benefit**: User can choose latency vs bandwidth tradeoff
**Effort**: 6-8 hours
**Impact**: Professional feature, competitive differentiation

---

### 2.2 Cursor Strategy (High Value, Product A)

**Current**: Cursor mode = "metadata" (client draws cursor)
**Enhancement**: Smart cursor handling based on scenario

**Options**:

**Mode 1: Metadata (current)**:
- Server sends cursor position + shape
- Client draws cursor locally
- **Pro**: Zero latency cursor movement
- **Con**: Requires custom cursor support

**Mode 2: Separate Cursor Stream**:
```rust
pub struct CursorStream {
    position: (i32, i32),
    hotspot: (i32, i32),
    image: Option<Vec<u8>>,  // RGBA cursor image
    last_update: Instant,
}

impl CursorStream {
    pub fn should_update(&self, new_pos: (i32, i32), moved: bool) -> bool {
        // Update if:
        // 1. Cursor moved significantly (>10px)
        // 2. Cursor shape changed
        // 3. Periodic refresh (every 100ms)

        let distance = ((new_pos.0 - self.position.0).pow(2) +
                       (new_pos.1 - self.position.1).pow(2)).sqrt();

        moved && distance > 10.0 || self.last_update.elapsed() > Duration::from_millis(100)
    }
}
```

**Benefit**: Works with all RDP clients, predictable
**Effort**: 8-10 hours
**Impact**: Better compatibility

**Mode 3: Predictive Cursor (innovation!)**:
```rust
pub struct PredictiveCursor {
    velocity: (f32, f32),
    acceleration: (f32, f32),
    history: VecDeque<(i32, i32, Instant)>,
}

impl PredictiveCursor {
    pub fn predict_position(&self, lookahead_ms: f32) -> (i32, i32) {
        // Use velocity + acceleration to predict where cursor will be
        // Send predicted position to client
        // Reduces perceived latency over WAN!

        let dt = lookahead_ms / 1000.0;
        let pred_x = self.position.0 + (self.velocity.0 * dt + 0.5 * self.acceleration.0 * dt * dt);
        let pred_y = self.position.1 + (self.velocity.1 * dt + 0.5 * self.acceleration.1 * dt * dt);

        (pred_x as i32, pred_y as i32)
    }
}
```

**Benefit**: Cursor feels responsive even with 100ms+ network latency
**Effort**: 12-16 hours
**Impact**: **Unique feature** - no other RDP server does this!

---

### 2.3 Capability Probing (Essential, Product A)

**Problem**: Currently assumes Portal capabilities
**Solution**: Runtime detection of compositor features

```rust
pub struct CompositorCapabilities {
    pub name: String,  // "GNOME", "KDE", "Sway"
    pub version: String,

    // Capture capabilities
    pub supports_portal: bool,
    pub supports_wlr_screencopy: bool,
    pub supports_ext_image_copy: bool,

    // Feature capabilities
    pub supports_damage_tracking: bool,
    pub supports_explicit_sync: bool,
    pub supports_color_management: bool,
    pub supports_fractional_scale: bool,

    // Performance hints
    pub preferred_buffer_type: BufferType,  // DMA-BUF, MemFd, SHM
    pub max_resolution: (u32, u32),
}

pub fn probe_compositor() -> Result<CompositorCapabilities> {
    // Connect to Wayland display
    // Enumerate globals in registry
    // Check for protocol versions
    // Return capability profile
}
```

**Usage**:
```rust
let caps = probe_compositor()?;

let capture_backend = if caps.supports_portal {
    CaptureBackend::Portal  // Most compatible
} else if caps.supports_ext_image_copy {
    CaptureBackend::ExtImageCopy  // Modern, performant
} else if caps.supports_wlr_screencopy {
    CaptureBackend::WlrScreencopy  // wlroots family
} else {
    return Err("No supported capture method");
};

// Configure based on capabilities
if caps.supports_damage_tracking {
    enable_compositor_damage_hints();
}
```

**Benefit**: Adapt to each DE automatically, no manual config
**Effort**: 16-20 hours (comprehensive probing)
**Impact**: Professional polish, "just works" on all DEs

---

## ENHANCEMENT CATEGORY 3: EXPLICIT SYNC (Performance, Both Products)

### linux-drm-syncobj-v1 ([Wayland Explorer](https://wayland.app/protocols/linux-drm-syncobj-v1))

**What it is**: Explicit GPU synchronization (acquire/release fences)
**Why it matters**: Prevents tearing, stalls, and race conditions with GPU buffers

**Current**: Likely using implicit sync (or no sync)
**Problem**: DMA-BUF captures can have timing issues, tearing

**Product A (RDP server) approach**:

**Even without protocol access**, design around explicit sync concepts:

```rust
pub struct FrameAcquireFence {
    ready: Arc<AtomicBool>,
    signaled: Instant,
}

pub struct FramePipeline {
    capture_fence: Option<FrameAcquireFence>,
    encode_fence: Option<FrameAcquireFence>,
}

impl FramePipeline {
    pub async fn process_frame(&mut self, buffer: DmaBuf) -> Result<EncodedFrame> {
        // Wait for capture fence (compositor says buffer ready)
        if let Some(fence) = &self.capture_fence {
            fence.wait_ready().await?;
        }

        // Safe to read buffer now
        let frame_data = buffer.map_read()?;

        // Encode
        let encoded = encode(frame_data)?;

        // Signal encode complete (buffer can be reused)
        if let Some(fence) = &mut self.encode_fence {
            fence.signal();
        }

        Ok(encoded)
    }
}
```

**Benefit**: No tearing, predictable performance
**Effort**: 8-12 hours (fence abstraction)
**Impact**: Eliminates visual artifacts with GPU capture

**Product B (headless) approach**:

**Implement linux-drm-syncobj-v1 protocol directly**:
- Full explicit sync support
- Clients can submit timeline semaphores
- GPU operations properly ordered
- Reference implementation quality

**Effort**: 24-32 hours
**Impact**: Technical showcase, enables advanced GPU features

---

## ENHANCEMENT CATEGORY 4: COLOR MANAGEMENT (Quality, Both Products)

### color-management-v1 ([Wayland Explorer](https://wayland.app/protocols/color-management-v1))

**What it is**: Proper color space, transfer function, gamut handling
**Why it matters**: Remote desktop has silent color lies (sRGB assumed everywhere)

**Current state**:
- ✅ You have BT.601/BT.709/sRGB color space config
- ✅ VUI parameters in H.264
- 🟡 Assumes sRGB on both sides

**Product A enhancements**:

**A. Track source colorspace when available**:
```rust
pub struct FrameColorInfo {
    pub primaries: ColorPrimaries,      // BT.709, P3, BT.2020
    pub transfer: TransferFunction,     // sRGB, PQ (HDR), HLG
    pub range: ColorRange,              // Full, Limited
}

pub fn get_frame_color_info(frame: &VideoFrame) -> FrameColorInfo {
    // Check PipeWire metadata for color info
    // Fall back to compositor detection (GNOME usually sRGB)
    // Embed in H.264 VUI
}
```

**B. Fidelity modes**:
```rust
pub enum ColorFidelityMode {
    /// Pixel-perfect color (high bandwidth)
    /// - Full range YUV
    /// - High bitrate
    /// - Lossless where possible
    PixelPerfect,

    /// Balanced (production default)
    /// - Standard range
    /// - Optimized bitrate
    /// - Perceptually lossless
    Balanced,

    /// Bandwidth-saver
    /// - Aggressive compression
    /// - Acceptable quality
    /// - Maximum efficiency
    Efficient,
}
```

**Benefit**: "Most correct SDR" RDP implementation
**Effort**: 8-12 hours
**Impact**: Professional graphics work, photography use cases

**Product B approach**:

**Implement color-management-v1 fully**:
- Support HDR content (BT.2020, PQ transfer)
- Proper gamut mapping
- ICC profile support
- Color-managed compositing

**Then**: Design transport for HDR over RDP (or custom protocol)

**Effort**: 40+ hours
**Impact**: Industry-leading color accuracy

---

## ENHANCEMENT CATEGORY 5: FRACTIONAL SCALING (UX, Product A)

### fractional-scale-v1 ([Wayland Explorer](https://wayland.app/protocols/fractional-scale-v1))

**What it is**: Proper HiDPI support (1.25x, 1.5x, 1.75x scales)
**Why it matters**: Remote desktop + HiDPI is often blurry or wrong DPI

**Current**: Assumes 1.0 scale
**Enhancement**: Detect and handle fractional scaling

**Implementation**:

```rust
pub struct ScaleInfo {
    pub server_scale: f64,   // Source compositor scale
    pub client_scale: f64,   // RDP client DPI scale
    pub strategy: ScalingStrategy,
}

pub enum ScalingStrategy {
    /// Server-side scaling (encode at client DPI)
    ServerScale,

    /// Client-side scaling (encode at server DPI, client scales)
    ClientScale,

    /// Hybrid (encode at optimal intermediate)
    Hybrid,
}

pub fn calculate_optimal_encoding_resolution(
    source: (u32, u32),
    source_scale: f64,
    client_dpi: u32,
) -> (u32, u32) {
    // Calculate logical size
    let logical_w = (source.0 as f64 / source_scale) as u32;
    let logical_h = (source.1 as f64 / source_scale) as u32;

    // Scale to client DPI
    let client_scale = client_dpi as f64 / 96.0;
    let target_w = (logical_w as f64 * client_scale) as u32;
    let target_h = (logical_h as f64 * client_scale) as u32;

    // Align to 16 (H.264 requirement)
    (align_to_16(target_w), align_to_16(target_h))
}
```

**Benefit**: Sharp text on HiDPI displays, correct sizing
**Effort**: 12-16 hours
**Impact**: Better UX for HiDPI users (common!)

---

## ENHANCEMENT CATEGORY 6: MODERN CAPTURE (Future, Product B)

### ext-image-copy-capture-v1 ([Wayland Explorer](https://wayland.app/protocols/ext-image-copy-capture-v1))

**Status**: Implemented in COSMIC, Sway, wlroots, Wayfire, NOT in GNOME/KDE
**Why**: GNOME/KDE prefer Portal (ecosystem consistency)

**Product A (RDP server)**:
- **Don't prioritize** - KDE won't implement it
- Portal is the right path for existing compositors

**Product B (headless)**:
- **DO implement** - it's the modern standard
- Advantages over Portal:
  - Direct control over buffers
  - Explicit cursor capture
  - Better damage signaling
  - Lower overhead

**Implementation notes**:
```rust
// ext_image_copy_capture_v1 provides:
struct ImageCaptureSource {
    source_type: SourceType,  // Output, Toplevel, Region
    damage: Option<Region>,   // Compositor damage hints!
}

struct ImageCopyCapture {
    buffer: WlBuffer,         // Client-allocated buffer
    damage: Vec<Rectangle>,   // What changed
    cursor_info: Option<CursorInfo>,  // Cursor state
}
```

**Benefit**: Reference implementation, technical showcase
**Effort**: 32-40 hours (full protocol implementation)
**Impact**: Establishes technical leadership

---

## PRIORITY ROADMAP

### Phase 1: Clipboard Completeness (Week 1)

**Immediate value, user-visible**:
1. DIB/DIBV5 conversion (4-6 hours)
2. Test Windows screenshot → Linux paste
3. Test Linux → Windows bitmap paste

**Deliverable**: Complete clipboard image support

---

### Phase 2: Capture Pipeline Excellence (Week 2-3)

**lamco-video enhancements**:
1. Adaptive FPS controller (8-12 hours)
2. Damage-aware scheduling (12-16 hours)
3. Latency governor modes (6-8 hours)

**Deliverable**: Industry-leading capture pipeline

---

### Phase 3: Professional Polish (Week 4-5)

**lamco-rdp-server enhancements**:
1. Capability probing (16-20 hours)
2. Cursor strategy options (8-10 hours)
3. Fractional scale handling (12-16 hours)
4. Color fidelity modes (8-12 hours)

**Deliverable**: Professional-grade RDP server

---

### Phase 4: Headless Compositor R&D (Future)

**When to start**: After Product A is shipped and mature

**What to build**:
1. Basic headless compositor (40-60 hours)
2. ext-image-copy-capture-v1 (32-40 hours)
3. linux-drm-syncobj-v1 (24-32 hours)
4. color-management-v1 (40+ hours)
5. Custom RDP-optimized features

**Deliverable**: Technical innovation platform

---

## COMPETITIVE DIFFERENTIATION

### What Makes You Unique

**Current (0.93 MB/s AVC444)**:
- ✅ Only Wayland-native RDP server with AVC444
- ✅ Competitive bandwidth
- ✅ Modern tech stack (Rust)

**With Phase 1-2 enhancements**:
- ➕ **Best clipboard** (complete format support)
- ➕ **Smartest capture pipeline** (damage-driven, adaptive)
- ➕ **Lowest latency** (governor modes, prioritization)

**With Phase 3**:
- ➕ **Most compatible** (works on all DEs automatically)
- ➕ **Best color accuracy** (fidelity modes)
- ➕ **Best HiDPI** (fractional scale handling)

**With Phase 4** (headless):
- ➕ **Technical leader** (reference Wayland implementations)
- ➕ **Innovation platform** (can experiment with protocols)
- ➕ **Future-proof** (not limited by Portal)

---

## IMPLEMENTATION PRIORITIES (My Recommendation)

### Immediate (This Week)

**1. DIB/DIBV5 clipboard** (4-6 hours):
- High user value
- Completes existing feature
- Low complexity

**2. Adaptive FPS** (8-12 hours):
- Big performance win
- Builds on existing damage tracking
- User-visible improvement

**Total**: 12-18 hours = 2-3 days

### Next Sprint (Week 2)

**3. Latency governor** (6-8 hours):
- Professional feature
- Competitive differentiation

**4. Cursor strategy** (8-10 hours):
- Better UX
- Compatibility improvement

**5. Capability probing** (16-20 hours):
- "Just works" on all DEs
- Reduces support burden

**Total**: 30-38 hours = 4-5 days

### Future Consideration

**6. Headless compositor**: When Product A is mature and shipped

---

## TECHNICAL NOTES

### Why NOT Prioritize Some Protocols

**zwlr_screencopy_manager_v1**:
- Wlroots-specific
- Portal is more universal
- **Skip for Product A**, consider for Product B

**ext-image-copy-capture-v1**:
- KDE won't implement (prefer Portal)
- GNOME won't implement (prefer Portal)
- **Skip for Product A**, implement in Product B

**Rationale**: Portal is the ecosystem-blessed path for existing compositors

### Why YES on Some Features

**Damage-informed scheduling**:
- Works with Portal
- Huge performance benefit
- Differentiating feature

**Cursor strategies**:
- Works within RDP protocol
- No compositor changes needed
- Immediate value

**Color fidelity**:
- Can work with what Portal provides
- Differentiates from competition
- Professional use cases

---

## SOURCES

**DIB Conversion**:
- [clipboard-win crate](https://docs.rs/clipboard-win/latest/clipboard_win/formats/index.html)
- [dib crate](https://crates.io/crates/dib)

**Wayland Protocols**:
- [ext-image-copy-capture-v1](https://wayland.app/protocols/ext-image-copy-capture-v1)
- [linux-drm-syncobj-v1](https://wayland.app/protocols/linux-drm-syncobj-v1)
- [color-management-v1](https://wayland.app/protocols/color-management-v1)
- [fractional-scale-v1](https://wayland.app/protocols/fractional-scale-v1)

---

## MY RECOMMENDATION

**Start with DIB clipboard + Adaptive FPS** (this week):
- Completes clipboard (user-visible)
- Improves performance (user-visible)
- Both are high ROI, low complexity
- Builds on what you have

**Then capability probing + latency modes** (next sprint):
- Professional features
- Competitive differentiation
- "Just works" quality

**Defer headless compositor** until Product A ships:
- Long-term innovation platform
- Requires significant investment
- Build when you have revenue/resources

**Sound good?** Should I implement DIB clipboard + Adaptive FPS first?
