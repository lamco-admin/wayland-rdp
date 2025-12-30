# EGFX H.264/RemoteFX Comprehensive Analysis

**Date:** 2025-12-24
**Purpose:** Answer all optimization and configuration questions for production-ready EGFX implementation

---

## Question 1: Previous PDU Fix - What Did We Actually Change?

### Answer: We Haven't Changed the Bounds Format Yet

**Original IronRDP Code (commit 415ff81a):**
```rust
// crates/ironrdp-egfx/src/pdu/avc.rs:89
rectangle.encode(dst)?;
// Calls InclusiveRectangle::encode() → writes left, top, right, bottom
```

**Current Working Directory (uncommitted):**
```rust
// Inline expansion (functionally identical):
dst.write_u16(rectangle.left);
dst.write_u16(rectangle.top);
dst.write_u16(rectangle.right);
dst.write_u16(rectangle.bottom);
```

**Status:** The encoding has **ALWAYS been bounds-based** in IronRDP. My change just made it explicit rather than calling `.encode()`.

**Previous Session Summary Confusion:**
The summary mentioned "Fix RFX_RECT encoding (width/height not right/bottom)" but there's NO commit in IronRDP git history for this. This was likely:
- A local change that was tested but not committed
- Or a misunderstanding in the summary

**Conclusion:** The bounds format (left,top,right,bottom) is correct and has been the implementation all along.

---

## Question 2: Is ZGFX Compression Implemented?

### Answer: NO - Critical Missing Feature!

**Status Check:**
- ✅ IronRDP has `zgfx::Decompressor` (client-side)
- ❌ IronRDP does **NOT** have `zgfx::Compressor` (server-side)
- ❌ ServerEvent::Egfx path sends **uncompressed** data

**Evidence:**
```bash
$ find IronRDP/crates -name "*.rs" -exec grep -l "Compressor" {} \;
# No ZGFX compressor found
```

**FreeRDP Implementation:**
```c
// channels/rdpgfx/server/rdpgfx_main.c:145
zgfx_compress_to_stream(context->priv->zgfx, fs, pSrcData, SrcSize, &flags)
```

**Impact:**
- Our 85KB H.264 frames are sent uncompressed
- ZGFX could compress them 2-10x (depending on content)
- Network bandwidth waste
- **MS-RDPEGFX requires ZGFX compression** - we're violating spec!

**Action Required:**
1. Implement `zgfx::Compressor` in ironrdp-graphics
2. Apply compression in ServerEvent::Egfx handling
3. This is a separate issue from H.264 level constraints
4. **File IronRDP PR** to add server-side ZGFX compression

---

## Question 3: Dirty Region Tracking (Damage Rectangles)

### Current Implementation

**What We Have:**
```rust
// config.toml
[video]
damage_tracking = true
damage_threshold = 0.05
```

**What We're Actually Doing:**
```rust
// src/server/egfx_sender.rs:249
let regions = vec![Avc420Region::full_frame(width, height, 22)];
//                   ^^^^^^^^^ Always full frame!
```

**Problem:** We have damage tracking CONFIG but we're not USING it for EGFX!

### How Damage Tracking Should Work

#### PipeWire Damage Regions

PipeWire provides damage rectangles through buffer metadata:

```c
// From PipeWire spa/buffer/buffer.h
struct spa_meta_region {
    struct spa_region region;
};

struct spa_region {
    uint32_t size;
    struct spa_rectangle rectangles[];
};

struct spa_rectangle {
    int32_t x, y, width, height;
};
```

**Our Code:**
Check if `lamco-pipewire` extracts damage regions from buffers.

#### Multi-Region Encoding

**RFX_AVC420_BITMAP_STREAM supports multiple regions:**

```rust
pub fn send_damaged_frame(
    &self,
    frame_data: &[u8],
    width: u16,
    height: u16,
    damage_rects: &[DamageRect],
) -> SendResult<u32> {
    // Create one Avc420Region per damaged rectangle
    let mut regions = Vec::new();
    let mut h264_data = Vec::new();

    for damage in damage_rects {
        // Extract sub-frame pixels
        let sub_frame = extract_rectangle(frame_data, width, damage);

        // Encode ONLY this region with H.264
        let region_h264 = encoder.encode_region(
            &sub_frame,
            damage.width,
            damage.height
        )?;

        regions.push(Avc420Region::new(
            damage.x,
            damage.y,
            damage.x + damage.width - 1,
            damage.y + damage.height - 1,
            22,  // qp
            100  // quality
        ));

        h264_data.extend_from_slice(&region_h264);
    }

    // Send frame with multiple regions
    server.send_avc420_frame(surface_id, &h264_data, &regions, timestamp_ms)
}
```

**Benefits:**
- Small cursor movement: Encode 32×32 region instead of 1280×800
  - Full frame: 4,000 MBs
  - Cursor region: 2 MBs
  - **2000x reduction in MB/s!**
- Typical office work: 5-20% screen damage
  - 4,000 MBs × 15% = 600 MBs/frame
  - 600 MBs × 30 fps = 18,000 MB/s ← **Fits in Level 3.0!**

**Complexity:**
- Need to encode multiple H.264 streams (one per region)
- OR merge nearby damaged rects into larger regions
- Coordinate with encoder (may need separate encoder instances)

---

## Question 4: H.264 Levels - Complete Understanding

### What Are H.264 Levels?

H.264 levels define **decoder capability constraints**:
- Maximum macroblocks per second (processing power)
- Maximum frame size (memory)
- Maximum bitrate
- Maximum DPB (decoded picture buffer) size

Think of levels as **hardware capability tiers**.

### Level Parameters Table

| Level | Max MB/s | Max Frame MBs | Max DPB (frames) | Max Bitrate (Main) | Target Devices |
|-------|----------|---------------|------------------|-------------------|----------------|
| 3.0 | 40,500 | 1,620 | 6 | 10 Mbps | Mobile devices, tablets |
| 3.1 | 108,000 | 3,600 | 6 | 14 Mbps | 720p capable devices |
| 3.2 | 216,000† | 5,120 | 7 | 20 Mbps | HD displays |
| 4.0 | 245,760 | 8,192 | 12 | 25 Mbps | 1080p displays, most modern devices |
| 4.1 | 245,760 | 8,192 | 13 | 50 Mbps | Blu-ray, professional |
| 4.2 | 522,240 | 8,704 | 13 | 50 Mbps | 1080p @ 60fps |
| 5.0 | 589,824 | 22,080 | 13 | 135 Mbps | 2K/4K capable devices |
| 5.1 | 983,040 | 36,864 | 16 | 240 Mbps | 4K displays |
| 5.2 | 2,073,600 | 36,864 | 16 | 240 Mbps | 4K @ 60fps |

†Level 3.2 special rule: 216,000 MB/s if frame size ≤ 1,620 MBs, else 108,000 MB/s

### Profiles vs Levels

**Profiles** (encoding features):
- Baseline: Basic features, low complexity
- Main: Better compression, B-frames
- High: Best compression, 8×8 transforms

**Levels** (decoder capabilities):
- Independent of profile
- A Baseline Profile Level 4.0 decoder can decode any Baseline stream up to 245,760 MB/s

**Windows RDP Context:**
- Windows Media Foundation supports Baseline, Main, High profiles
- Supports levels 1.0 through 5.2
- **BUT** may reject streams that claim lower levels than required!

---

## Question 5: Supporting Wide Range of Configurations

### Design: Resolution-Level-FPS Matrix

```rust
pub struct VideoCapabilityMatrix {
    configs: Vec<VideoConfig>,
}

#[derive(Debug, Clone)]
pub struct VideoConfig {
    resolution: (u16, u16),
    macroblocks: u32,
    supported_fps: Vec<FpsConfig>,
}

#[derive(Debug, Clone)]
pub struct FpsConfig {
    fps: f32,
    min_level: H264Level,
    recommended_bitrate_kbps: u32,
    qp_range: (u8, u8),
}

impl VideoCapabilityMatrix {
    pub fn new() -> Self {
        Self {
            configs: vec![
                // 720p configs
                VideoConfig {
                    resolution: (1280, 720),
                    macroblocks: 3600,
                    supported_fps: vec![
                        FpsConfig {
                            fps: 24.0,
                            min_level: H264Level::L3_0,
                            recommended_bitrate_kbps: 3000,
                            qp_range: (18, 28),
                        },
                        FpsConfig {
                            fps: 30.0,
                            min_level: H264Level::L3_1,  // 108,000 / 3600 = 30
                            recommended_bitrate_kbps: 4000,
                            qp_range: (18, 28),
                        },
                        FpsConfig {
                            fps: 60.0,
                            min_level: H264Level::L3_2,  // 216,000 / 3600 = 60
                            recommended_bitrate_kbps: 8000,
                            qp_range: (20, 30),
                        },
                    ],
                },

                // 1280×800 (WXGA) configs
                VideoConfig {
                    resolution: (1280, 800),
                    macroblocks: 4000,
                    supported_fps: vec![
                        FpsConfig {
                            fps: 24.0,
                            min_level: H264Level::L3_1,  // 108,000 / 4000 = 27
                            recommended_bitrate_kbps: 3500,
                            qp_range: (18, 28),
                        },
                        FpsConfig {
                            fps: 27.0,  // Max for Level 3.1/3.2
                            min_level: H264Level::L3_1,  // Exactly at limit
                            recommended_bitrate_kbps: 4000,
                            qp_range: (18, 28),
                        },
                        FpsConfig {
                            fps: 30.0,  // Requires Level 4.0!
                            min_level: H264Level::L4_0,  // 245,760 / 4000 = 61.4
                            recommended_bitrate_kbps: 5000,
                            qp_range: (18, 28),
                        },
                        FpsConfig {
                            fps: 60.0,
                            min_level: H264Level::L4_2,  // 522,240 / 4000 = 130
                            recommended_bitrate_kbps: 10000,
                            qp_range: (20, 30),
                        },
                    ],
                },

                // 1080p configs
                VideoConfig {
                    resolution: (1920, 1080),
                    macroblocks: 8100,
                    supported_fps: vec![
                        FpsConfig {
                            fps: 24.0,
                            min_level: H264Level::L4_0,
                            recommended_bitrate_kbps: 6000,
                            qp_range: (20, 30),
                        },
                        FpsConfig {
                            fps: 30.0,
                            min_level: H264Level::L4_0,  // 245,760 / 8100 = 30.3
                            recommended_bitrate_kbps: 8000,
                            qp_range: (20, 30),
                        },
                        FpsConfig {
                            fps: 60.0,
                            min_level: H264Level::L4_2,  // 522,240 / 8100 = 64.5
                            recommended_bitrate_kbps: 15000,
                            qp_range: (22, 32),
                        },
                    ],
                },

                // 1440p configs
                VideoConfig {
                    resolution: (2560, 1440),
                    macroblocks: 14400,
                    supported_fps: vec![
                        FpsConfig {
                            fps: 30.0,
                            min_level: H264Level::L5_0,  // 589,824 / 14400 = 40.9
                            recommended_bitrate_kbps: 12000,
                            qp_range: (22, 32),
                        },
                        FpsConfig {
                            fps: 60.0,
                            min_level: H264Level::L5_1,  // 983,040 / 14400 = 68.3
                            recommended_bitrate_kbps: 20000,
                            qp_range: (24, 34),
                        },
                    ],
                },

                // 4K configs
                VideoConfig {
                    resolution: (3840, 2160),
                    macroblocks: 32400,
                    supported_fps: vec![
                        FpsConfig {
                            fps: 24.0,
                            min_level: H264Level::L5_1,  // 983,040 / 32400 = 30.3
                            recommended_bitrate_kbps: 20000,
                            qp_range: (24, 34),
                        },
                        FpsConfig {
                            fps: 30.0,
                            min_level: H264Level::L5_1,
                            recommended_bitrate_kbps: 25000,
                            qp_range: (24, 34),
                        },
                        FpsConfig {
                            fps: 60.0,
                            min_level: H264Level::L5_2,  // 2,073,600 / 32400 = 64
                            recommended_bitrate_kbps: 40000,
                            qp_range: (26, 36),
                        },
                    ],
                },
            ],
        }
    }

    pub fn get_config(&self, width: u16, height: u16, target_fps: f32) -> Option<&FpsConfig> {
        self.configs
            .iter()
            .find(|c| c.resolution == (width, height))?
            .supported_fps
            .iter()
            .find(|f| f.fps >= target_fps)
    }

    pub fn validate(&self, width: u16, height: u16, fps: f32, level: H264Level) -> bool {
        if let Some(config) = self.get_config(width, height, fps) {
            level >= config.min_level
        } else {
            false
        }
    }
}
```

---

## Question 6: Support Both RemoteFX and H.264

### Rectangle Format Differences

**RemoteFX (TS_RFX_RECT):**
```c
// Used in: RFX_PROGRESSIVE_REGION
struct TS_RFX_RECT {
    uint16_t x;       // Left edge
    uint16_t y;       // Top edge
    uint16_t width;   // Width
    uint16_t height;  // Height
};
```

**H.264 AVC420/AVC444 (RDPGFX_RECT16):**
```c
// Used in: RFX_AVC420_METABLOCK, RDPGFX_WIRE_TO_SURFACE_PDU_1
struct RDPGFX_RECT16 {
    uint16_t left;    // Left bound (inclusive)
    uint16_t top;     // Top bound (inclusive)
    uint16_t right;   // Right bound (inclusive)
    uint16_t bottom;  // Bottom bound (inclusive)
};
```

### Implementation Strategy

**Create Separate Types:**

```rust
// For RemoteFX Progressive Codec
pub struct RfxRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Encode for RfxRect {
    fn encode(&self, dst: &mut WriteCursor<'_>) -> EncodeResult<()> {
        dst.write_u16(self.x);
        dst.write_u16(self.y);
        dst.write_u16(self.width);
        dst.write_u16(self.height);
        Ok(())
    }
}

// For H.264 AVC420/AVC444 and general EGFX use
pub struct GfxRect16 {
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
}

impl Encode for GfxRect16 {
    fn encode(&self, dst: &mut WriteCursor<'_>) -> EncodeResult<()> {
        dst.write_u16(self.left);
        dst.write_u16(self.top);
        dst.write_u16(self.right);
        dst.write_u16(self.bottom);
        Ok(())
    }
}

// Conversion utilities
impl From<GfxRect16> for RfxRect {
    fn from(bounds: GfxRect16) -> Self {
        Self {
            x: bounds.left,
            y: bounds.top,
            width: bounds.right.saturating_sub(bounds.left).saturating_add(1),
            height: bounds.bottom.saturating_sub(bounds.top).saturating_add(1),
        }
    }
}

impl From<RfxRect> for GfxRect16 {
    fn from(rect: RfxRect) -> Self {
        Self {
            left: rect.x,
            top: rect.y,
            right: rect.x.saturating_add(rect.width).saturating_sub(1),
            bottom: rect.y.saturating_add(rect.height).saturating_sub(1),
        }
    }
}
```

**Usage in Codecs:**

```rust
// H.264 AVC420/AVC444 - use GfxRect16 (bounds)
impl Encode for Avc420BitmapStream<'_> {
    fn encode(&self, dst: &mut WriteCursor<'_>) -> EncodeResult<()> {
        for rect in &self.rectangles {
            // rect is GfxRect16
            rect.encode(dst)?;  // left, top, right, bottom
        }
    }
}

// RemoteFX Progressive - use RfxRect (x,y,w,h)
impl Encode for RfxProgressiveRegion<'_> {
    fn encode(&self, dst: &mut WriteCursor<'_>) -> EncodeResult<()> {
        for rect in &self.rects {
            // rect is RfxRect
            rect.encode(dst)?;  // x, y, width, height
        }
    }
}
```

---

## Comprehensive Solution Architecture

### Layer 1: Damage Tracking

```rust
pub struct DamageTracker {
    last_frame: Option<Vec<u8>>,
    threshold: f32,  // 0.05 = 5% pixels must change
}

impl DamageTracker {
    pub fn compute_damage(&mut self, frame: &[u8], width: u16, height: u16)
        -> Vec<DamageRect>
    {
        if let Some(last) = &self.last_frame {
            let changed_pixels = count_changed_pixels(last, frame);
            let total_pixels = (width as usize) * (height as usize);

            if (changed_pixels as f32) / (total_pixels as f32) < self.threshold {
                return vec![];  // No significant change
            }

            // Compute damaged rectangles
            find_damaged_regions(last, frame, width, height)
        } else {
            // First frame = full damage
            vec![DamageRect::full_frame(width, height)]
        }
    }
}
```

### Layer 2: Level Management

```rust
pub struct H264LevelManager {
    current_level: H264Level,
    resolution_mbs: u32,
}

impl H264LevelManager {
    pub fn validate_and_adjust(&mut self, target_fps: f32) -> Result<f32, LevelError> {
        let required_mbs_per_sec = self.resolution_mbs as f32 * target_fps;
        let max_mbs_per_sec = self.current_level.max_macroblocks_per_second() as f32;

        if required_mbs_per_sec <= max_mbs_per_sec {
            Ok(target_fps)
        } else {
            // Auto-adjust FPS to fit current level
            let adjusted_fps = max_mbs_per_sec / self.resolution_mbs as f32;
            warn!(
                "Target {}fps exceeds Level {:?} constraint, reducing to {:.1}fps",
                target_fps, self.current_level, adjusted_fps
            );
            Ok(adjusted_fps)
        }
    }

    pub fn upgrade_level_if_needed(&mut self, target_fps: f32) -> Option<H264Level> {
        let required_mbs_per_sec = self.resolution_mbs as f32 * target_fps;

        for level in H264Level::iter_ascending() {
            if required_mbs_per_sec <= level.max_macroblocks_per_second() as f32 {
                if level > self.current_level {
                    info!("Upgrading H.264 level from {:?} to {:?}", self.current_level, level);
                    self.current_level = level;
                    return Some(level);
                }
                break;
            }
        }
        None
    }
}
```

### Layer 3: Encoder Configuration

```rust
pub struct EncoderConfigBuilder {
    resolution: (u16, u16),
    target_fps: f32,
    level: H264Level,
    bitrate_kbps: u32,
}

impl EncoderConfigBuilder {
    pub fn new(width: u16, height: u16, fps: f32) -> Self {
        let mbs = ((width as u32 / 16) * (height as u32 / 16));
        let level = H264Level::for_resolution_fps(mbs, fps);

        Self {
            resolution: (width, height),
            target_fps: fps,
            level,
            bitrate_kbps: Self::recommend_bitrate(width, height, fps),
        }
    }

    fn recommend_bitrate(width: u16, height: u16, fps: f32) -> u32 {
        // Rule of thumb: 0.1 bits per pixel at 30fps
        let pixels = (width as u32) * (height as u32);
        let base_bitrate = (pixels as f32 * 0.1) as u32;  // bits per second
        let fps_factor = fps / 30.0;
        ((base_bitrate as f32 * fps_factor) / 1000.0) as u32  // kbps
    }

    pub fn build(self) -> Result<EncoderConfig, ConfigError> {
        // Validate constraints
        let mbs = (self.resolution.0 as u32 / 16) * (self.resolution.1 as u32 / 16);
        let required_mbs_per_sec = mbs as f32 * self.target_fps;

        if required_mbs_per_sec > self.level.max_macroblocks_per_second() as f32 {
            return Err(ConfigError::LevelConstraintViolation {
                resolution: self.resolution,
                fps: self.target_fps,
                level: self.level,
                required: required_mbs_per_sec,
                max: self.level.max_macroblocks_per_second(),
            });
        }

        Ok(EncoderConfig {
            width: self.resolution.0,
            height: self.resolution.1,
            fps: self.target_fps,
            level: self.level,
            bitrate_kbps: self.bitrate_kbps,
        })
    }
}
```

---

## Immediate Action Items

### Critical Bugs to Fix

1. **ZGFX Compression Missing**
   - File: IronRDP needs `zgfx::Compressor` implementation
   - PR Required: Yes (to IronRDP upstream)
   - Impact: High - violates MS-RDPEGFX spec

2. **H.264 Level Configuration**
   - File: Need access to OpenH264 level setting
   - Options: Use C API directly or fork openh264 crate
   - Impact: High - current level violation blocks Windows clients

3. **Damage Region Encoding Not Used**
   - File: src/server/egfx_sender.rs
   - Current: Always full_frame()
   - Target: Use damage rectangles from PipeWire
   - Impact: Medium - massive performance improvement opportunity

### Implementation Priorities

**Phase 1: Make It Work (Level Constraints)**
1. Implement H264Level enum and constraint checking
2. Add OpenH264 C API wrapper for level configuration
3. Configure encoder for Level 4.0 for 1280×800 @ 30fps
4. Test and verify frame ACKs received

**Phase 2: Make It Efficient (Compression & Damage)**
1. Implement ZGFX Compressor (PR to IronRDP)
2. Extract damage regions from PipeWire metadata
3. Implement multi-region H.264 encoding
4. Test bandwidth savings

**Phase 3: Make It Robust (Configuration Matrix)**
1. Implement VideoCapabilityMatrix
2. Add dynamic level/FPS adjustment
3. Support 720p, 1080p, 1440p, 4K configurations
4. Add adaptive quality control

---

## Testing Matrix

| Test Case | Resolution | FPS | Expected Level | Should Work? |
|-----------|------------|-----|----------------|--------------|
| Current (broken) | 1280×800 | 30 | 3.2 | ❌ Exceeds constraint |
| Quick fix | 1280×800 | 27 | 3.2 | ✅ At limit |
| Proper fix | 1280×800 | 30 | 4.0 | ✅ Well within |
| Validation | 1280×720 | 30 | 3.1 | ✅ Exactly at limit |
| Standard | 1920×1080 | 30 | 4.0 | ✅ Standard config |
| High refresh | 1920×1080 | 60 | 4.2 | ✅ Gaming/pro |
| 4K | 3840×2160 | 30 | 5.1 | ✅ Modern displays |

---

## Conclusion

Your questions reveal multiple optimization dimensions:

1. **ZGFX Compression** - Missing, needs implementation (IronRDP PR)
2. **Dirty Regions** - Configured but not used, huge performance opportunity
3. **H.264 Levels** - Need proper understanding and configuration system
4. **Framerate Regulation** - Need dynamic adjustment based on level constraints
5. **Multi-Configuration Support** - Need comprehensive resolution/level/FPS matrix

The immediate blocker is H.264 level configuration. We need to either:
- Access OpenH264 C API to set Level 4.0
- Or reduce framerate to 27 fps as a workaround
- Or test at 1280×720 to validate the hypothesis

All other optimizations (ZGFX, damage tracking) are important for production but won't fix the current crash.
