# Architecture Issues and Focused Solutions

## CURRENT ARCHITECTURE ANALYSIS

### Issue 1: NO CHANNEL PRIORITIZATION ❌

**Current Implementation (IronRDP + wrd-server)**:
```
ALL events → Single unbounded channel → First-come-first-served processing
├─ Input events (keyboard/mouse)
├─ Clipboard events
├─ Audio events
└─ Control events (quit, credentials)
```

**Problems**:
1. **No priority ordering** - Graphics frames can block input events
2. **No bounded queues** - Clipboard can consume unlimited memory
3. **No dropping/coalescing** - Graphics congestion affects ALL traffic
4. **No per-channel flow control** - One slow channel blocks others

**What We SHOULD Have (per your requirements)**:
```
Input Queue (bounded 32)       → Priority 1 (always drain first)
Control Queue (bounded 16)     → Priority 2 (session management)
Clipboard Queue (bounded 8)    → Priority 3 (user operations)
Graphics Queue (bounded 4)     → Priority 4 (drop/coalesce when full)

Multiplexer drains: Input → Control → Clipboard → Graphics (in order)
Graphics: Drop frames if queue full (never blocks other channels)
```

### Issue 2: INCOMPLETE MS-RDPECLIP IMPLEMENTATION ❌

**What IronRDP Implements**:
- ✅ FormatList / FormatListResponse
- ✅ FormatDataRequest / FormatDataResponse (text/images)
- ✅ Capabilities negotiation
- ✅ Lock/Unlock (for file transfer)
- ✅ FileContentsRequest / FileContentsResponse PDU structures

**What IronRDP Does NOT Implement**:
- ❌ FileGroupDescriptorW structure parsing/encoding
- ❌ File transfer state machine
- ❌ FILECONTENTS_SIZE vs FILECONTENTS_RANGE handling
- ❌ Multi-stream file transfer coordination

**What WRD-Server Implements**:
- ✅ Text clipboard (UTF-8 ↔ UTF-16 conversion)
- ✅ HTML Format
- ✅ Image formats (PNG, JPEG, GIF) - basic
- ❌ File transfer (stubs only at lines 149-157 of ironrdp_backend.rs)
- ❌ FileGroupDescriptor generation from Portal file URIs
- ❌ Stream-based file content delivery

**Conclusion**: File transfer is ~30% implemented (PDUs exist, no state machine/logic)

### Issue 3: NO GRAPHICS DROPPING/COALESCING ❌

**Current**:
- Display handler tries to get frame: `try_recv_frame()`
- If channel full: Backpressure propagates to PipeWire thread
- PipeWire thread can't send frames → warns about full channel
- **Graphics congestion affects everything**

**What We Need**:
```rust
// Graphics queue with explicit drop policy
let (gfx_tx, gfx_rx) = mpsc::channel(4); // Small bounded queue

// In PipeWire thread:
if gfx_tx.try_send(frame).is_err() {
    // Channel full - DROP frame (don't block)
    frames_dropped += 1;
}

// In display handler:
// Coalesce multiple frames - only send latest
while let Ok(newer_frame) = gfx_rx.try_recv() {
    current_frame = newer_frame; // Discard older
}
send_rdp_update(current_frame);
```

---

## FOCUSED ACTION PLAN

### PHASE 1: IMMEDIATE FIXES (This Session)

#### 1A. Fix Display Handler Frame Consumption Bug ⚠️ CRITICAL
**File**: `src/server/display_handler.rs:279-340`
**Issue**: Frame rate regulator code broke frame processing loop
**Investigation**:
- Add heartbeat logging to prove loop iterates
- Add error handling for frame_regulator.should_send_frame()
- Temporarily disable regulator to verify pipeline works

**Time**: 30 minutes
**Impact**: **CRITICAL** - Video currently broken despite appearing to work

#### 1B. Deduplicate Portal SelectionTransfer ⚠️ HIGH
**File**: `src/clipboard/manager.rs:265-270`
**Issue**: Portal sends multiple SelectionTransfer for single paste
**Solution**:
```rust
// Track last transfer
let mut last_transfer: Option<(String, u32, Instant)> = None;

while let Some(transfer_event) = transfer_rx.recv().await {
    // Deduplicate within 2-second window
    if let Some((last_mime, last_serial, last_time)) = &last_transfer {
        if transfer_event.mime_type == *last_mime &&
           last_time.elapsed() < Duration::from_secs(2) {
            info!("Ignoring duplicate SelectionTransfer (serial {}, last was {})",
                  transfer_event.serial, last_serial);
            continue;
        }
    }
    last_transfer = Some((transfer_event.mime_type.clone(), transfer_event.serial, Instant::now()));

    // Process transfer...
}
```
**Time**: 15 minutes
**Impact**: Eliminates Windows→Linux paste duplication

#### 1C. Fix Stride Calculation for Screen Quality ⚠️ HIGH
**File**: `src/pipewire/pw_thread.rs:699`
**Issue**: Calculating stride as (size/height), should extract from PipeWire metadata
**Current**:
```rust
stride: (size / config.height as usize) as u32,
```

**Should Be**:
```rust
// Extract stride from SPA buffer metadata
let stride = if let Some(meta) = buffer.metas().first() {
    // Parse spa_meta_region for stride information
    extract_stride_from_metadata(meta, config.width)
} else {
    // Fallback: proper stride calculation with alignment
    ((config.width * 4 + 15) / 16) * 16  // 16-byte aligned
};
```

**Time**: 1 hour
**Impact**: Fixes horizontal line artifacts

### PHASE 2: ARCHITECTURE REFACTOR (Next 2-3 Days)

#### 2A. Implement Priority-Based Channel Multiplexing
**Scope**: Replace single ServerEvent channel with prioritized queues

**Files to Modify**:
- Create new: `src/server/event_multiplexer.rs`
- Modify: `src/server/mod.rs` - Replace IronRDP event handling
- Modify: IronRDP fork if needed (may need to bypass ServerEvent enum)

**Architecture**:
```rust
struct EventMultiplexer {
    input_rx: mpsc::Receiver<InputEvent>,       // Bounded 32
    control_rx: mpsc::Receiver<ControlEvent>,   // Bounded 16
    clipboard_rx: mpsc::Receiver<ClipboardEvent>, // Bounded 8
    graphics_rx: mpsc::Receiver<GraphicsFrame>,  // Bounded 4 (DROP if full)
}

impl EventMultiplexer {
    async fn drain_prioritized(&mut self, writer: &mut impl Write) {
        // Priority 1: Input (never starve)
        while let Ok(input) = self.input_rx.try_recv() {
            process_input(input, writer).await;
        }

        // Priority 2: Control
        if let Ok(ctrl) = self.control_rx.try_recv() {
            process_control(ctrl, writer).await;
        }

        // Priority 3: Clipboard
        if let Ok(clip) = self.clipboard_rx.try_recv() {
            process_clipboard(clip, writer).await;
        }

        // Priority 4: Graphics (coalesce)
        let mut latest_frame = None;
        while let Ok(frame) = self.graphics_rx.try_recv() {
            latest_frame = Some(frame); // Keep only latest
        }
        if let Some(frame) = latest_frame {
            send_graphics(frame, writer).await;
        }
    }
}
```

**Time**: 2-3 days
**Impact**: **FUNDAMENTAL** - Solves congestion, prioritization, flow control

#### 2B. Implement Full MS-RDPECLIP File Transfer
**Scope**: Complete file copy/paste protocol

**Components Needed** (OUR implementation, not IronRDP):
1. **FileGroupDescriptorW builder** (`clipboard/file_descriptor.rs`)
   - Parse Portal file:// URIs
   - Extract file metadata (size, name, timestamps)
   - Build FileGroupDescriptorW structure

2. **File content streamer** (`clipboard/file_streamer.rs`)
   - Handle FileContentsRequest (stream_id, position, size)
   - Read file chunks from Portal
   - Build FileContentsResponse PDUs

3. **Backend event handlers** (`clipboard/ironrdp_backend.rs`)
   - Implement lines 149-157 (currently stubs)
   - Route to file_streamer
   - Handle async file I/O

**Time**: 3-5 days
**Impact**: Enables file copy/paste both directions

### PHASE 3: PROTOCOL COMPLETENESS AUDIT (Next 1-2 Weeks)

#### 3A. Audit IronRDP Protocol Coverage
Create matrix of protocol implementation:

| Protocol | IronRDP Status | wrd-server Status | Priority |
|----------|----------------|-------------------|----------|
| MS-RDPECLIP (Clipboard) | 70% | 60% | HIGH |
| MS-RDPBCGR (Core) | 90% | N/A | - |
| MS-RDPEGFX (Graphics) | 30% | 0% | MEDIUM |
| MS-RDPEDISP (Display Control) | 50% | 0% | HIGH |
| MS-RDPUDP (UDP Transport) | 0% | 0% | LOW |
| MS-RDPMT (Multitransport) | 0% | 0% | LOW |
| MS-RDPDYC (Dynamic Channels) | 60% | 0% | MEDIUM |
| MS-RDPEFS (File System) | 0% | 0% | MEDIUM |
| MS-RDPEAI (Audio Input) | 0% | 0% | LOW |

**Method**: For each protocol:
1. Read Microsoft spec document
2. Compare against IronRDP source
3. List missing features
4. Assess: Implement in IronRDP vs wrd-server vs skip

**Time**: 1 week of research + documentation
**Output**: `PROTOCOL-COMPLETENESS-MATRIX.md`

#### 3B. Decision: Fork IronRDP vs Contribute Upstream
**Factors**:
- IronRDP is client-focused (server support is secondary)
- Our changes (clipboard server fix) might be accepted
- Large protocol additions (MS-RDPEGFX) may not align with their roadmap
- Maintenance burden of fork vs upstream collaboration

**Recommendation**:
1. Submit clipboard server fix as PR to IronRDP (high chance of acceptance)
2. For major protocols (RDPEGFX, EDISP): Implement in wrd-server first, propose to IronRDP later
3. Maintain lightweight fork only if absolutely necessary

---

## IMMEDIATE NEXT STEPS (Right Now)

### Step 1: Fix Display Handler (30 min)
The frame rate regulator broke frame processing. Let me add debug logging to find where loop is stuck.

### Step 2: Fix Stride for Screen Quality (1 hour)
Extract stride from PipeWire metadata instead of calculating.

### Step 3: Deduplicate SelectionTransfer (15 min)
Prevent Portal from triggering multiple pastes.

**After these 3 fixes**:
- Video quality: Should be clean (no horizontal lines)
- Clipboard: Reliable, no duplicates
- Performance: Responsive typing, smooth video

**Then we can tackle**:
- Priority queue architecture (2-3 days)
- File transfer implementation (3-5 days)
- Protocol completeness audit (1 week)

---

## SPECIFIC ANSWERS TO YOUR QUESTIONS

**Q: Is MS-RDPECLIP implementation ours or IronRDP's?**
**A**: Hybrid
- IronRDP: PDU encoding/decoding (~70% complete)
- Our code: Integration logic, Portal API, file transfer (~60% complete)
- **File transfer**: PDUs exist in IronRDP, business logic needed in wrd-server

**Q: Do we have separate queues per channel?**
**A**: ❌ **NO** - Currently single unbounded channel for all events
- Need to implement separate bounded queues as you specified
- Need priority-based multiplexing: input → control → clipboard → graphics
- Graphics needs drop/coalesce policy

**Q: Should graphics drop/converge to not congest other channels?**
**A**: ✅ **ABSOLUTELY** - This is critical QoS requirement
- Graphics queue should be smallest (bounded 4)
- try_send() to graphics - if full, DROP frame
- Coalesce multiple queued frames - send only latest
- Never block input/clipboard on graphics congestion

---

## RECOMMENDED IMMEDIATE WORKFLOW

1. **Fix display handler** (I'll do this now - 30 min)
2. **Test video quality**
3. **Fix stride** if still has horizontal lines (1 hour)
4. **Test clipboard duplication**
5. **Implement deduplication** if still duplicating (15 min)
6. **Comprehensive test** of all features
7. **Then** tackle architectural refactor (priority queues)

Should I proceed with Step 1 (fixing display handler) immediately?
