# Architecture Review - Current State
## Date: 2025-12-10
## Purpose: Factual assessment for decision-making

---

## CURRENT IMPLEMENTATION STATUS

### What's Working ✅

| Component | Status | Quality | Notes |
|-----------|--------|---------|-------|
| Video Streaming | ✅ Working | Good | 30 FPS, periodic refresh added |
| Keyboard Input | ✅ Working | Excellent | 10ms batching |
| Mouse Input | ✅ Working | Good | No issues |
| Text Clipboard L→W | ✅ Working | Excellent | Format names fixed |
| Text Clipboard W→L | ✅ Working | Excellent | Paste loop fixed |
| Graphics Multiplexer | ✅ Working | Good | Phase 1 only (graphics queue) |
| Frame Rate Regulation | ✅ Working | Excellent | 30 FPS target |

### What's Not Working ❌

| Component | Status | Reason |
|-----------|--------|--------|
| File Transfer | ❌ Not Implemented | Stubs only |
| Full Multiplexer | ⚠️ Partial | Phase 1 only (graphics) |
| Resolution Negotiation | ❌ Not Implemented | MS-RDPEDISP needed |
| H.264 Codec | ❌ Not Implemented | MS-RDPEGFX needed |
| Horizontal Lines | ⚠️ Mitigated | RemoteFX artifacts (periodic refresh added) |

---

## MULTIPLEXER IMPLEMENTATION

### Phase 1: Graphics Queue (Implemented)
- Graphics events isolated in bounded queue (4 frames)
- Drop/coalesce policy active
- Non-blocking sends

### Phase 2-4: Input/Control/Clipboard (Not Implemented)
- Would require forking IronRDP's event loop
- Estimate: 1-2 weeks work
- Benefit: Full priority control
- Complexity: High

---

## REMOTEFX STATUS

### Current Mitigation
- Periodic full-frame refresh every 10 seconds
- Should reduce horizontal line visibility

### Alternative Options
1. Damage regions (2-3 hours)
2. Quantization tuning (if API available)
3. H.264 migration (2-3 weeks)

---

## FILE TRANSFER STATUS

### What Exists
- IronRDP PDU structures (FileContentsRequest/Response)
- Stub handlers in ironrdp_backend.rs (lines 149-157)

### What's Needed
- FileGroupDescriptorW builder
- File streaming logic
- Portal file:// URI integration
- Temp file management

### Implementation Time Estimate
6-8 hours

---

## PROTOCOL COMPLETENESS

### MS-RDPECLIP (Clipboard)
- Text: ~90% complete
- Images: ~70% complete
- Files: ~30% complete (PDUs exist, no logic)

### MS-RDPEGFX (Graphics Pipeline)
- IronRDP: ~30% complete
- wrd-server: 0% (not started)

### MS-RDPEDISP (Display Control)
- IronRDP: ~50% complete
- wrd-server: 0% (not started)

---

## TECHNICAL DEBT

### Current Debt Items
1. **Periodic refresh** - Temporary RemoteFX mitigation (not a long-term solution)
2. **Phase 1 multiplexer only** - Partial implementation (but functional)
3. **IronRDP fork** - Using custom clipboard-fix branch
4. **Empty rectangles workaround** - Symptom fix, not root cause fix

### Not Debt
- Graphics queue implementation (intentional, sufficient design)
- Current codec choice (RemoteFX reasonable given constraints)

---

## DECISION POINTS

### Immediate Decisions Needed

**1. Test Periodic Refresh:**
- Does it reduce horizontal lines?
- Is 10-second interval acceptable?
- Need shorter/longer interval?

**2. File Transfer Priority:**
- Implement now (6-8 hours) vs later?
- Both directions or Linux→Windows first?
- Test scenarios to cover?

**3. Multiplexer Completion:**
- Keep Phase 1 only?
- Invest in Phase 2-4?
- Submit PR to IronRDP instead?

**4. Codec Strategy:**
- Continue with RemoteFX + mitigations?
- Plan H.264 migration timeline?
- Investigate damage regions?

---

## NEXT POSSIBLE WORK ITEMS

*Options presented for your prioritization:*

### Option A: File Transfer (6-8 hours)
- Complete MS-RDPECLIP implementation
- Enable file copy/paste

### Option B: H.264 Research (2-3 days)
- Assess IronRDP MS-RDPEGFX support
- Design migration plan
- Evaluate VA-API integration

### Option C: RemoteFX Optimization (2-4 hours)
- Test periodic refresh effectiveness
- Implement damage regions
- Research quantization tuning

### Option D: Full Multiplexer (1-2 weeks)
- Phase 2-4 implementation
- Fork IronRDP event loop
- Complete QoS system

### Option E: Resolution Negotiation (2-3 days)
- MS-RDPEDISP implementation
- Dynamic resize support
- Multi-monitor handling

### Option F: Testing & Stability (1 day)
- Automated test suite
- Long-running stability tests
- Performance benchmarking

---

## RESEARCH FINDINGS SUMMARY

### RemoteFX
- Deprecated by Microsoft (2020)
- Security vulnerabilities (CVE-2020-1036)
- Removed April 2021
- Lossy block-based codec
- Horizontal artifacts are characteristic

### Industry Standard
- H.264/AVC444 (RDP 8+)
- 3x faster than RemoteFX
- Hardware acceleration available
- Better quality
- No security issues

### Other Implementations
- FreeRDP: Multi-threaded, quantization tuning
- xrdp: 10ms damage timer, H.264 support
- Both use periodic refresh strategies

---

## CURRENT BUILD STATUS

**Binary:** target/release/wrd-server
**Deployed:** 192.168.10.3 (ready for testing)
**Changes:** Periodic refresh added (10-second interval)

**Test Command:** `./run-test-multiplexer.sh`

---

## END OF REVIEW
Awaiting your direction on priorities.
