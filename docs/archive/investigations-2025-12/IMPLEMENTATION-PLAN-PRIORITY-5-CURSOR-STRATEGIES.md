# Implementation Plan: Priority 5 - Cursor Strategies

**Effort**: 8-10 hours
**Impact**: Better UX, reduced latency perception
**Dependencies**: None
**Files**: New cursor module, integration in display_handler

---

## OVERVIEW

**Goal**: Smart cursor handling based on network conditions and use case

**Why**: Cursor is most latency-sensitive element

**What**: Multiple cursor modes with automatic selection

---

## MODES

### Mode 1: Metadata (Current)

**How**: Server sends position + shape, client draws
**Pro**: Zero latency for cursor movement
**Con**: Requires RDP cursor support

### Mode 2: Painted Cursor

**How**: Composite cursor into frame, send as video
**Pro**: Works with all clients
**Con**: Cursor has video latency

### Mode 3: Separate Cursor Stream (NEW)

**How**: Dedicated lightweight stream for cursor
**Pro**: Low latency, works everywhere
**Con**: Slightly more complex

### Mode 4: Predictive Cursor (INNOVATION)

**How**: Predict cursor position based on velocity
**Pro**: Feels instant even with 100ms+ latency
**Con**: Can overshoot or predict wrong

---

## IMPLEMENTATION

### File: `src/cursor/predictor.rs` (NEW)

```rust
pub struct CursorPredictor {
    position: (i32, i32),
    velocity: (f32, f32),
    acceleration: (f32, f32),
    history: VecDeque<CursorSample>,
    config: PredictorConfig,
}

struct CursorSample {
    position: (i32, i32),
    timestamp: Instant,
}

struct PredictorConfig {
    history_size: usize,
    lookahead_ms: f32,
    velocity_smoothing: f32,
}

impl CursorPredictor {
    pub fn update(&mut self, position: (i32, i32)) {
        self.history.push_back(CursorSample {
            position,
            timestamp: Instant::now(),
        });

        if self.history.len() > self.config.history_size {
            self.history.pop_front();
        }

        // Calculate velocity from recent samples
        self.update_velocity();
        self.update_acceleration();
    }

    pub fn predict(&self, lookahead_ms: f32) -> (i32, i32) {
        let dt = lookahead_ms / 1000.0;

        // Physics-based prediction
        let pred_x = self.position.0 as f32
            + self.velocity.0 * dt
            + 0.5 * self.acceleration.0 * dt * dt;

        let pred_y = self.position.1 as f32
            + self.velocity.1 * dt
            + 0.5 * self.acceleration.1 * dt * dt;

        (pred_x as i32, pred_y as i32)
    }

    fn update_velocity(&mut self) {
        if self.history.len() < 2 {
            return;
        }

        let recent = &self.history[self.history.len() - 1];
        let prev = &self.history[self.history.len() - 2];

        let dt = recent.timestamp.duration_since(prev.timestamp).as_secs_f32();
        if dt > 0.0 {
            let vx = (recent.position.0 - prev.position.0) as f32 / dt;
            let vy = (recent.position.1 - prev.position.1) as f32 / dt;

            // Smooth with previous velocity
            let alpha = self.config.velocity_smoothing;
            self.velocity.0 = alpha * vx + (1.0 - alpha) * self.velocity.0;
            self.velocity.1 = alpha * vy + (1.0 - alpha) * self.velocity.1;
        }
    }
}
```

---

## CONFIGURATION

```toml
[cursor]
# Cursor mode: "metadata", "painted", "separate", "predictive"
mode = "metadata"

# Predictive cursor settings
predictive_lookahead_ms = 50  # Predict 50ms ahead
predictive_enabled_over_latency_ms = 100  # Enable if latency > 100ms

# Separate stream settings
separate_stream_fps = 60  # Higher FPS for cursor smoothness
```

---

## TESTING

**Predictive cursor**:
- Move mouse in straight line
- Predicted cursor should lead actual
- Stop mouse: Predicted should converge quickly
- Zigzag movement: Should track reasonably

**All modes**:
- Cursor visible and correct
- Click accuracy maintained
- No visual artifacts
- Smooth on high-latency connections

---

**Estimated**: 8-10 hours
**Priority**: #5
