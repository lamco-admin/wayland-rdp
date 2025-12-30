# Implementation Plan: Priority 4 - Latency Governor

**Effort**: 6-8 hours
**Impact**: Professional modes for different use cases
**Dependencies**: Adaptive FPS (optional enhancement)
**Files**: New module in lamco-video, config additions

---

## OVERVIEW

**Goal**: Configurable latency vs quality tradeoffs

**Why**: Different users have different priorities (gaming vs design vs WAN)

**What**: Three modes with different encoding/scheduling policies

---

## MODES

### Interactive Mode (Low Latency)

**Target latency**: <50ms
**Use case**: Gaming, CAD, interactive design
**Tradeoffs**: Higher bandwidth, lower quality

**Settings**:
```rust
LatencyMode::Interactive {
    max_frame_delay_ms: 16,     // 1 frame @ 60fps
    encode_timeout_ms: 10,
    damage_threshold: 0.0,      // Encode ANY change immediately
    quality_preset: QualityPreset::Speed,
    adaptive_fps: false,        // Always max FPS
}
```

### Balanced Mode (Default)

**Target latency**: <100ms
**Use case**: General desktop, office work
**Tradeoffs**: Balanced

**Settings**:
```rust
LatencyMode::Balanced {
    max_frame_delay_ms: 33,     // ~30 FPS
    encode_timeout_ms: 20,
    damage_threshold: 0.02,     // Wait for 2% damage
    quality_preset: QualityPreset::Balanced,
    adaptive_fps: true,
}
```

### Quality Mode (High Quality)

**Target latency**: <300ms
**Use case**: Photo/video editing, color work
**Tradeoffs**: Higher latency, best quality

**Settings**:
```rust
LatencyMode::Quality {
    max_frame_delay_ms: 100,    // Can batch changes
    encode_timeout_ms: 50,
    damage_threshold: 0.05,     // Wait for 5% damage
    quality_preset: QualityPreset::Quality,
    adaptive_fps: true,         // But with longer windows
}
```

---

## IMPLEMENTATION

### File: `../lamco-rdp-workspace/crates/lamco-video/src/latency_governor.rs` (NEW)

```rust
pub struct LatencyGovernor {
    mode: LatencyMode,
    frame_accumulator: FrameAccumulator,
    metrics: LatencyMetrics,
}

struct FrameAccumulator {
    pending_damage: f32,
    first_damage_time: Option<Instant>,
}

struct LatencyMetrics {
    capture_to_encode_ms: RollingAverage,
    encode_to_send_ms: RollingAverage,
    total_latency_ms: RollingAverage,
}

impl LatencyGovernor {
    pub fn should_encode_frame(&mut self, damage_ratio: f32) -> EncodingDecision {
        let elapsed = self.frame_accumulator.first_damage_time
            .map(|t| t.elapsed().as_secs_f32() * 1000.0)
            .unwrap_or(0.0);

        match &self.mode {
            LatencyMode::Interactive { max_frame_delay_ms, damage_threshold, .. } => {
                // Encode immediately if ANY damage
                if damage_ratio > *damage_threshold {
                    EncodingDecision::EncodeNow
                } else if elapsed > *max_frame_delay_ms {
                    EncodingDecision::EncodeKeepalive
                } else {
                    EncodingDecision::Skip
                }
            }

            LatencyMode::Balanced { max_frame_delay_ms, damage_threshold, .. } => {
                // Accumulate damage, encode when threshold met or timeout
                self.frame_accumulator.pending_damage += damage_ratio;

                if self.frame_accumulator.pending_damage >= *damage_threshold {
                    self.frame_accumulator.pending_damage = 0.0;
                    EncodingDecision::EncodeNow
                } else if elapsed > *max_frame_delay_ms {
                    self.frame_accumulator.pending_damage = 0.0;
                    EncodingDecision::EncodeTimeout
                } else {
                    EncodingDecision::Skip
                }
            }

            LatencyMode::Quality { max_frame_delay_ms, damage_threshold, .. } => {
                // Wait longer to batch changes for better compression
                self.frame_accumulator.pending_damage += damage_ratio;

                if self.frame_accumulator.pending_damage >= *damage_threshold {
                    self.frame_accumulator.pending_damage = 0.0;
                    EncodingDecision::EncodeBatch
                } else if elapsed > *max_frame_delay_ms {
                    self.frame_accumulator.pending_damage = 0.0;
                    EncodingDecision::EncodeTimeout
                } else {
                    EncodingDecision::WaitForMore
                }
            }
        }
    }
}

pub enum EncodingDecision {
    EncodeNow,       // Encode immediately
    EncodeKeepalive, // Encode to prevent timeout
    EncodeBatch,     // Encode accumulated changes
    EncodeTimeout,   // Timeout reached
    Skip,            // Skip this frame
    WaitForMore,     // Wait for more damage
}
```

---

## CONFIGURATION

```toml
[latency]
# Latency mode: "interactive", "balanced", "quality"
mode = "balanced"

# Interactive mode settings
interactive_max_delay_ms = 16
interactive_damage_threshold = 0.0

# Balanced mode settings
balanced_max_delay_ms = 33
balanced_damage_threshold = 0.02

# Quality mode settings
quality_max_delay_ms = 100
quality_damage_threshold = 0.05
```

---

## INTEGRATION

```rust
// In display_handler, add latency governor
let mut latency_governor = LatencyGovernor::new(config.latency.mode);

// In frame loop
let damage_ratio = calculate_damage_ratio(&damage_regions, frame_area);

match latency_governor.should_encode_frame(damage_ratio) {
    EncodingDecision::EncodeNow => {
        // Encode and send immediately
    }
    EncodingDecision::Skip => {
        // Skip this frame
        continue;
    }
    EncodingDecision::WaitForMore => {
        // Continue accumulating
        continue;
    }
    // ... other cases
}
```

---

## TESTING

**Interactive mode**:
- Mouse movement: <50ms latency
- Keystroke: <50ms latency
- High CPU: Acceptable tradeoff

**Balanced mode**:
- General use: <100ms latency
- Good responsiveness
- Lower CPU than interactive

**Quality mode**:
- Photo editing: Best quality
- Latency: <300ms acceptable
- Lowest CPU

---

**Estimated**: 6-8 hours
**Priority**: #4
