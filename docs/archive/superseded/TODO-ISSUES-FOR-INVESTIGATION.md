# TODO: Issues for Further Investigation and Debugging

## Date: 2025-12-10
## Status: Deferred for Future Work

---

## ISSUE 1: Horizontal Lines in Video (Static Areas)

**Priority**: MEDIUM
**Severity**: Visual quality degradation
**Impact**: Lines appear in non-updating areas (white space, static backgrounds)

### Current Status
- ✅ Stride calculation verified correct (5,120 bytes/row)
- ✅ Buffer size matches expected (4,096,000 bytes for 1280x800)
- ✅ 16-byte alignment confirmed
- ❌ Horizontal lines persist in static content areas

### Hypotheses to Investigate

**1. Pixel Format Mismatch**
- **Current**: Assume BGRx format
- **Need**: Log actual negotiated format from PipeWire SPA parameters
- **Action**: Extract format from `param_changed` callback (line 513 of pw_thread.rs)
- **Files**: `src/pipewire/pw_thread.rs:513-518`

**2. Interlacing/Scan Order**
- **Check**: PipeWire buffer might be interlaced vs progressive
- **Action**: Parse SPA metadata flags for scan order
- **Look for**: SPA_META_VideoCrop, SPA_META_VideoScanOrder

**3. RemoteFX Codec Artifacts**
- **Evidence**: Static areas don't get re-encoded (delta compression)
- **Issue**: Initial frame corruption persists through session
- **Solutions**:
  - Force periodic full-frame refresh (every 5-10 seconds)
  - Use damage regions to force refresh of static areas
  - Test with lossless encoding mode
  - Consider H.264 codec instead of RemoteFX

**4. Color Space/Byte Order**
- **Check**: BGRA vs BGRX vs RGBA byte ordering
- **Issue**: Might be interpreting alpha channel incorrectly
- **Action**: Log first 64 bytes of pixel data, verify byte order

### Investigation Steps
1. Add pixel format logging in `param_changed` callback
2. Log first 64 bytes of buffer data (hex dump)
3. Test with periodic full-frame refresh
4. Try different RDP codecs if available

### Related Code
- `src/pipewire/pw_thread.rs:513-518` - Format negotiation callback
- `src/video/converter.rs` - Pixel format conversion
- `src/server/display_handler.rs` - Frame processing

---

## ISSUE 2: RemoteFX Encoding Slow Frames

**Priority**: LOW
**Severity**: Occasional frame lag
**Impact**: 2 frames out of 960 took 11-15ms to encode

### Current Status
- **Threshold**: IronRDP warns if encoding > 10ms
- **Occurrences**: 2 in 130-second session (99.8% frames encode fast)
- **Budget**: 33ms per frame @ 30 FPS (15ms still within budget)

### Observations
```
Frame 0: 15ms encoding time (cold start)
Frame 1: 11ms encoding time
Frames 2-960: <10ms encoding time
```

### Hypotheses
1. **Cold start penalty**: First frames initialize codec
2. **Content complexity**: Frames with lots of detail take longer
3. **Full-frame updates**: First frame is always full, subsequent are delta

### Action Items (Low Priority)
- Monitor if frequency increases over longer sessions
- Consider H.264 hardware encoding for better performance
- Implement damage regions to reduce encoded area

### Related Code
- IronRDP's RemoteFX encoder (external library)
- Consider MS-RDPEGFX implementation for better codecs

---

## ISSUE 3: Clipboard Format Request Failures (Self-Correcting)

**Priority**: LOW
**Severity**: Transparent to user
**Impact**: RDP client occasionally returns error for specific clipboard formats

### Pattern Observed
```
Serial 41: Requested CF_UNICODETEXT (13) → ERROR from RDP
Serial 42: Requested CF_TEXT (1) → ERROR from RDP
Serial 43: Requested CF_UNICODETEXT (13) → SUCCESS
Serial 44: Requested CF_UNICODETEXT (13) → SUCCESS
```

### Hypotheses
1. **Format not available**: Clipboard might not have specific variant
2. **Race condition**: Format list changes between announcement and request
3. **Windows RDP client quirk**: Known issue with format availability

### Investigation Needed
- Log which format ID triggered error
- Check if specific formats consistently fail
- Test with different Windows RDP client versions
- Consider fallback format order (try UTF16 first, then UTF8, then TEXT)

### Current Behavior
- ✅ System retries with different format
- ✅ Eventually succeeds
- ✅ User doesn't notice

### Action Items (Low Priority)
- Add format ID logging to error messages
- Implement smart format selection (prefer formats likely to succeed)
- Track format success rates to optimize request order

### Related Code
- `src/clipboard/ironrdp_backend.rs:368-404` - on_format_data_response
- `src/clipboard/manager.rs:1117-1175` - handle_rdp_data_error

---

## ISSUE 4: Paste Deduplication Still Allows 2 Writes (Minor)

**Priority**: LOW
**Severity**: Minor inconvenience
**Impact**: Sometimes 2 pastes instead of 1 (vs 45 before - major improvement)

### Observations
- Serial 43 at 09:57:13
- Serial 44 at 09:57:18 (4.5 seconds later)
- Both processed as "First SelectionTransfer"
- Suggests: These might be separate paste operations (user pasted twice)

### Questions
1. Are these truly duplicates or did user paste twice?
2. Should we track paste operations across longer time windows?
3. Is 2-second deduplication window too short?

### Investigation Needed
- Monitor logs over longer session
- Check if serials 43/44 came from distinct copy operations
- Consider increasing deduplication window to 5-10 seconds

### Current Status
✅ Acceptable (2 vs 45 is 95% improvement)

---

## ISSUE 5: File Transfer Not Implemented

**Priority**: HIGH (for next major feature)
**Severity**: Feature gap
**Impact**: Cannot copy/paste files between Windows and Linux

### What's Needed
Per MS-RDPECLIP specification, file transfer requires:

1. **FileGroupDescriptorW Structure** (MS-RDPECLIP 2.2.5.2)
   - Array of file descriptors (name, size, attributes, timestamps)
   - Parse from Portal file:// URIs
   - Convert to Windows file descriptor format

2. **FileContents Request/Response Protocol** (MS-RDPECLIP 2.2.5.3/2.2.5.4)
   - Handle CB_FILECONTENTS_REQUEST (0x0008)
   - Stream file data in chunks
   - Send CB_FILECONTENTS_RESPONSE (0x0009)

3. **Backend Integration**
   - Implement stubs at `clipboard/ironrdp_backend.rs:149-157`
   - Create `clipboard/file_transfer.rs` module
   - Handle Portal file APIs

### Scope Estimate
- **Time**: 3-5 days
- **Complexity**: MEDIUM (well-defined protocol)
- **Dependencies**: None (IronRDP PDUs exist, need business logic)

### Files to Create/Modify
- NEW: `src/clipboard/file_transfer.rs` (FileGroupDescriptor builder, file streamer)
- Modify: `src/clipboard/ironrdp_backend.rs` (implement file event handlers)
- Modify: `src/clipboard/manager.rs` (Portal file URI handling)

---

## ISSUE 6: Resolution Negotiation Not Implemented

**Priority**: MEDIUM
**Severity**: Functional limitation
**Impact**: Cannot resize RDP session dynamically

### Current Behavior
- Fixed resolution from Portal (1280x800 in current setup)
- No dynamic resize when RDP client changes resolution
- No multi-monitor support

### What's Needed
1. **MS-RDPEDISP Implementation**
   - Handle display control channel
   - Parse resolution change requests from client
   - Dynamically reconfigure PipeWire streams

2. **Portal Session Reconfiguration**
   - May need to recreate Portal session with new resolution
   - Or dynamically change stream parameters

3. **Multi-Monitor Support**
   - Handle multiple PipeWire streams
   - Coordinate transformations
   - Send proper monitor topology to client

### Scope Estimate
- **Time**: 2-3 days
- **Complexity**: MEDIUM
- **Dependencies**: May need IronRDP MS-RDPEDISP support

---

## ISSUE 7: MS-RDP Protocol Completeness

**Priority**: LONG-TERM
**Severity**: Feature gaps for production use
**Impact**: Missing advanced RDP features

### Protocols to Audit and Implement

| Protocol | Description | IronRDP Status | wrd-server Status | Priority |
|----------|-------------|----------------|-------------------|----------|
| MS-RDPBCGR | Core RDP | ~90% | N/A | - |
| MS-RDPECLIP | Clipboard | ~70% | ~70% | HIGH |
| MS-RDPEFS | File System Redirection | 0% | 0% | MEDIUM |
| MS-RDPEGFX | Graphics Pipeline (H.264) | ~30% | 0% | HIGH |
| MS-RDPEDISP | Display Control | ~50% | 0% | HIGH |
| MS-RDPDYC | Dynamic Channels | ~60% | 0% | MEDIUM |
| MS-RDPUDP | UDP Transport | 0% | 0% | LOW |
| MS-RDPMT | Multitransport | 0% | 0% | LOW |
| MS-RDPEAI | Audio Input | 0% | 0% | LOW |
| MS-RDPEVOR | Video Optimized Remoting | 0% | 0% | LOW |

### Action Plan
1. **Week 1**: Audit IronRDP source for each protocol
2. **Week 2**: Document gaps in `PROTOCOL-COMPLETENESS-MATRIX.md`
3. **Week 3-4**: Prioritize and implement critical missing features
4. **Decision**: Contribute to IronRDP vs maintain fork

---

## ISSUE 8: Graphics Congestion Can Still Block Other Channels

**Priority**: HIGH (Being Addressed)
**Severity**: Architecture limitation
**Impact**: Video frames can delay input/clipboard/control events

### Current Architecture
```
ALL Events → Single Unbounded Queue (IronRDP ServerEvent) → FIFO Processing
```

**Problem**: No priority, no QoS, no dropping

### Solution: Event Multiplexer (IN PROGRESS)
```
Input (32)     → Priority 1: Drain all
Control (16)   → Priority 2: Process 1
Clipboard (8)  → Priority 3: Process 1
Graphics (4)   → Priority 4: Drop/coalesce
```

**Status**: Module created (`src/server/event_multiplexer.rs`), integration in progress

---

## LOWER PRIORITY / NICE TO HAVE

### Issue 9: Clipboard Hash Cleanup Overhead
**Status**: ✅ FIXED (moved to background task)

### Issue 10: PipeWire Polling Jitter
**Status**: ✅ FIXED (non-blocking iterate + 5ms sleep)

### Issue 11: Per-Keystroke Task Spawning
**Status**: ✅ FIXED (10ms input batching)

### Issue 12: No Clipboard Timeout
**Status**: ✅ FIXED (5-second timeout implemented)

---

## TESTING TODO

### Remaining Tests Needed
1. **File copy/paste** (both directions) - Expected to fail until implemented
2. **Resolution change** - RDP client resize request
3. **Multi-monitor** - If test environment supports it
4. **Long session stability** (24+ hours)
5. **High load** (multiple concurrent connections)
6. **Different Windows RDP client versions**
7. **Different Linux desktop environments** (GNOME, Sway, etc.)

---

## NOTES FOR NEXT SESSION

### State of Codebase
- **Branch**: `feature/gnome-clipboard-extension`
- **Commit**: bdf6dc3 (input handler fix + comprehensive analysis)
- **Binary Deployed**: `wrd-server-paste-fix` (with all current fixes)

### Immediate Work
1. Complete event multiplexer integration (2-3 hours)
2. Test integrated system
3. Investigate horizontal lines if multiplexer doesn't help

### Future Roadmap
1. File transfer implementation (3-5 days)
2. Resolution negotiation (2-3 days)
3. Protocol completeness audit (1 week)
4. H.264/RDPEGFX implementation (2+ weeks)

### Performance Targets Met
- ✅ Typing latency: <50ms (was 200-500ms)
- ✅ Frame rate: 30 FPS stable (was unstable 35-70)
- ✅ Clipboard: 95%+ reliability (was 60-70%)
- ⚠️ Video quality: Functional but has artifacts
- ❌ QoS: Not yet implemented (multiplexer pending)

---

## End of TODO Document
Created: 2025-12-10
Last Updated: 2025-12-10 11:58 UTC
