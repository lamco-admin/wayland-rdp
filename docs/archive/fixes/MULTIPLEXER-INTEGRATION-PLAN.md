# Event Multiplexer Integration Plan

## Status: Foundation Complete, Integration Pending

---

## WHAT'S DONE ✅

### 1. Event Multiplexer Module Created
**File**: `src/server/event_multiplexer.rs` (330 lines)

**Components**:
- `EventMultiplexer` struct with 4 priority queues
- `InputEvent`, `ControlEvent`, `ClipboardEvent`, `GraphicsFrame` enums
- `drain_cycle()` - Priority-based event processing
- Statistics tracking for dropped/coalesced events

**Queue Configuration**:
```rust
Input Queue:     bounded(32)  - Priority 1
Control Queue:   bounded(16)  - Priority 2
Clipboard Queue: bounded(8)   - Priority 3
Graphics Queue:  bounded(4)   - Priority 4 (drop/coalesce)
```

### 2. Display Handler Prepared
**File**: `src/server/display_handler.rs`
- Added `graphics_tx: Option<mpsc::Sender<GraphicsFrame>>` field
- Added `set_graphics_queue()` method
- Routing logic placeholder (currently sends direct to IronRDP)

---

## WHAT'S NEEDED 🔧

### Integration Architecture

Current flow (IronRDP's single queue):
```
Input Events ────┐
Clipboard Events ├──> ServerEvent (unbounded) ──> FIFO Processing ──> TCP
Graphics Events ─┤
Control Events ──┘
```

Target flow (Priority-based multiplexing):
```
Input Events ────> Input Queue (32) ────┐
                                        │
Control Events ──> Control Queue (16) ──┤
                                        ├──> Multiplexer ──> Priority Drain ──> TCP
Clipboard Events > Clipboard Queue (8) ─┤      drain_cycle()     Input→Control
                                        │                        →Clipboard→Gfx
Graphics Events ─> Graphics Queue (4) ──┘
                   (drop if full)
```

### Step-by-Step Integration

#### STEP 1: Create Multiplexer in Server Initialization
**File**: `src/server/mod.rs` (around line 240-290)

```rust
use crate::server::event_multiplexer::EventMultiplexer;

// In WrdServer::new():
// After creating display_handler, input_handler, clipboard_manager:

// Create event multiplexer
let multiplexer = Arc::new(Mutex::new(EventMultiplexer::new()));

// Configure display handler with graphics queue
{
    let mut display = Arc::clone(&display_handler);
    // Can't get mutable access to Arc<WrdDisplayHandler> easily
    // Need different approach - see Step 1b
}
```

**Problem**: `display_handler` is `Arc<WrdDisplayHandler>`, can't get `&mut` for `set_graphics_queue()`

**Solution**:
- Option A: Make `graphics_tx` an `Arc<RwLock<Option<...>>>` instead of `Option<...>`
- Option B: Pass graphics_tx to constructor instead of setting later
- Option C: Use interior mutability pattern

**Recommendation**: Option B (cleaner)

#### STEP 2: Modify Display Handler Constructor
**File**: `src/server/display_handler.rs:179-257`

Change signature:
```rust
pub async fn new(
    initial_width: u16,
    initial_height: u16,
    pipewire_fd: i32,
    stream_info: Vec<StreamInfo>,
    graphics_tx: Option<mpsc::Sender<GraphicsFrame>>,  // NEW PARAMETER
) -> Result<Self>
```

Update struct initialization:
```rust
Ok(Self {
    size,
    pipewire_thread,
    bitmap_converter,
    update_sender,
    update_receiver,
    graphics_tx,  // Use parameter instead of None
    stream_info,
})
```

#### STEP 3: Create Event Drain Loop
**File**: NEW - `src/server/event_loop.rs` or in `mod.rs`

```rust
pub async fn run_multiplexed_event_loop(
    mut multiplexer: EventMultiplexer,
    mut ironrdp_writer: impl AsyncWrite,
    display_handler: Arc<WrdDisplayHandler>,
) -> Result<()> {
    info!("Starting multiplexed event loop with priority QoS");

    loop {
        // Drain events in priority order
        let events = multiplexer.drain_cycle().await;

        // PRIORITY 1: Process ALL input events (never starve)
        for input_event in events.input_events {
            match input_event {
                InputEvent::Keyboard(kbd) => {
                    // Encode and write keyboard PDU
                    // (Currently handled by IronRDP internally)
                    // May need to bypass IronRDP for this
                }
                InputEvent::Mouse(mouse) => {
                    // Encode and write mouse PDU
                }
            }
        }

        // PRIORITY 2: Process control event
        if let Some(control) = events.control_event {
            match control {
                ControlEvent::Quit(reason) => {
                    info!("Quit event: {}", reason);
                    break;
                }
                // ... other control events
            }
        }

        // PRIORITY 3: Process clipboard event
        if let Some(clipboard) = events.clipboard_event {
            // Process clipboard PDU
        }

        // PRIORITY 4: Process graphics (coalesced)
        if let Some(frame) = events.graphics_frame {
            // Convert GraphicsFrame back to IronRDP's DisplayUpdate
            // Send to IronRDP encoder
        }

        // Small yield to prevent busy-looping
        tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
    }

    Ok(())
}
```

#### STEP 4: Bypass IronRDP's Event System

**Challenge**: IronRDP's `RdpServer` uses internal `ServerEvent` channel
- Input events generated by IronRDP input callbacks
- We can't easily intercept before they hit IronRDP's queue

**Solutions**:

**Option A**: Fork IronRDP server event loop
- Copy `src/server.rs:367-700` to our codebase
- Modify to use our multiplexer instead of ServerEvent channel
- Maintenance burden: High

**Option B**: Wrapper pattern
- Let IronRDP handle input/control as-is
- Route ONLY graphics/clipboard through multiplexer
- Partial QoS (better than none)
- Maintenance burden: Low

**Option C**: Submit PR to IronRDP
- Add pluggable event multiplexer interface to IronRDP
- Allows custom QoS implementations
- Maintenance burden: None (if accepted)
- Timeline: Weeks/months for PR review

**Recommendation for Now**: **Option B** (pragmatic)

---

## IMPLEMENTATION STRATEGY (Option B - Partial Multiplexer)

This gives us 80% of the benefits with 20% of the complexity:

### Phase 1: Graphics Queue Only (2-3 hours)

**What**: Route graphics through priority queue with drop/coalesce
**Impact**: Prevents graphics from blocking anything else
**Complexity**: LOW

```rust
// In server initialization:
let (graphics_tx, graphics_rx) = mpsc::channel(4);

// Create display handler with graphics channel
let display_handler = WrdDisplayHandler::new(
    width, height, fd, streams,
    Some(graphics_tx),  // Enable graphics queue
).await?;

// Start graphics drain task
tokio::spawn(async move {
    let mut last_frame: Option<GraphicsFrame> = None;

    loop {
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Coalesce: Keep only latest frame
        while let Ok(frame) = graphics_rx.try_recv() {
            last_frame = Some(frame);
        }

        if let Some(frame) = last_frame.take() {
            // Convert to DisplayUpdate and send to IronRDP
            send_graphics_update(frame, &display_handler).await;
        }
    }
});
```

**Benefits**:
- Graphics never blocks input/clipboard
- Automatic frame coalescing under load
- Drop policy prevents congestion

### Phase 2: Full Multiplexer (Future - 1-2 weeks)

**What**: Fork IronRDP event loop or submit PR
**Impact**: Complete QoS control
**Complexity**: HIGH

---

## CURRENT RECOMMENDATION

**Immediate** (this session):
1. Deploy current fixes (input handler, clipboard paste fix)
2. Test thoroughly
3. Document multiplexer integration plan (this document)

**Next Session** (2-3 hours):
1. Implement Phase 1 (graphics queue only)
2. Test graphics QoS behavior
3. Verify input/clipboard never blocked

**Future** (when needed):
1. Evaluate full multiplexer necessity
2. Consider IronRDP contribution vs fork
3. Implement based on production requirements

---

## FILES TO MODIFY FOR PHASE 1

1. **src/server/display_handler.rs**
   - Change constructor to accept `graphics_tx`
   - Remove placeholder graphics routing code
   - Actually use graphics queue when provided

2. **src/server/mod.rs**
   - Create graphics channel
   - Pass to display handler constructor
   - Spawn graphics drain task

3. **NEW: src/server/graphics_drain.rs**
   - Implement graphics coalescing and IronRDP update generation
   - Handle GraphicsFrame → DisplayUpdate conversion

**Estimated Time**: 2-3 hours focused work

---

## TESTING PLAN

### Graphics Queue Tests
1. **High frame load**: Move windows rapidly, verify no input lag
2. **Graphics congestion**: Generate heavy screen activity, verify frames drop (not block)
3. **Input priority**: Type during heavy graphics, verify responsive
4. **Clipboard during video**: Copy/paste during video playback

### Success Criteria
- ✅ Input never lags even during graphics congestion
- ✅ Clipboard operations complete quickly regardless of video load
- ✅ Frame drops visible in logs when graphics queue full
- ✅ No backpressure warnings from PipeWire thread

---

## DECISION POINT

Given time/complexity trade-offs:

**Deploy current stable build** (with all fixes applied):
- Input handler fix
- Clipboard paste deduplication
- Performance optimizations
- Comprehensive logging

**Document multiplexer integration** for next focused session

**OR**

**Continue multiplexer integration now** (2-3 more hours):
- Implement Phase 1 (graphics queue)
- Test QoS behavior
- Deploy integrated system

---

## END OF INTEGRATION PLAN
Status: Ready for Phase 1 implementation
Created: 2025-12-10
