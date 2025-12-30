# Damage Tracking Implementation Status

**Date**: 2025-12-26
**Status**: ✅ Phase 2 Complete - Multi-Region EGFX Operational

## Overview

Tile-based damage detection for bandwidth optimization. Detects unchanged screen regions to skip encoding, achieving 90%+ bandwidth reduction for static content.

## Architecture

```
PipeWire Frame
      │
      ▼
┌─────────────────────┐
│ DamageDetector      │
│ ┌─────────────────┐ │
│ │ Tile Grid 64×64 │ │
│ │ SIMD Comparison │ │
│ │ Region Merging  │ │
│ └─────────────────┘ │
└─────────────────────┘
      │
      ├─► Empty? → Skip Frame (no encoding)
      │
      ▼
┌─────────────────────┐
│ H.264 Encoder       │
│ (AVC420/AVC444)     │
└─────────────────────┘
      │
      ▼
   EGFX Channel
```

## Components Implemented

### 1. DamageDetector (`src/damage/mod.rs`)

**Types:**
- `DamageRegion` - Rectangle with x, y, width, height
- `DamageConfig` - Tile size, thresholds, merge distance
- `DamageStats` - Frame counts, bandwidth savings

**API:**
```rust
let mut detector = DamageDetector::new(DamageConfig::default());

// Detect changes - returns empty Vec if no damage
let regions = detector.detect(&frame_data, width, height);

if regions.is_empty() {
    // Skip frame - no changes!
}
```

**Algorithm:**
1. Divide frame into 64×64 pixel tiles
2. SIMD-compare each tile against previous frame
3. Mark tile dirty if >5% pixels differ
4. Merge adjacent dirty tiles within 32px
5. Return merged damage regions

### 2. SIMD Optimization

**x86_64 (AVX2):**
- 32 bytes processed per iteration
- Uses `_mm256_subs_epu8` for absolute difference
- `_mm256_movemask_epi8` for fast counting

**aarch64 (NEON):**
- 16 bytes processed per iteration
- Uses `vabdq_u8` for absolute difference
- `vaddvq_u8` for horizontal sum

**Scalar Fallback:**
- Portable implementation for all platforms
- Used for edge cases and small buffers

### 3. Display Handler Integration (`src/server/display_handler.rs`)

Added damage detection to frame processing pipeline:

```rust
// Detect changed regions
let damage_regions = damage_detector.detect(&frame.data, frame.width, frame.height);

if damage_regions.is_empty() {
    // No changes - skip encoding entirely
    frames_skipped_damage += 1;
    continue;
}

// Proceed with encoding only if damage detected
```

## Performance Benchmarks

| Resolution | Detection Time | Throughput | Target |
|------------|---------------|------------|--------|
| 480p (640×480) | 0.43ms | 715 Mpix/s | ✅ <3ms |
| 720p (1280×720) | 1.29ms | 715 Mpix/s | ✅ <3ms |
| 1080p (1920×1080) | 3.05ms | 680 Mpix/s | ⚠️ ~3ms |
| 4K (3840×2160) | ~12ms | - | - |

## Bandwidth Reduction Estimates

| Scenario | Without Damage | With Damage | Reduction |
|----------|----------------|-------------|-----------|
| Static Desktop | 8 Mbps | 0.5 Mbps | 94% |
| Typing in Editor | 8 Mbps | 1 Mbps | 87% |
| Window Dragging | 8 Mbps | 2 Mbps | 75% |
| Fast Scrolling | 8 Mbps | 6 Mbps | 25% |

## Test Coverage

**29 unit tests covering:**
- DamageRegion operations (overlaps, union, is_adjacent)
- DamageConfig presets
- Pixel comparison (scalar and SIMD paths)
- Region merging algorithm
- DamageDetector (first frame, identical, partial, dimension change)
- Edge cases (odd dimensions, small frames, large frames)

## Files Created/Modified

### New Files
- `src/damage/mod.rs` - Complete damage detection implementation
- `benches/damage_detection.rs` - Performance benchmarks

### Modified Files
- `src/lib.rs` - Export damage module
- `src/server/display_handler.rs` - Integration with frame pipeline (Phases 1 & 2)
- `src/server/egfx_sender.rs` - Multi-region EGFX methods (Phase 2)
- `Cargo.toml` - Benchmark configuration

## Configuration

```rust
pub struct DamageConfig {
    pub tile_size: usize,        // Default: 64
    pub diff_threshold: f32,     // Default: 0.05 (5%)
    pub pixel_threshold: u8,     // Default: 4
    pub merge_distance: u32,     // Default: 32
    pub min_region_area: u64,    // Default: 256
}

// Presets
DamageConfig::default()       // Balanced
DamageConfig::low_bandwidth() // Fine-grained, sensitive
DamageConfig::high_motion()   // Coarse, faster
```

## Phase 2: Multi-Region EGFX (Complete ✅)

Damage regions are now passed to EGFX for client-side rendering optimization:

1. **`egfx_sender.rs` updated** with `send_frame_with_regions()` and `send_avc444_frame_with_regions()` methods
2. **`damage_regions_to_avc420()` helper** converts DamageRegion to Avc420Region (LTRB format)
3. **display_handler.rs integrated** - passes detected regions to EGFX sender

### How It Works

```rust
// Detect damage regions
let damage_regions = damage_detector.detect(&frame.data, frame.width, frame.height);

if damage_regions.is_empty() {
    continue; // Skip frame - no changes (Phase 1)
}

// Send with regions - client knows which areas changed (Phase 2)
sender.send_frame_with_regions(&h264_data, ..., &damage_regions, ...).await;
```

### Client Benefits

- Client can optimize rendering by only updating damaged areas
- Reduces GPU work on client side
- Better visual latency for partial updates (typing, cursors)

## Running Benchmarks

```bash
# All damage benchmarks
cargo bench --bench damage_detection

# Specific scenarios
cargo bench --bench damage_detection -- "no_damage"
cargo bench --bench damage_detection -- "partial"
```

## Running Tests

```bash
# All damage tests
cargo test damage --lib

# Full test suite
cargo test --lib --features h264
```

## Known Limitations

1. **First frame always full damage** - No previous frame to compare
2. **Resolution change invalidates** - Full damage on dimension change
3. **3ms detection at 1080p** - Right at target, may need optimization for 4K
4. **Full-frame encoding** - Phase 2 needed for per-region encoding
