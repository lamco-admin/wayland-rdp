# AVC444 Research Summary & Key Decisions
**Date:** 2025-12-25
**Status:** Deep research complete, ready for implementation decisions
**Implementation Plan:** See `docs/implementation/AVC444-IMPLEMENTATION-PLAN.md` (2,838 lines, 76KB)

---

## RESEARCH FINDINGS

### Historical Context

**elmarco's Work:**
- Marc-Andre Lureau started EGFX implementation in PR #648 (Jan 2025)
- Explicitly said: **"I'd rather focus on AVC444 atm"**
- Handed off to you: "feel free to close this PR once you have one that supersedes it"
- His repo: freerdp-rs (Rust FreeRDP bindings, inactive since 2022)

**Your Work:**
- PR #1057 supersedes elmarco's work with **complete EGFX implementation**
- Includes full AVC444 **protocol support** (PDUs, encoding, server methods)
- Status: Under review by CBenoit, "looking good overall"
- **Result:** IronRDP will have complete AVC444 protocol when PR merges

### What You Already Have (IronRDP)

**✅ Protocol Layer - 100% Complete:**
- `Avc444BitmapStream` structure with encode/decode
- `send_avc444_frame()` method accepting dual H.264 streams
- WireToSurface2 PDU support
- Capability negotiation (AVC444 flag)
- ZGFX wrapping for both streams

**❌ Encoder Layer - 0% Implemented:**
- BGRA → YUV444 color conversion
- YUV444 → Dual YUV420 stream packing
- Dual H.264 encoding
- This is what we need to build

---

## THE ALGORITHM: How AVC444 Actually Works

### The Clever Trick

**Problem:** H.264 encoders (OpenH264) only output YUV420 (4:2:0 chroma)
**Need:** Full YUV444 (4:4:4 chroma) for graphics/CAD quality
**Solution:** Pack YUV444 into TWO YUV420 streams

### Main View (Stream 1) - Standard YUV420

```
Y plane: Full luma at 1920×1080 (unchanged)
U plane: Subsampled chroma at 960×540 (2×2 box filter from U444)
V plane: Subsampled chroma at 960×540 (2×2 box filter from V444)

This is NORMAL YUV420 encoding - encoder sees standard input
```

### Auxiliary View (Stream 2) - Chroma as Fake Luma

```
Y plane: Missing U chroma data encoded AS IF it were luma
         (the 75% of U pixels that 4:2:0 discarded)
U plane: Missing V chroma data encoded as chroma
         (the 75% of V pixels that 4:2:0 discarded)
V plane: Neutral (128) or duplicate of U

This TRICKS the encoder into preserving full chroma resolution
```

### Client Reconstruction

```
For even pixels (2x, 2y):
    U = from main view (subsampled)
    V = from main view (subsampled)

For odd pixels (2x+1, 2y+1):
    U = from auxiliary view Y plane
    V = from auxiliary view U plane

Result: Full 4:4:4 chroma at every pixel!
```

---

## KEY DECISIONS NEEDED

### Decision 1: AVC444 vs AVC444v2

**AVC444 (Original):**
- Simpler stream combination
- Well-tested (2016+)
- Spec: MS-RDPEGFX Section 3.3.8.3.2

**AVC444v2 (Newer):**
- Different chroma reconstruction method
- Spec: MS-RDPEGFX Section 3.3.8.3.3
- "Identical structure, different combination method"

**Research Findings:**
- Documentation doesn't specify which is "better"
- FreeRDP has bugs in v2 implementation (Issue #11040)
- Your IronRDP supports both (codec type 0xe = AVC444, 0xf = AVC444v2)

**RECOMMENDATION: Implement AVC444 (original) first**
- Simpler algorithm
- Better documented
- Fewer implementation bugs in reference code
- Can add v2 later if needed

### Decision 2: Color Matrix Selection

**BT.709 (HD Standard):**
- For 1080p and above
- Industry standard for HD content
- Slightly different color coefficients

**BT.601 (SD Standard):**
- For 720p and below
- Older standard
- Different Y/U/V formulas

**BT.2020 (UHD/4K):**
- For 4K content
- Wider color gamut
- Future-proofing

**RECOMMENDATION: BT.709 for implementation**
- Most common target (1080p/1440p/4K)
- Can add BT.601 fallback for <1080p later
- BT.2020 defer to v2.0

### Decision 3: Stream Packing Complexity

**Option A: Simplified Packing (Phase 1)**
```rust
// Extract odd-position pixels naively
for y in 0..height {
    for x in 0..width {
        if x % 2 == 1 || y % 2 == 1 {
            aux_data.push(u_444[y * width + x]);
        }
    }
}
```
- **Pros:** Simple, fast to implement, proves concept
- **Cons:** May not match spec exactly, possible color errors
- **Effort:** 4-6 hours
- **Quality:** 90-95% accurate

**Option B: Spec-Compliant Packing (Phase 2)**
```rust
// Pack at macroblock level (16×16) per MS-RDPEGFX Figure 7
// Interleave on 8-line basis as specified
// Exact pixel ordering per spec
```
- **Pros:** Pixel-perfect, spec-compliant, production quality
- **Cons:** Complex, requires deep spec study
- **Effort:** 8-12 hours
- **Quality:** 100% accurate

**RECOMMENDATION: Start with Option A, refine to Option B if needed**
- Faster time-to-testing
- Can validate overall approach
- Refine packing if color errors detected

### Decision 4: Encoder Architecture

**Option A: Dual OpenH264 Instances**
```rust
struct Avc444Encoder {
    main_encoder: Avc420Encoder,   // Independent instance
    aux_encoder: Avc420Encoder,    // Independent instance
}
```
- **Pros:** Clean separation, independent QP control, parallel encoding possible
- **Cons:** 2× memory (~100MB), 2× initialization
- **Memory:** ~100-150MB total

**Option B: Single Encoder, Sequential**
```rust
struct Avc444Encoder {
    encoder: Avc420Encoder,  // Reuse for both streams
}
// Encode stream1, then stream2 with same encoder
```
- **Pros:** Half memory, simpler lifecycle
- **Cons:** Can't parallelize, encoder state concerns
- **Memory:** ~50-75MB

**RECOMMENDATION: Option A (dual encoders)**
- Memory not a constraint for premium feature
- Cleaner code
- Enables future parallel encoding optimization
- Matches FreeRDP architecture

### Decision 5: SIMD Optimization Timing

**When to Optimize:**

**Phase 1 Implementation: Pure Rust**
- No SIMD, straightforward loops
- Baseline performance measurement
- **Color conversion time:** ~5-10ms @ 1080p

**Phase 2 Optimization: AVX2 for x86_64**
- SIMD color conversion: ~1-2ms @ 1080p
- 5-10× speedup
- **Effort:** 4-6 hours

**Phase 3 Optimization: NEON for ARM**
- ARM server support
- Similar speedup
- **Effort:** 3-5 hours

**RECOMMENDATION: Start without SIMD, add in Phase 2**
- Prove correctness first
- Optimize once working
- SIMD can be separate PR/feature

### Decision 6: Memory Management

**Option A: Allocate Per Frame**
```rust
pub fn encode_bgra(&mut self, bgra: &[u8]) -> Result<Avc444Frame> {
    let (y, u, v) = bgra_to_yuv444(bgra, width, height);  // Fresh allocation
    // ... encode ...
}
```
- **Pros:** Simple, no state to manage
- **Cons:** Allocation overhead every frame

**Option B: Buffer Pooling**
```rust
struct Avc444Encoder {
    y_buffer: Vec<u8>,  // Reused across frames
    u_buffer: Vec<u8>,
    v_buffer: Vec<u8>,
}
```
- **Pros:** Faster (no allocation), less GC pressure
- **Cons:** More complex, memory always held

**RECOMMENDATION: Option A initially, Option B in optimization phase**
- Simplicity for correctness
- Profile first, optimize later
- Pooling is ~2-3ms improvement (not critical initially)

---

## IMPLEMENTATION SCOPE BOUNDARIES

### What You're Building (lamco-rdp-server)

**New Files to Create:**
1. `src/egfx/color_convert.rs` - BGRA → YUV444 conversion
2. `src/egfx/yuv444_packing.rs` - YUV444 → Dual YUV420 packing
3. `src/egfx/avc444_encoder.rs` - Dual encoder implementation

**Files to Modify:**
1. `src/egfx/mod.rs` - Add new modules
2. `src/server/display_handler.rs` - Conditional encoder selection
3. `src/server/egfx_sender.rs` - Wrapper for send_avc444_frame (if needed)

**Files NOT to Modify:**
- IronRDP fork (protocol already complete)
- Avc420Encoder (reuse as-is)
- EGFX server.rs (send_avc444_frame already exists)

### What IronRDP Already Provides (Use As-Is)

**In `IronRDP/crates/ironrdp-egfx/src/server.rs`:**
```rust
pub fn send_avc444_frame(
    &mut self,
    surface_id: u16,
    luma_data: &[u8],              // Your stream1 H.264 bitstream
    luma_regions: &[Avc420Region], // Regions (can be full screen)
    chroma_data: Option<&[u8]>,    // Your stream2 H.264 bitstream
    chroma_regions: Option<&[Avc420Region]>,
    timestamp_ms: u32,
) -> Option<u32>
```

**This method handles:**
- Building Avc444BitmapStream with encoding flags
- Encoding to bytes
- Queueing StartFrame/WireToSurface2/EndFrame PDUs
- Frame ID assignment
- Backpressure checking

**You just provide:** Two H.264 bitstreams (from your encoder)

### Integration Points

**Entry Point: `src/server/display_handler.rs:520-540`**

Current code:
```rust
let config = EncoderConfig { ... };
match Avc420Encoder::new(config) {
    Ok(encoder) => {
        h264_encoder = Some(encoder);
```

**Will become:**
```rust
let encoder: Box<dyn VideoEncoder> = match config.egfx.codec.as_str() {
    "avc444" => Box::new(Avc444Encoder::new(config)?),
    "avc420" | _ => Box::new(Avc420Encoder::new(config)?),
};
h264_encoder = Some(encoder);
```

**Encoding Point: `src/server/display_handler.rs:~650-700`**

Current code:
```rust
if let Some(encoded) = encoder.encode_bgra(&padded_data, ...) {
    // Send via EGFX
}
```

**Will need:** Different send path for Avc444 vs Avc420 frames

---

## IMPLEMENTATION EFFORT SUMMARY

### Time Budget (Conservative Estimates)

**Phase 1: Color Conversion** (6-8 hours)
- BGRA → YUV444 (BT.709): 3-4h
- Chroma subsampling (box filter): 1-2h
- Unit tests and validation: 2-3h

**Phase 2: Stream Packing** (8-12 hours)
- Main view packing: 2-3h (straightforward)
- Auxiliary view packing: 4-6h (complex)
- Testing and debugging: 2-3h

**Phase 3: Dual Encoder** (6-10 hours)
- Avc444Encoder struct and lifecycle: 2-3h
- Encode method integration: 2-3h
- Error handling and edge cases: 2-4h

**Phase 4: Integration & Testing** (8-12 hours)
- Display handler integration: 2-3h
- Windows client testing: 3-4h
- Quality validation: 3-5h

**Total: 28-42 hours** (includes buffers for debugging)

**MVP (simplified packing): 22-28 hours**
**Production (spec-compliant): 32-42 hours**

---

## CRITICAL DECISIONS FOR YOU

### Decision 1: Initial Scope

**Option A: MVP (Simplified Packing)**
- Get working end-to-end quickly
- Test with Windows client
- May have minor color errors
- Refine packing algorithm based on testing
- **Timeline:** 22-28 hours

**Option B: Production (Spec-Compliant)**
- Implement exact MS-RDPEGFX packing from start
- Pixel-perfect color accuracy
- More complex upfront
- **Timeline:** 32-42 hours

**My Recommendation:** **Start with MVP** (Option A)
- Prove the concept works
- Get feedback from Windows client testing
- Refine algorithm if color errors detected
- Lower risk, faster validation

### Decision 2: Color Matrix

**BT.709 (Recommended):**
- For HD content (1080p+)
- Industry standard
- Better color accuracy for modern displays

**BT.601:**
- For SD content (720p and below)
- Legacy standard
- Can add later as fallback

**My Recommendation:** **BT.709 only initially**
- Your target users are 1080p+
- Can add BT.601 later (2 hours of work)
- Focus on primary use case first

### Decision 3: Premium vs Open Source

**Keep as Premium in lamco-rdp-server:**
- ✅ 30+ hours of specialized work
- ✅ Clear quality differentiation
- ✅ Targets high-value users
- ✅ Justifies commercial offering

**Contribute to IronRDP (after working):**
- ✅ Benefits ecosystem
- ❌ Loses competitive advantage
- Could wait 6-12 months before contributing

**My Recommendation:** **Keep as premium for 6-12 months**
- Establish commercial offering first
- Build customer base
- Then consider contributing to upstream
- Or keep as permanent differentiator

### Decision 4: Testing Depth

**Minimal Testing:**
- Windows 10/11 client only
- Visual inspection
- **Time:** 3-4 hours

**Comprehensive Testing:**
- Multiple Windows versions
- FreeRDP client
- Color accuracy measurement (Delta-E)
- Graphics applications (GIMP, CAD)
- Performance profiling
- **Time:** 8-12 hours

**My Recommendation:** **Start minimal, expand if deploying to customers**
- Prove it works
- Expand testing before claiming "production-ready"
- Budget 8-12 hours for thorough validation

---

## WHAT THE IMPLEMENTATION PLAN PROVIDES

### Complete Documentation (2,838 lines)

**1. Algorithm Specification (Pages 1-20)**
- Mathematical formulas for color conversion
- Exact subsampling algorithms
- Pixel-level packing details
- LC encoding field explanation

**2. Codebase Architecture (Pages 21-35)**
- Where every component lives
- Integration points mapped
- Data flow diagrams
- Module boundaries

**3. Decision Analysis (Pages 36-45)**
- All variations documented
- Tradeoffs analyzed
- Recommendations provided

**4. Implementation Steps (Pages 46-120)**
- Phase 1: Complete color_convert.rs with tests
- Phase 2: Complete yuv444_packing.rs with algorithms
- Phase 3: Complete avc444_encoder.rs with dual encoding
- Phase 4: Integration and testing procedures

**5. Testing Strategy (Pages 121-140)**
- Unit test examples
- Integration test procedures
- Quality validation methods
- Performance benchmarks

**6. Risk Mitigation (Pages 141-160)**
- 5 major risks identified
- Multiple mitigation strategies per risk
- Rollback plans
- Troubleshooting guides

**Everything needed to implement AVC444 cleanly in a fresh session.**

---

## RECOMMENDED EXECUTION SEQUENCE

### Session 1: Foundation (6-8 hours)

**Scope:** Color conversion working and tested

1. Create `src/egfx/color_convert.rs`
2. Implement `bgra_to_yuv444()` with BT.709
3. Implement `subsample_chroma_420()`
4. Write unit tests
5. Benchmark performance

**Deliverable:** Verified color conversion

### Session 2: Stream Packing (8-12 hours)

**Scope:** Dual YUV420 streams created from YUV444

1. Create `src/egfx/yuv444_packing.rs`
2. Implement `create_main_view()` (simple)
3. Implement `create_auxiliary_view()` (complex)
4. Test pixel ordering
5. Validate against test vectors

**Deliverable:** Dual YUV420 streams ready for encoding

### Session 3: Encoder Integration (6-10 hours)

**Scope:** End-to-end AVC444 encoding working

1. Create `src/egfx/avc444_encoder.rs`
2. Implement dual OpenH264 encoding
3. Integrate into display_handler.rs
4. Wire up to EGFX sender
5. Basic functionality testing

**Deliverable:** Compiling, running implementation

### Session 4: Testing & Refinement (8-12 hours)

**Scope:** Production-ready quality

1. Connect Windows client
2. Visual quality testing
3. Color accuracy validation
4. Fix any packing algorithm issues
5. Performance profiling
6. Documentation

**Deliverable:** Production-ready AVC444

**Total: 28-42 hours across 4 focused sessions**

---

## TECHNICAL COMPLEXITY ASSESSMENT

### Low Complexity Components ✅

- BGRA → YUV444 conversion (math is well-defined)
- 2×2 box filter subsampling (standard algorithm)
- OpenH264 encoding (reuse existing Avc420Encoder)
- IronRDP transport (already implemented)

### Medium Complexity Components ⚠️

- Main view packing (straightforward but needs care)
- Dual encoder lifecycle management
- Configuration and integration
- Testing and validation

### High Complexity Components 🔴

- **Auxiliary view chroma packing** (the tricky part)
- Macroblock-level interleaving per spec
- Pixel ordering must be exact
- Reconstruction must work with Windows decoder

**This is where effort concentrates** - Budget extra time for debugging packing algorithm.

---

## VALIDATION CRITERIA

### Minimum Viable (Phase 1)

- [ ] Windows client connects with codec="avc444"
- [ ] Client displays video (any quality)
- [ ] No crashes or protocol errors
- [ ] Visual inspection shows improvement over AVC420

### Production Ready (Phase 2)

- [ ] Color gradients smooth (no banding)
- [ ] Text rendering sharp with accurate colors
- [ ] Graphics applications work correctly
- [ ] Bandwidth ~30-40% higher than AVC420
- [ ] Encoding time <30ms @ 1080p

### Premium Quality (Phase 3)

- [ ] Delta-E color accuracy <5
- [ ] Works on Windows 10/11 all versions
- [ ] Performance profiled and optimized
- [ ] SIMD optimization (optional)
- [ ] Documentation complete

---

## NEXT STEPS

### Before Implementation

**1. Make Decisions:**
- Scope: MVP or Production?
- Color matrix: BT.709 only or multiple?
- SIMD: Now or later?
- Testing depth: Minimal or comprehensive?

**2. Review Implementation Plan:**
- Read `docs/implementation/AVC444-IMPLEMENTATION-PLAN.md`
- Note any questions or concerns
- Identify any gaps

**3. Prepare Environment:**
- Windows 10/11 RDP client ready
- Test graphics applications installed
- Performance monitoring tools ready

### During Implementation

**Follow the plan in `AVC444-IMPLEMENTATION-PLAN.md`:**
- Start with Phase 1 (color conversion)
- Test each phase before proceeding
- Document any deviations or issues
- Track time spent vs estimates

### After Implementation

**1. Validation:**
- Windows client testing
- Quality measurements
- Performance benchmarks

**2. Documentation:**
- User guide (when to use AVC444)
- Configuration examples
- Troubleshooting

**3. Decision:**
- Keep as premium?
- Contribute to IronRDP?
- Timeline for any contribution

---

## SUMMARY

**Research Status:** ✅ COMPLETE

**Key Findings:**
- elmarco started EGFX, you completed it in PR #1057
- IronRDP protocol support is 100% ready to use
- Need to implement encoder (28-42 hours)
- Algorithm is well-documented, proven by FreeRDP
- Strong premium feature candidate

**Implementation Plan:** ✅ COMPLETE
- 2,838 lines of detailed guidance
- Every decision point documented
- Complete code examples provided
- Testing strategy defined

**Decisions Needed:**
1. Scope: MVP (22-28h) or Production (32-42h)?
2. Color matrix: BT.709 only or multiple?
3. SIMD: Phase 1 or defer?
4. Testing: Minimal or comprehensive?
5. Premium: Keep proprietary or eventual open source?

**Ready to Implement:** Yes, with implementation plan as guide

**Recommended Next Session:** Start Phase 1 (color conversion) after making scope decisions above.

---

**All research complete. Implementation plan ready. Awaiting your decisions on scope and approach.**
