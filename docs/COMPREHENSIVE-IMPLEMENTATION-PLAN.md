# Comprehensive Implementation Plan - Production-Grade RDP Server

**Date:** 2025-12-21
**Context:** Post-initial testing - video and text clipboard working
**Goal:** Build a robust, full-featured, production-quality RDP server

---

## CURRENT STATE

### ✅ What Actually Works
- RDP connection (TLS, authentication)
- Video streaming (RemoteFX - deprecated, needs replacement)
- Text clipboard bidirectional (Windows ↔ Linux)
- Portal integration (screen sharing, input injection)
- PipeWire frame capture (MemFd buffers)

### ❌ What's Broken or Missing
1. Using deprecated RemoteFX codec (artifacts, not optimal)
2. 40% frame drop rate (architectural issue)
3. No frame damage region coalescing (inefficient)
4. File transfer claimed but not implemented (false advertising)
5. Limited MIME type support (text only, no images/HTML/RTF)
6. Zero-size buffer crashes
7. H.264/EGFX code exists but not integrated

---

## IMPLEMENTATION PRIORITIES

### Phase 1: Video Pipeline Robustness (CURRENT FOCUS)

#### 1.1 Enable H.264/EGFX Codec (Replace RemoteFX)

**Why:**
- RemoteFX is DEPRECATED by Microsoft
- H.264 has 50-70% better compression
- Fewer artifacts, better video quality
- Hardware acceleration support (VAAPI)
- Modern, maintained codec

**What needs to happen:**
1. ✅ Enable h264 feature by default (already done)
2. Integrate EgfxServer as DVC processor with IronRDP
3. Wire EgfxVideoHandler to receive frames from PipeWire
4. Implement capability negotiation (prefer H.264, fallback to RemoteFX if needed)
5. Test encoding performance
6. Configure bitrate, quality settings

**Files to modify:**
- `src/server/mod.rs` - Integration
- `src/server/display_handler.rs` - Frame routing
- `src/egfx/*` - Already implemented, needs wiring

**Research needed:**
- How to register DVC processors with IronRDP server
- EGFX channel lifecycle management
- Frame acknowledgment flow control

#### 1.2 Implement Frame Damage Region Coalescing

**Current problem:**
- Every frame processed independently
- Small changes encode entire frame
- Wasteful bandwidth and CPU

**Proper implementation:**
- Collect damage regions from PipeWire
- Merge overlapping rectangles
- Only encode changed regions
- Accumulate changes over queue depth
- Send composite update

**Implementation:**
```rust
struct DamageCoalescer {
    pending_regions: Vec<Rectangle>,
    max_regions: usize,
    merge_threshold: f32,
}

impl DamageCoalescer {
    fn add_damage(&mut self, region: Rectangle);
    fn merge_overlapping(&mut self);
    fn coalesce_to_minimal_set(&mut self) -> Vec<Rectangle>;
}
```

**Location:** `src/video/damage.rs` (new module) or enhance graphics_drain.rs

#### 1.3 Fix Graphics Queue Architecture

**Changes needed:**
- ✅ Increase queue size to 64 (done)
- Implement proper backpressure to PipeWire when queue fills
- Add frame coalescing in graphics_drain task
- Monitor queue depth and adjust capture rate dynamically

**Current graphics_drain.rs:**
- Just forwards frames 1:1
- No coalescing
- No backpressure

**Needs enhancement:**
- Coalesce multiple frames if queue has backlog
- Signal PipeWire to slow down if queue > 80% full
- Speed up if queue < 20% full

#### 1.4 Fix PipeWire Buffer Handling

**Issues:**
- Zero-size buffers cause crashes
- Stride mismatches
- Not using DMA-BUF (copying 4MB/frame unnecessarily)

**Proper fixes:**
- Validate buffer metadata before processing
- Handle stride correctly for all formats
- Prefer DMA-BUF when available (zero-copy)
- Log buffer issues without crashing

**Location:** `lamco-pipewire/src/pw_thread.rs` - process() callback

---

### Phase 2: Clipboard Completeness

#### 2.1 Implement File Transfer Support

**Current:** Claims STREAM_FILECLIP_ENABLED but doesn't implement it

**Full implementation needed:**

**A. Handle FileDescriptor Format (CF_HDROP)**
```rust
// When FormatList contains format 49158 (CF_HDROP):
async fn handle_file_descriptor_format(
    &self,
    formats: &[ClipboardFormat],
) -> Result<Vec<FileDescriptor>> {
    // Request FILEDESCRIPTOR format from client
    // Parse FILEDESCRIPTORW structures
    // Extract: filename, size, attributes, timestamps
}
```

**B. Implement FileContentsRequest Handler**
```rust
async fn handle_file_contents_request(
    &self,
    stream_id: u32,
    list_index: u32,
    position: u64,
    size: u32,
) -> Result<()> {
    // Client wants file data from Linux
    // Read from Linux filesystem
    // Stream in chunks (max ~4MB per chunk)
    // Send FileContentsResponse
}
```

**C. Implement FileContentsResponse Handler**
```rust
async fn handle_file_contents_response(
    &self,
    stream_id: u32,
    data: Vec<u8>,
    is_error: bool,
) -> Result<()> {
    // Receiving file from Windows
    // Assemble chunks
    // Write to Linux filesystem
    // Handle progress/errors
}
```

**D. Add to Event Bridge**
- Route FileContentsRequest events
- Route FileContentsResponse events

**E. File I/O Management**
```rust
struct FileTransferSession {
    files: Vec<FileDescriptor>,
    current_file: usize,
    current_position: u64,
    output_directory: PathBuf,
    temp_files: HashMap<u32, TempFile>,
}
```

**Estimated code:** 800-1000 lines
**Files:**
- `src/clipboard/file_transfer.rs` (new)
- `src/clipboard/manager.rs` (extend)
- Event bridge (extend)

#### 2.2 Implement All MIME Types

**Currently supported:** text/plain

**Need to add:**
- text/html (HTML clipboard)
- text/rtf (Rich Text Format)
- image/png, image/jpeg, image/bmp (images)
- image/x-bmp, image/x-ms-bmp (Windows BMP variants)
- application/x-qt-image (Qt images)
- Files (x-special/gnome-copied-files, text/uri-list)

**Implementation:**
```rust
// In format converter
fn convert_image_to_dib(&self, png_data: &[u8]) -> Result<Vec<u8>>;
fn convert_dib_to_png(&self, dib_data: &[u8]) -> Result<Vec<u8>>;
fn convert_html_encoding(&self, html: &str, to_windows: bool) -> Result<String>;
fn convert_rtf(&self, rtf_data: &[u8]) -> Result<Vec<u8>>;
```

**Location:** lamco-clipboard-core (published crate) or server extension

---

### Phase 3: Protocol Robustness

#### 3.1 Capability Negotiation Strategy

**Current:** Hardcoded "remotefx"

**Proper implementation:**
```rust
fn negotiate_best_codec(client_caps: &[ClientCodec]) -> Vec<String> {
    // Priority order:
    // 1. H.264/AVC420 (if client supports EGFX)
    // 2. AVC444 (if client supports it)
    // 3. RemoteFX (fallback only)
    // 4. NSCodec (last resort)

    // Return best available codecs
}
```

#### 3.2 Implement Proper Error Recovery

**Current:** Errors logged but not handled

**Need:**
- Reconnection support
- Graceful degradation (fallback codecs)
- Frame error recovery
- Clipboard retry logic

---

### Phase 4: Performance Optimization

#### 4.1 Damage Tracking Integration

**Goal:** Only encode changed screen regions

**Sources of damage info:**
- PipeWire damage metadata
- Frame differencing
- Application hints

**Implementation:**
- Parse SPA damage region metadata from PipeWire
- Merge adjacent/overlapping regions
- Skip encoding if damage < threshold
- Send only damaged rectangles

#### 4.2 Adaptive Bitrate

**Current:** Fixed 4000 kbps

**Proper:**
```rust
struct AdaptiveBitrate {
    current_bitrate: u32,
    target_latency_ms: u32,
    measure_bandwidth: bool,
}

impl AdaptiveBitrate {
    fn adjust_for_network_conditions(&mut self, rtt_ms: u32, packet_loss: f32);
    fn adjust_for_content(&mut self, motion_score: f32);
}
```

---

## TECHNICAL DEBT TO ADDRESS

### Issue 1: RemoteFX Deprecation

**Problem:** Microsoft deprecated RemoteFX in 2020 (security issues)

**Timeline:**
- Windows 10 2004: Disabled by default
- Windows 11: Removed entirely
- Modern RDP clients: May not support it

**Action:** MUST migrate to H.264/EGFX

### Issue 2: Frame Copying

**Current:** Copying 4MB per frame (MemFd buffers)

**Should:** Use DMA-BUF (zero-copy GPU memory)

**Why not working:**
- Format negotiation issue?
- GPU not available?
- Need to investigate

### Issue 3: False Capability Advertising

**Current:** Advertising STREAM_FILECLIP_ENABLED without implementation

**Options:**
1. Implement it fully (preferred)
2. Would need to remove flag and break compatibility claims

**Decision:** IMPLEMENT IT (this is your requirement)

---

## IMPLEMENTATION ORDER

### Week 1: Core Video (Priority: CRITICAL)

**Day 1-2:**
1. Integrate EGFX/H.264 encoder with display pipeline
2. Replace RemoteFX with H.264 as primary codec
3. Test encoding performance
4. Verify client compatibility

**Day 3-4:**
5. Implement damage region coalescing
6. Optimize graphics drain pipeline
7. Test bandwidth savings

**Day 5:**
8. Fix zero-size buffer handling
9. Investigate DMA-BUF usage
10. Performance testing

### Week 2: Clipboard Completeness

**Day 1-3:**
1. Implement FileContents request/response handlers
2. File I/O logic (read/write with chunking)
3. File descriptor parsing
4. Test file copy/paste both directions

**Day 4-5:**
5. Add image MIME type support (PNG, JPEG, BMP)
6. Add HTML/RTF support
7. Test all MIME types

### Week 3: Testing & Polish

**Day 1-2:**
1. Multi-monitor testing
2. Stress testing (high load, many pastes)
3. Error recovery testing

**Day 3-5:**
4. Performance optimization
5. Fix any bugs found
6. Documentation updates

---

## QUESTIONS NEEDING RESEARCH

1. **EGFX Integration:** How exactly does IronRDP server integrate DVC processors?
2. **Damage Regions:** How to extract from PipeWire SPA metadata?
3. **DMA-BUF:** Why isn't it being used? Format issue?
4. **File Transfer:** What's the exact FILEDESCRIPTOR structure layout?
5. **Image Conversion:** Best library for PNG ↔ DIB conversion?

---

## NEXT IMMEDIATE ACTIONS

Since this is substantial work, I should:

1. Research EGFX integration with IronRDP (understand the API)
2. Create detailed design for frame coalescing
3. Plan file transfer architecture
4. Then implement systematically

Do you want me to:
A) Start researching and designing the EGFX integration?
B) Start implementing file transfer first?
C) Something else?

This is 2-3 weeks of solid implementation work to do it right.
