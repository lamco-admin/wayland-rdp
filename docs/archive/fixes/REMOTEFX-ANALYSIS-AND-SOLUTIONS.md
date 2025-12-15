# RemoteFX Analysis and Solutions
## Date: 2025-12-10
## Status: Research Complete, Implementation Pending

---

## SCREENSHOT ANALYSIS ✅

### Observed Artifacts

Looking at screenshots 221-225, I can see:

**Pattern:**
- Faint horizontal lines in static/white areas
- Appears in Firefox window background (gray)
- Appears in LibreOffice Writer (white document area)
- Lines are thin (1-2 pixels)
- Consistent horizontal spacing
- More visible in uniform color regions

**Characteristics:**
- ✅ Stride calculation correct (verified in logs)
- ✅ Pixel format correct (BGRx verified)
- ✅ Byte order correct (hex dump normal)
- ❌ **Classic RemoteFX compression artifacts**

### Confirmation: RemoteFX Codec Artifacts

This matches the research findings perfectly:
- Block-based encoding (64x64 tiles) creates tile boundaries
- Lossy quantization introduces subtle color variations
- Delta compression means static areas never refresh
- Initial frame artifacts persist throughout session

---

## RESEARCH FINDINGS SUMMARY

### Key Insights from FreeRDP/xrdp

**1. RemoteFX is DEPRECATED by Microsoft**
- CVE-2020-1036: Remote code execution vulnerability
- Disabled July 2020, removed April 2021
- No architectural fix possible
- Industry moving to H.264/AVC444

**2. Common Mitigation Strategies**
- **Periodic refresh:** 10-40ms timer-based full-frame updates
- **Damage regions:** Only encode changed areas
- **Frame acknowledgement:** MS-RDPEGFX protocol
- **Quality tuning:** Quantization values 6-15 (lower = better)

**3. FreeRDP Approach**
- Default quantization: [6, 6, 6, 6, 7, 7, 8, 8, 8, 9]
- Multi-threaded tile processing (2.4+)
- 16-byte buffer alignment for SIMD
- Progressive RFX option (but has corruption bugs)

**4. xrdp Approach**
- 10ms damage tracking timer
- Frame intervals: 32ms for RFX (31.25 FPS)
- H.264 preferred (3x faster, half the size)
- Quality presets in gfx.toml

**5. Best Practices**
- **Short-term:** Periodic full refresh every 5-10 seconds
- **Medium-term:** Implement proper damage regions
- **Long-term:** Migrate to MS-RDPEGFX with H.264

---

## IMMEDIATE SOLUTIONS TO IMPLEMENT

### Solution 1: Periodic Full-Frame Refresh (QUICK WIN)

Force RemoteFX to re-encode everything every 10 seconds:

**Implementation:**
```rust
// In display_handler.rs start_pipeline()
let mut last_full_refresh = Instant::now();
const REFRESH_INTERVAL: Duration = Duration::from_secs(10);

// After frame processing:
let force_refresh = last_full_refresh.elapsed() >= REFRESH_INTERVAL;
if force_refresh {
    info!("🔄 Forcing full-frame refresh to clear codec artifacts");
    // Send a resize event to force full re-encode
    // OR send explicit refresh update
    last_full_refresh = Instant::now();
}
```

**Expected Result:**
- Lines disappear every 10 seconds
- Visual "flash" as frame refreshes
- Acceptable for short-term fix

**Time:** 30 minutes
**Complexity:** LOW

### Solution 2: Damage Region Tracking (BETTER)

Only encode changed areas (FreeRDP/xrdp standard):

**Implementation:**
- Track previous frame
- Compute diff rectangles
- Send only changed regions
- Forces more frequent updates of "static" areas

**Time:** 2-3 hours
**Complexity:** MEDIUM

### Solution 3: Quantization Tuning (TEST)

Reduce compression artifacts by lowering quantization:

**Current:** IronRDP default (unknown, likely medium)
**Test:** Can we configure IronRDP to use lower quantization?
**Need:** Check IronRDP RemoteFX encoder API

**Time:** 1 hour research + testing
**Complexity:** LOW (if API exists)

---

## LONG-TERM: H.264 MIGRATION PLAN

### Why H.264 is Superior

**From Research:**
- 3x faster encoding than RemoteFX
- 50% smaller bandwidth
- Hardware acceleration available (VA-API)
- No security vulnerabilities
- Industry standard (RDP 8+)
- AVC444 mode for sharp text

**Trade-offs:**
- Requires MS-RDPEGFX protocol implementation
- More complex than RemoteFX
- Need hardware encoder support

### Implementation Scope

**Phase 1: MS-RDPEGFX Foundation** (1 week)
- Implement graphics pipeline dynamic channel
- Frame acknowledgement protocol
- Reset graphics PDU handling
- Surface management

**Phase 2: H.264 Encoder Integration** (1 week)
- VA-API encoder bindings
- x264/openh264 fallback
- Quality/bitrate configuration
- Frame sequencing

**Phase 3: AVC444 Color Mode** (3-5 days)
- 4:4:4 chroma sampling
- Lossless text rendering
- Split YUV encoding

**Total Estimate:** 2-3 weeks

---

## MULTIPLEXER STATUS REVIEW

### What's Implemented ✅

**Phase 1: Graphics Queue ONLY**
- Graphics queue (bounded 4) with drop/coalesce
- Graphics drain task
- Non-blocking sends
- Statistics tracking

**What's NOT Implemented ❌**

**Phase 2-4: Input/Control/Clipboard Queues**
- Currently using IronRDP's single ServerEvent channel
- No priority-based routing for input/control/clipboard
- Cannot prioritize input over clipboard over graphics

### Why Full Multiplexer is Hard

**IronRDP Architecture:**
- Manages its own event loop in `RdpServer`
- Uses internal `ServerEvent` enum for all events
- Input/control generated inside IronRDP callbacks
- We can't easily intercept before they hit IronRDP's queue

**Options:**
1. **Fork IronRDP event loop** - High maintenance burden
2. **Partial multiplexer** (current) - 80% benefit, 20% effort
3. **Submit PR to IronRDP** - Add pluggable multiplexer hooks
4. **Accept limitations** - Phase 1 already prevents main issue

### Recommendation

**Keep Phase 1 (graphics only) for now:**
- Prevents graphics from blocking anything (primary goal)
- Low maintenance burden
- Full multiplexer has diminishing returns
- Focus effort on H.264 migration instead

---

## COMPREHENSIVE WORK PLAN

### Immediate (This Session - 2-3 hours)

**1. Implement Periodic Refresh** (30 min)
- Add 10-second timer to display pipeline
- Force full-frame update (resize trick or explicit update)
- Test and verify lines disappear periodically

**2. Research IronRDP RemoteFX Configuration** (30 min)
- Check if we can configure quantization
- Look for quality mode settings
- Test with lower compression if available

**3. Document Current State** (30 min)
- Multiplexer status (Phase 1 only)
- RemoteFX findings
- Architecture decisions

### Short-term (Next Session - 1 day)

**4. File Transfer Implementation** (6-8 hours)
Following TODO-ISSUES-FOR-INVESTIGATION.md:

**Phase A: FileGroupDescriptorW** (2-3 hours)
- Create `src/clipboard/file_descriptor.rs`
- Parse Portal file:// URIs
- Build Windows file descriptor structure
- Handle metadata (size, timestamps, attributes)

**Phase B: FileContents Streaming** (3-4 hours)
- Create `src/clipboard/file_streamer.rs`
- Handle FileContentsRequest PDU
- Stream file data in chunks
- Implement FileContentsResponse

**Phase C: Integration** (1-2 hours)
- Wire up ironrdp_backend.rs stubs (lines 149-157)
- Portal file URI handling in manager.rs
- Error handling and timeouts

### Medium-term (1-2 Weeks)

**5. MS-RDPEGFX Investigation** (2-3 days)
- Audit IronRDP's MS-RDPEGFX support
- Assess completeness and gaps
- Design integration strategy

**6. H.264 Encoder Research** (2-3 days)
- VA-API bindings evaluation
- x264/openh264 comparison
- Performance testing

**7. Damage Region Implementation** (2-3 days)
- Frame diffing algorithm
- Rectangle packing optimization
- Integration with bitmap converter

### Long-term (3-4 Weeks)

**8. MS-RDPEGFX + H.264 Implementation**
- Full graphics pipeline
- Hardware encoding
- AVC444 color mode
- Frame acknowledgement

**9. Resolution Negotiation (MS-RDPEDISP)**
- Dynamic resize support
- Multi-monitor configuration
- PipeWire stream reconfiguration

---

## DECISION POINTS

### Should We Invest in RemoteFX Improvements?

**Arguments FOR:**
- Quick wins available (periodic refresh)
- Immediate visual quality improvement
- Users can benefit now

**Arguments AGAINST:**
- RemoteFX is deprecated and removed by Microsoft
- Effort better spent on H.264 migration
- Periodic refresh is a hack, not a solution
- Technical debt

**Recommendation:**
✅ **Implement periodic refresh** (30 min for immediate relief)
❌ **Don't invest more** in RemoteFX optimization
✅ **Focus on H.264 migration** (proper long-term solution)

### Multiplexer: Phase 1 vs Full Implementation?

**Phase 1 Benefits (Current):**
- Graphics isolation (main goal achieved)
- Low maintenance
- Simple integration

**Full Multiplexer Benefits:**
- Complete QoS control
- Input priority over clipboard
- More granular statistics

**Complexity Gap:**
- Phase 1: ✅ Done (2 hours)
- Phase 2-4: 1-2 weeks (fork IronRDP event loop)

**Recommendation:**
✅ **Keep Phase 1** for now
⏸️ **Defer Phase 2-4** until real-world need emerges
✅ **Document as technical debt** in case needed later

---

## NEXT SESSION PRIORITIES (IN ORDER)

### 1. Quick RemoteFX Improvement (30-60 min)
- Implement 10-second periodic refresh
- Test and verify line reduction
- Document as temporary mitigation

### 2. File Transfer Implementation (6-8 hours)
- MS-RDPECLIP FileContents protocol
- FileGroupDescriptorW structure
- Portal file URI integration
- Both directions: Linux→Windows and Windows→Linux

### 3. Architecture Review (1-2 hours)
- Document multiplexer decision (Phase 1 sufficient)
- Assess IronRDP MS-RDPEGFX support
- Create H.264 migration roadmap
- Prioritize protocol implementations

### 4. Testing Infrastructure (2-3 hours)
- Create automated test suite
- Multi-MIME type clipboard tests
- File transfer test scenarios
- Performance benchmarks

---

## FILES TO CREATE/MODIFY

### This Session
1. `src/server/display_handler.rs` - Add periodic refresh timer
2. `REMOTEFX-MITIGATION-SUMMARY.md` - Document findings and plan
3. `MULTIPLEXER-STATUS-FINAL.md` - Phase 1 complete, rationale

### Next Session
1. NEW: `src/clipboard/file_descriptor.rs` - FileGroupDescriptorW builder
2. NEW: `src/clipboard/file_streamer.rs` - File content streaming
3. MODIFY: `src/clipboard/ironrdp_backend.rs` - Wire up file handlers
4. MODIFY: `src/clipboard/manager.rs` - Portal file URI support
5. NEW: `ARCHITECTURE-DECISION-RECORD.md` - Multiplexer, codec, roadmap

---

## END OF ANALYSIS
Ready to implement periodic refresh and proceed with file transfer!
