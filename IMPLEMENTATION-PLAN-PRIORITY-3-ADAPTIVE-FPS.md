# Implementation Plan: Priority 3 - Adaptive FPS

**Effort**: 8-12 hours
**Impact**: 30-50% CPU reduction, better responsiveness
**Dependencies**: Damage tracking (already implemented)
**Files**: New module in lamco-video, integration in display_handler

---

## OVERVIEW

**Goal**: Dynamically adjust frame rate based on screen activity

**Why**: Static screens don't need 30 FPS, active screens benefit from it

**What**: Damage-driven FPS controller that adapts to content

---

## IMPLEMENTATION

### Location: `../lamco-rdp-workspace/crates/lamco-video/src/adaptive_fps.rs` (NEW)

```rust
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct AdaptiveFpsController {
    /// Target FPS (configured max)
    target_fps: u32,

    /// Current FPS (dynamically adjusted)
    current_fps: u32,

    /// Damage history (last N frames)
    damage_history: VecDeque<DamageRatio>,

    /// Last frame time
    last_frame: Instant,

    /// Configuration
    config: AdaptiveFpsConfig,
}

pub struct AdaptiveFpsConfig {
    /// Minimum FPS (even for static content)
    pub min_fps: u32,  // Default: 5

    /// Maximum FPS (target)
    pub max_fps: u32,  // Default: 30

    /// History window size
    pub history_size: usize,  // Default: 10

    /// Thresholds for FPS adjustment
    pub high_activity_threshold: f32,  // Default: 0.3 (30% damage)
    pub medium_activity_threshold: f32,  // Default: 0.1 (10% damage)
    pub low_activity_threshold: f32,  // Default: 0.01 (1% damage)
}

#[derive(Clone, Copy)]
pub struct DamageRatio {
    pub ratio: f32,  // 0.0-1.0
    pub timestamp: Instant,
}

impl AdaptiveFpsController {
    pub fn new(config: AdaptiveFpsConfig) -> Self {
        Self {
            target_fps: config.max_fps,
            current_fps: config.max_fps,
            damage_history: VecDeque::with_capacity(config.history_size),
            last_frame: Instant::now(),
            config,
        }
    }

    /// Update with new frame damage info
    pub fn update(&mut self, damage_ratio: f32) {
        // Add to history
        self.damage_history.push_back(DamageRatio {
            ratio: damage_ratio,
            timestamp: Instant::now(),
        });

        // Limit history size
        if self.damage_history.len() > self.config.history_size {
            self.damage_history.pop_front();
        }

        // Calculate average damage over recent history
        let avg_damage = self.average_damage();

        // Adjust FPS based on activity level
        self.current_fps = if avg_damage > self.config.high_activity_threshold {
            // High activity (video, dragging) - full FPS
            self.config.max_fps
        } else if avg_damage > self.config.medium_activity_threshold {
            // Medium activity (scrolling) - 2/3 FPS
            (self.config.max_fps * 2 / 3).max(self.config.min_fps)
        } else if avg_damage > self.config.low_activity_threshold {
            // Low activity (typing, cursor) - 1/2 FPS
            (self.config.max_fps / 2).max(self.config.min_fps)
        } else {
            // Static - minimum FPS (keepalive only)
            self.config.min_fps
        };

        debug!(
            "Adaptive FPS: avg_damage={:.1}%, target_fps={}",
            avg_damage * 100.0,
            self.current_fps
        );
    }

    /// Should we capture this frame?
    pub fn should_capture_frame(&mut self) -> bool {
        let elapsed = self.last_frame.elapsed();
        let frame_interval = Duration::from_secs_f32(1.0 / self.current_fps as f32);

        if elapsed >= frame_interval {
            self.last_frame = Instant::now();
            true
        } else {
            false
        }
    }

    fn average_damage(&self) -> f32 {
        if self.damage_history.is_empty() {
            return 0.0;
        }

        let sum: f32 = self.damage_history.iter().map(|d| d.ratio).sum();
        sum / self.damage_history.len() as f32
    }
}
```

---

## INTEGRATION

### File: `src/server/display_handler.rs`

**Add to frame processing loop**:

```rust
// After damage detection
let damage_ratio = if !damage_regions.is_empty() {
    let frame_area = (frame.width * frame.height) as u64;
    let damage_area: u64 = damage_regions.iter().map(|r| r.area()).sum();
    damage_area as f32 / frame_area as f32
} else {
    0.0
};

// Update adaptive FPS controller
adaptive_fps.update(damage_ratio);

// Check if we should capture this frame
if !adaptive_fps.should_capture_frame() {
    // Skip this frame - FPS throttling
    frames_skipped_fps += 1;
    continue;
}

// Proceed with encoding...
```

---

## CONFIGURATION

### Add to config.toml:

```toml
[adaptive_fps]
enabled = true
min_fps = 5      # Static screen keepalive
max_fps = 30     # Maximum frame rate
high_activity_threshold = 0.30   # 30% damage = full FPS
medium_activity_threshold = 0.10  # 10% damage = 2/3 FPS
low_activity_threshold = 0.01    # 1% damage = 1/2 FPS
```

---

## TESTING

**Scenario 1: Static screen**:
- Expected: Drops to 5 FPS
- CPU usage: Should decrease significantly
- Bandwidth: Minimal (damage tracking skips frames anyway)

**Scenario 2: Typing**:
- Expected: 15 FPS (medium activity)
- Feel: Responsive for text input
- Bandwidth: Low

**Scenario 3: Video playback**:
- Expected: 30 FPS (high activity)
- Quality: Smooth playback
- Bandwidth: Full rate

**Scenario 4: Window dragging**:
- Expected: Ramps up to 30 FPS quickly
- Feel: Smooth movement
- Ramps down: Returns to low FPS when stopped

---

## SUCCESS CRITERIA

- [ ] Static screen: <10% CPU of full-rate capture
- [ ] Typing: Responsive, <300ms input latency
- [ ] Video: Smooth 30 FPS maintained
- [ ] Window drag: Smooth, ramps up/down correctly
- [ ] Config toggleable (enabled=true/false works)

---

**Estimated**: 8-12 hours
**Priority**: #3
