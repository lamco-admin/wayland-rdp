# H.264 Optimization Strategies for EGFX - Comprehensive Analysis

**Date:** 2025-12-24
**Context:** EGFX H.264 streaming - level constraints, compression, dirty regions, framerate regulation

---

## Current Issue: H.264 Level Constraint Violation

**Configuration:**
- Resolution: 1280×800 (4,000 macroblocks per frame)
- Framerate: 30 fps
- OpenH264 auto-selected: Level 3.2

**Problem:**
- Level 3.2 max: 108,000 macroblocks/second
- Our stream: 120,000 macroblocks/second
- **Exceeds spec by 11%**

**Symptom:**
- Windows MFT decoder silently rejects stream (no frame ACKs)
- Backpressure stuck at 3 frames
- Connection eventually drops

---

## H.264 Levels - Complete Reference

### Level Definitions (ITU-T H.264 Annex A)

| Level | Max MB/s | Max Frame Size (MBs) | Common Resolutions @ Max FPS |
|-------|----------|---------------------|------------------------------|
| **1.0** | 1,485 | 99 | QCIF (176×144) @ 15 fps |
| **1.1** | 3,000 | 396 | CIF (352×288) @ 7.5 fps |
| **1.2** | 6,000 | 396 | CIF @ 15 fps |
| **1.3** | 11,880 | 396 | CIF @ 30 fps |
| **2.0** | 11,880 | 396 | CIF @ 30 fps |
| **2.1** | 19,800 | 792 | CIF @ 50 fps, 352×480 @ 30 fps |
| **2.2** | 20,250 | 1,620 | SD (640×480) @ 15 fps |
| **3.0** | 40,500 | 1,620 | SD @ 30 fps |
| **3.1** | 108,000 | 3,600 | 720p (1280×720) @ 30 fps |
| **3.2** | 216,000* | 5,120 | 720p @ 60 fps, 1280×1024 @ 42 fps |
| **4.0** | 245,760 | 8,192 | 1080p (1920×1080) @ 30 fps |
| **4.1** | 245,760 | 8,192 | 1080p @ 30 fps |
| **4.2** | 522,240 | 8,704 | 1080p @ 60 fps |
| **5.0** | 589,824 | 22,080 | 1080p @ 72 fps, 2K @ 30 fps |
| **5.1** | 983,040 | 36,864 | 4K (3840×2160) @ 30 fps |
| **5.2** | 2,073,600 | 36,864 | 4K @ 60 fps |
| **6.0** | 4,177,920 | 139,264 | 8K (7680×4320) @ 30 fps |

*Note: Level 3.2 has special handling - 216,000 MB/s if frame size ≤ 1,620 MBs, else 108,000 MB/s

### Resolution to Level Mapping

| Resolution | Macroblocks | Level for 30 fps | Level for 60 fps |
|------------|-------------|------------------|------------------|
| 640×480 (VGA) | 1,200 | 3.0 | 3.1 |
| 800×600 (SVGA) | 1,875 | 3.1 | 3.2 |
| 1024×768 (XGA) | 3,072 | 3.1 | 4.0 |
| **1280×720 (720p)** | 3,600 | **3.1** ✅ | 3.2 |
| **1280×800 (WXGA)** | 4,000 | **4.0** ⚠️ | 4.2 |
| **1280×1024 (SXGA)** | 5,120 | **4.0** | 4.2 |
| 1920×1080 (1080p) | 8,100 | 4.0 | 4.2 |
| 1920×1200 (WUXGA) | 9,000 | 4.1 | 4.2 |
| 2560×1440 (1440p) | 14,400 | 5.0 | 5.1 |
| 3840×2160 (4K) | 32,400 | 5.1 | 5.2 |

**Key Insight:** 1280×800 @ 30fps requires Level 4.0, not Level 3.2!

---

## Optimization Strategies

### 1. ZGFX Compression (Currently Missing?)

**What it is:**
ZGFX is the compression algorithm specified in MS-RDPEGFX for compressing EGFX PDUs before transmission.

**Implementation Status:**
Let me check if we're using it...

```rust
// FreeRDP: channels/rdpgfx/server/rdpgfx_main.c:145
zgfx_compress_to_stream(context->priv->zgfx, fs, pSrcData, (UINT32)SrcSize, &flags)
```

**Question:** Does IronRDP's EGFX implementation use ZGFX compression?

**Impact:**
- Reduces bandwidth by 2-10x for typical screen content
- Doesn't affect level constraints (encoder still produces same MB/s)
- Helps with network transmission, not decoder constraints

---

### 2. Dirty Region Tracking / Damage Rectangles

**What it is:**
Only encode regions of the screen that have changed, not the full frame.

**Current Implementation:**
```rust
// src/server/egfx_sender.rs:249
let regions = vec![Avc420Region::full_frame(width, height, 22)];
```

We're currently encoding the **entire frame** every time, even if only a small region changed.

**Proper Implementation:**

```rust
// Pseudo-code for damage-aware encoding
fn send_damaged_regions(damage_rects: &[DamageRect], frame_data: &[u8]) {
    let regions: Vec<Avc420Region> = damage_rects
        .iter()
        .map(|rect| {
            // Extract sub-region from frame
            // Encode only that region with H.264
            Avc420Region::new(rect.left, rect.top, rect.right, rect.bottom, qp, quality)
        })
        .collect();

    // Each region can have its own H.264 bitstream
    // Or combine into multi-region frame
}
```

**Benefits:**
- Reduces encoded data size dramatically for partial screen updates
- Lower bitrate = less network traffic
- Faster encoding (smaller regions)
- **CRITICAL:** Can reduce effective MB/s for level constraints
  - Full frame: 4,000 MBs × 30 fps = 120,000 MB/s
  - 25% damaged: 1,000 MBs × 30 fps = 30,000 MB/s ✅ Well within Level 3.1

**Implementation Requirements:**
1. PipeWire damage tracking (might already have via `damage_tracking: true` in config)
2. Sub-frame H.264 encoding (encode arbitrary rectangles)
3. Multi-region RFX_AVC420_BITMAP_STREAM support
4. Coordinate dirty rects with encoder

**Note:** Our config already has:
```toml
damage_tracking: true
damage_threshold: 0.05
```

Need to verify if this is being used for EGFX path.

---

### 3. Framerate Regulation

**Current Approach:**
```rust
// src/server/display_handler.rs - hardcoded 30 fps
let timestamp_ms = (frames_sent * 33) as u64;  // ~30fps timing
```

**Proper Dynamic Regulation:**

```rust
pub struct LevelConstraintCalculator {
    resolution_mbs: u32,
}

impl LevelConstraintCalculator {
    fn calculate_max_fps(&self, h264_level: H264Level) -> f32 {
        let max_mbs_per_sec = h264_level.max_macroblocks_per_second();
        (max_mbs_per_sec as f32) / (self.resolution_mbs as f32)
    }

    fn recommend_level(&self, target_fps: f32) -> H264Level {
        let required_mbs_per_sec = (self.resolution_mbs as f32) * target_fps;
        H264Level::from_macroblock_rate(required_mbs_per_sec)
    }
}

// Example:
let calc = LevelConstraintCalculator { resolution_mbs: 4000 };  // 1280×800
calc.calculate_max_fps(H264Level::L3_1) // 108,000 / 4,000 = 27 fps
calc.calculate_max_fps(H264Level::L4_0) // 245,760 / 4,000 = 61.4 fps
calc.recommend_level(30.0)               // Needs Level 4.0
```

**Adaptive Framerate:**

```rust
pub struct AdaptiveFramerate {
    target_fps: f32,
    level_constraint_fps: f32,
    network_constraint_fps: f32,

    current_fps: f32,
    frame_interval_ms: u64,
}

impl AdaptiveFramerate {
    fn adjust(&mut self) {
        // Use minimum of all constraints
        self.current_fps = self.target_fps
            .min(self.level_constraint_fps)
            .min(self.network_constraint_fps);

        self.frame_interval_ms = ((1000.0 / self.current_fps) as u64).max(1);
    }

    fn get_frame_interval(&self) -> u64 {
        self.frame_interval_ms
    }
}
```

---

### 4. H.264 Level Configuration in OpenH264

**Current Issue:**
The Rust `openh264` crate doesn't expose level configuration in `EncoderConfig`.

**Solutions:**

#### Option A: Use OpenH264 C API directly (via openh264-sys2)

```rust
use openh264_sys2::*;

unsafe {
    let mut params: SEncParamExt = std::mem::zeroed();
    (*encoder).GetDefaultParams(&mut params);

    // Set profile and level
    params.iTargetBitrate = 5_000_000;
    params.iUsageType = SCREEN_CONTENT_REAL_TIME;
    params.bEnableFrameSkip = 1;
    params.uiMaxFramerate = 30;

    // CRITICAL: Set level constraint
    params.iLevelIdc = LEVEL_4_0;  // or AUTO_LEVEL
    // Or set dimensions and let it auto-calculate:
    params.iPicWidth = 1280;
    params.iPicHeight = 800;
    params.fMaxFrameRate = 30.0;

    (*encoder).InitializeExt(&mut params);
}
```

#### Option B: Fork openh264 crate to add level configuration

```rust
// Add to openh264/src/encoder.rs
pub fn set_level(mut self, level: H264Level) -> Self {
    self.level = Some(level);
    self
}

pub enum H264Level {
    Level3_0 = 30,
    Level3_1 = 31,
    Level3_2 = 32,
    Level4_0 = 40,
    Level4_1 = 41,
    Level4_2 = 42,
    Auto = -1,
}
```

#### Option C: Configure via dimensions hinting

OpenH264 might auto-select correct level if we set proper dimension constraints upfront.

---

### 5. Multi-Resolution Support Strategy

**Product Requirements:**
- Support 720p, 1080p, 1440p, 4K
- Support 24/30/60 fps
- Always stay within spec

**Configuration Matrix:**

```rust
pub struct ResolutionConfig {
    width: u16,
    height: u16,
    macroblocks: u32,
    recommended_level: H264Level,
    max_fps_by_level: HashMap<H264Level, f32>,
}

const SUPPORTED_CONFIGS: &[ResolutionConfig] = &[
    ResolutionConfig {
        width: 1280,
        height: 720,
        macroblocks: 3600,
        recommended_level: H264Level::L3_1,
        max_fps_by_level: hashmap! {
            H264Level::L3_0 => 11.25,
            H264Level::L3_1 => 30.0,
            H264Level::L3_2 => 60.0,
        },
    },
    ResolutionConfig {
        width: 1280,
        height: 800,
        macroblocks: 4000,
        recommended_level: H264Level::L4_0,  // REQUIRED for 30fps!
        max_fps_by_level: hashmap! {
            H264Level::L3_1 => 27.0,   // Just within limit
            H264Level::L3_2 => 27.0,   // Still constrained by >1620 MB rule
            H264Level::L4_0 => 61.4,   // Recommended
        },
    },
    ResolutionConfig {
        width: 1920,
        height: 1080,
        macroblocks: 8100,
        recommended_level: H264Level::L4_0,
        max_fps_by_level: hashmap! {
            H264Level::L4_0 => 30.3,
            H264Level::L4_1 => 30.3,
            H264Level::L4_2 => 64.5,
        },
    },
];
```

---

## Detailed Mitigation Strategies

### Strategy 1: ZGFX Compression

**Current Status: UNKNOWN - Need to verify IronRDP implementation**

**Check:**
```rust
// Does IronRDP's ServerEvent::Egfx path apply ZGFX?
// File: ironrdp-server/src/server.rs
EgfxServerMessage::SendMessages { channel_id, messages } => {
    // Are messages compressed here?
}
```

**FreeRDP Implementation:**
```c
// Compresses EGFX PDUs before sending
zgfx_compress_to_stream(context->priv->zgfx, fs, pSrcData, SrcSize, &flags)
```

**Action Required:**
1. Verify if IronRDP applies ZGFX compression in EGFX path
2. If not, this is a separate bug that needs fixing
3. ZGFX is REQUIRED per MS-RDPEGFX spec for efficient transmission

---

### Strategy 2: Dirty Region / Damage Rectangle Tracking

**Current Implementation:**
```rust
// src/server/egfx_sender.rs:249
let regions = vec![Avc420Region::full_frame(width, height, 22)];
```

**Problem:** Always encodes full frame, even for cursor movement.

**Proper Implementation:**

#### Step 1: Extract Damage Info from PipeWire

```rust
// PipeWire provides damage rectangles via metadata
// Need to extract pw_buffer damage regions

struct DamageRegion {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

// From PipeWire buffer metadata:
fn extract_damage_regions(buffer: &PwBuffer) -> Vec<DamageRegion> {
    // spa_meta_region provides damage rectangles
    // See: PipeWire spa/buffer/buffer.h
}
```

#### Step 2: Encode Only Damaged Regions

```rust
pub async fn send_frame_with_damage(
    &self,
    frame_data: &[u8],
    width: u16,
    height: u16,
    damage_rects: &[DamageRegion],
) -> SendResult<u32> {
    if damage_rects.is_empty() {
        // No damage = no encoding needed
        return Ok(0);
    }

    // Option A: Encode each damaged region separately
    for rect in damage_rects {
        let sub_frame = extract_sub_frame(frame_data, width, height, rect);
        let h264_data = encoder.encode_region(&sub_frame, rect.width, rect.height)?;
        let region = Avc420Region::new(
            rect.x,
            rect.y,
            rect.x + rect.width - 1,
            rect.y + rect.height - 1,
            22,  // qp
            100  // quality
        );
        // Send multi-region frame
    }

    // Option B: Merge nearby rects and encode larger region
    let merged = merge_nearby_rects(damage_rects, threshold = 64);
    // ... encode merged regions
}
```

#### Step 3: Configure Damage Threshold

```toml
# config.toml
[video]
damage_tracking = true
damage_threshold = 0.05  # 5% of screen must change to send update
min_damage_area = 256    # Minimum pixels to encode (skip tiny updates)
merge_distance = 64      # Merge rects within 64 pixels
```

**Effectiveness:**
- Typical office work: 5-20% screen damage per frame
  - 4,000 MBs × 20% = 800 MBs/frame
  - 800 MBs × 30 fps = 24,000 MB/s ← **Well within any level!**
- Video playback region: Still high, but localized
- Full screen changes: Falls back to full frame encoding

---

### Strategy 3: Adaptive Quality (QP) Adjustment

**Current:**
```rust
Avc420Region::full_frame(width, height, 22)  // Fixed QP=22
```

**Adaptive Approach:**

```rust
pub struct QualityController {
    target_bitrate: u32,
    current_bitrate: u32,
    qp_min: u8,  // 10 = high quality
    qp_max: u8,  // 40 = low quality
    current_qp: u8,
}

impl QualityController {
    fn adjust_qp(&mut self, backpressure_level: f32, network_congestion: f32) -> u8 {
        if backpressure_level > 0.8 || self.current_bitrate > self.target_bitrate * 1.2 {
            // Increase QP = lower quality = smaller frames
            self.current_qp = (self.current_qp + 2).min(self.qp_max);
        } else if backpressure_level < 0.3 && self.current_bitrate < self.target_bitrate * 0.8 {
            // Decrease QP = higher quality
            self.current_qp = (self.current_qp.saturating_sub(1)).max(self.qp_min);
        }
        self.current_qp
    }
}
```

**Impact:**
- Lower quality = smaller encoded size
- Doesn't directly affect MB/s (frame dimensions unchanged)
- Helps with bitrate and network constraints
- Can enable higher framerates at lower quality

---

### Strategy 4: Proper Level Selection & Framerate Regulation

**Dynamic Level Selection:**

```rust
pub struct H264LevelManager {
    resolution_mbs: u32,
    target_fps: f32,
}

impl H264LevelManager {
    pub fn select_level(&self) -> (H264Level, f32) {
        let required_mbs_per_sec = self.resolution_mbs as f32 * self.target_fps;

        // Try to meet target FPS with lowest level
        let levels = [
            (H264Level::L3_0, 40_500),
            (H264Level::L3_1, 108_000),
            (H264Level::L3_2, self.level_3_2_limit()),
            (H264Level::L4_0, 245_760),
            (H264Level::L4_1, 245_760),
            (H264Level::L4_2, 522_240),
            (H264Level::L5_0, 589_824),
        ];

        for (level, max_mbs_per_sec) in levels {
            if required_mbs_per_sec <= max_mbs_per_sec as f32 {
                return (level, self.target_fps);
            }
        }

        // Can't meet target FPS - reduce FPS to fit highest level
        let max_fps = (levels.last().unwrap().1 as f32) / (self.resolution_mbs as f32);
        (H264Level::L5_0, max_fps)
    }

    fn level_3_2_limit(&self) -> u32 {
        // Special handling for Level 3.2
        if self.resolution_mbs <= 1620 {
            216_000
        } else {
            108_000
        }
    }
}
```

**Framerate Regulator:**

```rust
pub struct FramerateRegulator {
    target_interval_ms: u64,
    last_frame_time: Instant,
    fps_stats: RollingAverage,
}

impl FramerateRegulator {
    pub fn should_send_frame(&mut self) -> bool {
        let elapsed = self.last_frame_time.elapsed();
        if elapsed.as_millis() >= self.target_interval_ms as u128 {
            self.last_frame_time = Instant::now();
            true
        } else {
            false
        }
    }

    pub fn set_target_fps(&mut self, fps: f32) {
        self.target_interval_ms = ((1000.0 / fps) as u64).max(1);
    }
}
```

---

### Strategy 5: Resolution-Aware Configuration

**Configuration Schema:**

```toml
# config.toml
[video.h264]
# Auto-select level based on resolution and framerate
level_selection = "auto"  # or explicit: "3.1", "4.0", "4.2"

# Target framerate (may be reduced to meet level constraints)
target_fps = 30
allow_fps_reduction = true  # Reduce FPS rather than fail

# Quality settings
qp_default = 22
qp_min = 10   # Best quality
qp_max = 40   # Lowest quality
adaptive_quality = true

[[video.h264.resolution_profiles]]
max_width = 1280
max_height = 720
level = "3.1"
max_fps = 30
recommended_bitrate = 4000

[[video.h264.resolution_profiles]]
max_width = 1280
max_height = 800
level = "4.0"          # REQUIRED for 30fps
max_fps = 30
recommended_bitrate = 5000

[[video.h264.resolution_profiles]]
max_width = 1920
max_height = 1080
level = "4.0"
max_fps = 30
recommended_bitrate = 8000

[[video.h264.resolution_profiles]]
max_width = 1920
max_height = 1080
level = "4.2"
max_fps = 60
recommended_bitrate = 15000
```

---

## Immediate Action Plan

### Priority 1: Fix Level Constraint (Choose One)

**Option A: Reduce framerate to 27 fps (Quick Fix)**
```rust
// src/server/display_handler.rs
let timestamp_ms = (frames_sent * 37) as u64;  // 27 fps (1000/37 ≈ 27)
```
- Pros: Simple, stays within Level 3.2
- Cons: Slightly choppy, not a proper solution

**Option B: Configure OpenH264 for Level 4.0 (Proper Fix)**
- Requires accessing C API or forking openh264 crate
- Pros: Meets spec, supports 30fps properly
- Cons: More implementation work

**Option C: Test at 1280×720 resolution (Validation)**
- Confirms level constraints are the issue
- 3,600 MBs @ 30fps = 108,000 MB/s (exactly Level 3.1 limit)

### Priority 2: Verify ZGFX Compression

Check if IronRDP applies ZGFX in EGFX message path.

### Priority 3: Implement Damage-Aware Encoding

Extract PipeWire damage regions and encode only changed areas.

### Priority 4: Build Proper Level/FPS Management System

Dynamic level selection and framerate regulation based on resolution.

---

## Questions to Investigate

1. ✅ Is RFX_RECT bounds or x/y/w/h? → **BOUNDS (left,top,right,bottom)**
2. ❓ Does IronRDP use ZGFX compression for EGFX?
3. ❓ Can we access OpenH264 level configuration?
4. ❓ Does PipeWire provide damage rectangles we can use?
5. ❓ What's the proper way to encode multi-region frames in AVC420?

---

## Next Steps

1. Verify ZGFX compression implementation in IronRDP
2. Test framerate reduction to 27 fps as immediate workaround
3. Investigate OpenH264 C API for Level 4.0 configuration
4. Design and implement LevelConstraintCalculator
5. Implement damage-aware region encoding
6. Create comprehensive configuration system for resolution/level/fps combinations
