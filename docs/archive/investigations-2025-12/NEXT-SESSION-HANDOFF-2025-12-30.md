# Next Session Handoff - December 30, 2025

**Previous session**: 2025-12-29 (AVC444 optimization complete)
**Current status**: Production-ready at 0.93 MB/s, all configs working
**Next focus**: Wayland-aligned enhancements (Product A - lamco-rdp-server)

---

## CURRENT STATE SUMMARY

### ✅ What's Complete and Working

**AVC444 Bandwidth Optimization**:
- Single encoder architecture (MS-RDPEGFX spec compliant)
- Auxiliary stream omission (92.3% skip rate)
- Scene change detection disabled
- **Bandwidth**: 0.93 MB/s (way below 2 MB/s target) ✅
- **Quality**: Perfect, no corruption ✅
- **Commits**: d8a1a35, ac56d6f, e1a2dea, 9f4d957, b6fcc3a

**Configuration System**:
- All config.toml values now functional ✅
- Damage tracking configurable ✅
- Aux omission configurable ✅
- Both features working correctly ✅

**Multimonitor**:
- Code complete (1,584 lines) ✅
- Code reviewed and validated ✅
- Awaiting real hardware test (deferred to final testing)

**Infrastructure**:
- Video: RemoteFX, AVC420, AVC444 all working
- Hardware encoding: VA-API, NVENC implemented
- Clipboard: Text, images (PNG/JPEG), files working
- Input: Keyboard, mouse working
- All published crates up to date

---

## ROADMAP: 5 PRIORITIES (Product A Focus)

**Your directive**: Focus on lamco-rdp-server (existing compositor support), not headless compositor

**Order** (as you specified):
1. **DIB/DIBV5 Support** (11-15 hours) - Complete clipboard with alpha
2. **Capability Probing** (16-20 hours) - Auto-adapt to each DE
3. **Adaptive FPS** (8-12 hours) - Damage-driven frame rate
4. **Latency Governor** (6-8 hours) - Professional modes
5. **Cursor Strategies** (8-10 hours) - Smart cursor handling

**Total effort**: ~50-65 hours (2-3 weeks)

---

## IMPLEMENTATION PLANS (All Ready)

### Priority 1: DIB/DIBV5 Clipboard Support

**Document**: `IMPLEMENTATION-PLAN-PRIORITY-1-DIBV5.md`

**Current state**:
- ✅ CF_DIB (format 8) fully implemented in lamco-clipboard-core
- ❌ CF_DIBV5 (format 17) missing

**Gap**:
- Transparency lost in clipboard operations
- Modern Windows apps use DIBV5 for alpha
- Professional use case (designers, screenshots)

**Implementation**:
1. Extend `lamco-clipboard-core/src/image.rs` (follows existing pattern)
2. Add `png_to_dibv5()`, `dibv5_to_png()` functions
3. 124-byte BITMAPV5HEADER (vs 40-byte DIB)
4. Handle "short DIBV5" compatibility bug
5. Integrate in `src/clipboard/manager.rs`
6. Publish lamco-clipboard-core 0.5.0

**Effort**: 11-15 hours
**Impact**: Complete clipboard image support with alpha

**Key files**:
- `../lamco-rdp-workspace/crates/lamco-clipboard-core/src/image.rs` (extend)
- `src/clipboard/manager.rs` (integrate)

**Test**: Windows screenshot with transparency → Linux, preserves alpha

---

### Priority 2: Capability Probing

**Document**: `IMPLEMENTATION-PLAN-PRIORITY-2-CAPABILITY-PROBING.md`

**Goal**: Auto-detect compositor and adapt configuration

**What to detect**:
- Compositor type (GNOME, KDE, Sway, etc.)
- Wayland protocol support
- Portal capabilities
- Buffer type preferences
- Quirks and workarounds needed

**Implementation**:
1. New module `src/compositor/`
2. Probe Wayland globals via wayland-client
3. Query Portal version and capabilities
4. Create compositor profiles
5. Apply configuration automatically

**Effort**: 16-20 hours
**Impact**: "Just works" on all DEs

**Benefit**: No manual per-DE configuration, graceful adaptation

---

### Priority 3: Adaptive FPS

**Document**: `IMPLEMENTATION-PLAN-PRIORITY-3-ADAPTIVE-FPS.md`

**Goal**: Adjust frame rate based on screen activity

**Modes**:
- Static screen (wallpaper): 5 FPS
- Low activity (typing): 15 FPS
- Medium (scrolling): 20 FPS
- High (video, dragging): 30 FPS

**Implementation**:
1. New module in lamco-video: `adaptive_fps.rs`
2. Track damage history (rolling window)
3. Calculate average activity level
4. Adjust FPS dynamically
5. Integrate with damage tracking

**Effort**: 8-12 hours
**Impact**: 30-50% CPU reduction for typical desktop

**Test**: Static screen should drop to ~10% CPU of full-rate

---

### Priority 4: Latency Governor

**Document**: `IMPLEMENTATION-PLAN-PRIORITY-4-LATENCY-GOVERNOR.md`

**Goal**: Configurable latency vs quality tradeoffs

**Modes**:
- **Interactive**: <50ms latency (gaming, CAD)
- **Balanced**: <100ms latency (default)
- **Quality**: <300ms latency (design, photo editing)

**Implementation**:
1. New module in lamco-video: `latency_governor.rs`
2. Define mode-specific policies
3. Frame accumulation logic
4. Encoding decision engine
5. Latency metrics tracking

**Effort**: 6-8 hours
**Impact**: Professional feature, competitive differentiation

**Test**: Interactive mode feels instant, Quality mode has best compression

---

### Priority 5: Cursor Strategies

**Document**: `IMPLEMENTATION-PLAN-PRIORITY-5-CURSOR-STRATEGIES.md`

**Goal**: Smart cursor handling for different scenarios

**Modes**:
- Metadata (current): Client draws cursor
- Painted: Cursor in video stream
- Separate stream: Dedicated cursor channel
- **Predictive**: Predict position (UNIQUE!)

**Implementation**:
1. New module `src/cursor/`
2. Cursor predictor (velocity + acceleration)
3. Multiple rendering strategies
4. Automatic mode selection based on latency
5. Configuration options

**Effort**: 8-10 hours
**Impact**: Unique feature, better UX over WAN

**Innovation**: Predictive cursor - no other RDP server has this!

---

## STARTING THE NEXT SESSION

### Step 1: Review Context (15 minutes)

**Read these documents IN ORDER**:
1. **FINAL-SUCCESS-ALL-CONFIGS-WORKING.md** - Current status
2. **This document** - Roadmap and plans
3. **DIB-DIBV5-ULTRATHINK-ANALYSIS.md** - Priority #1 deep dive
4. **WAYLAND-INNOVATIONS-ULTRATHINK.md** - Overall strategy

### Step 2: Verify Current State (10 minutes)

**Check committed code**:
```bash
git log --oneline -5
# Should show: b6fcc3a, 9f4d957, e1a2dea, ac56d6f, d8a1a35

git status
# Should be clean (all committed)
```

**Verify deployment**:
```bash
ssh greg@192.168.10.205 "md5sum ~/lamco-rdp-server ~/config.toml"
# Binary: 5dd7b9062ae5c892042d573203712834
# Config: 2e5ec1e57afc5ce5d9d07f14bc73c589
```

**Test if needed**:
```bash
ssh greg@192.168.10.205
~/run-server.sh
# Should show:
# - 🎯 Damage tracking ENABLED
# - AVC444 aux omission configured: enabled=true, force_idr=false
# - Bandwidth: ~0.9-1.0 MB/s
```

### Step 3: Start Implementation (Priority Order)

**Recommended sequence**:

**Week 1**: DIB/DIBV5 (11-15 hours)
- Completes clipboard feature
- High user value
- Clear implementation path

**Week 2**: Capability Probing (16-20 hours)
- Foundation for other features
- "Just works" quality

**Week 3**: Adaptive FPS + Latency Governor (14-20 hours)
- Performance improvements
- Professional features
- Can be done together

**Week 4**: Cursor Strategies (8-10 hours)
- Polish and innovation
- Unique differentiator

---

## KEY FILES TO UNDERSTAND

### Clipboard Architecture

**Where format conversions live**:
- `../lamco-rdp-workspace/crates/lamco-clipboard-core/src/image.rs` (401 lines)
- Currently has: png↔dib, jpeg↔dib, bmp↔dib
- **Add here**: dibv5 functions

**Where conversions are used**:
- `src/clipboard/manager.rs` (line ~1150 for RDP→Portal, ~1310 for Portal→RDP)
- **Add here**: format_id == 17 handling

### Video Pipeline Architecture

**Where performance features go**:
- `../lamco-rdp-workspace/crates/lamco-video/src/` (add new modules)
- `src/server/display_handler.rs` (integration)

**Existing**:
- Damage tracking: `src/damage/mod.rs` (1,000+ lines)
- Frame processing loop: `src/server/display_handler.rs` (line ~500+)

---

## DEPENDENCIES AND CRATES

### For DIB/DIBV5

**Already have**:
- `image` crate (decoding/encoding)
- `bytes` crate (BytesMut for header building)

**Pattern to follow**:
- Look at `create_dib_from_image()` in image.rs
- Extend with `create_dibv5_from_image()`
- Same pixel conversion, bigger header

### For Capability Probing

**Need to add**:
```toml
wayland-client = "0.31"  # For global enumeration
```

**Use existing**:
- ashpd (Portal queries)

### For Adaptive FPS / Latency / Cursor

**All in lamco-video** (already a dependency):
- No new external dependencies
- Use std collections (VecDeque, HashMap)
- Use tokio primitives (already available)

---

## TESTING APPROACH

### For Each Feature

**Unit tests**: In the module
**Integration tests**: End-to-end clipboard/capture tests
**Manual validation**: Real RDP client testing

**Test matrix**:
- Ubuntu 24.04 GNOME ✅ (current test VM)
- Windows 10/11 RDP client ✅
- Other DEs: When available

---

## WHAT NOT TO DO

**Don't**:
- Implement Wayland protocols in Product A (you can't - not the compositor)
- Add ext-image-copy-capture to RDP server (Portal is the right path)
- Overcomplicate - incremental, tested implementations

**Do**:
- Follow existing patterns in codebase
- Test thoroughly at each step
- Document as you go
- Commit frequently

---

## MEASUREMENT CRITERIA

### Priority 1 (DIB/DIBV5)

**Success**: Windows screenshot with transparency → Linux preserves alpha ✅

### Priority 2 (Capability Probing)

**Success**: Detects GNOME/KDE/Sway correctly, applies appropriate config ✅

### Priority 3 (Adaptive FPS)

**Success**: Static screen <10% CPU, video maintains 30 FPS ✅

### Priority 4 (Latency Governor)

**Success**: Interactive mode <50ms latency, Quality mode best compression ✅

### Priority 5 (Cursor)

**Success**: Cursor feels instant even with 100ms network latency ✅

---

## PRODUCTION TIMELINE

**Week 1**: DIB/DIBV5 (shipping clipboard completeness)
**Week 2**: Capability probing (professional quality)
**Week 3**: Performance features (competitive edge)
**Week 4**: Innovation features (market differentiation)

**End state**: Industry-leading Wayland RDP server with unique features

---

## IMPORTANT NOTES

### Configuration Defaults

**Remember**: Serde attributes override everything!
- Check `#[serde(default = "function_name")]`
- Match to actual desired defaults
- Both Default trait AND serde attribute must be correct

### Repository Structure

**lamco crates** (separate repo):
- `../lamco-rdp-workspace/crates/`
- Publish updates to crates.io
- Update version in lamco-rdp-server Cargo.toml

**lamco-rdp-server** (this repo):
- Main product code
- Integrates published crates
- Commercial license (BUSL 1.1)

### Deployment Workflow

**Always**:
1. Delete old binary on server
2. Copy new binary
3. Copy config.toml
4. Verify MD5s match
5. Test

---

## QUICK START COMMAND

```bash
# Read this file first
cat NEXT-SESSION-HANDOFF-2025-12-30.md

# Then read priority #1 plan
cat IMPLEMENTATION-PLAN-PRIORITY-1-DIBV5.md

# Start with:
cd ../lamco-rdp-workspace/crates/lamco-clipboard-core
# Edit src/image.rs
# Add DIBV5 functions following existing DIB pattern
```

---

## DOCUMENTS REFERENCE

**Implementation plans** (read in order):
1. IMPLEMENTATION-PLAN-PRIORITY-1-DIBV5.md
2. IMPLEMENTATION-PLAN-PRIORITY-2-CAPABILITY-PROBING.md
3. IMPLEMENTATION-PLAN-PRIORITY-3-ADAPTIVE-FPS.md
4. IMPLEMENTATION-PLAN-PRIORITY-4-LATENCY-GOVERNOR.md
5. IMPLEMENTATION-PLAN-PRIORITY-5-CURSOR-STRATEGIES.md

**Strategic docs**:
- WAYLAND-INNOVATIONS-ULTRATHINK.md (overall vision)
- DIB-DIBV5-ULTRATHINK-ANALYSIS.md (detailed #1 analysis)
- CURRENT-STATUS-AND-NEXT-STEPS.md (big picture)

**Session history**:
- FINAL-SUCCESS-ALL-CONFIGS-WORKING.md (latest achievement)
- SESSION-SUCCESS-2025-12-29.md (AVC444 complete)

---

## CODEBASE LANDMARKS

**Current binary**: 5dd7b9062ae5c892042d573203712834 (deployed and tested)

**Key modules**:
- `src/egfx/avc444_encoder.rs` - Single encoder, aux omission
- `src/server/display_handler.rs` - Main loop, damage tracking
- `src/clipboard/manager.rs` - Clipboard coordination
- `../lamco-rdp-workspace/crates/lamco-clipboard-core/src/image.rs` - Format conversions

**Configuration**:
- `config.toml` - All settings functional
- All serde attributes corrected
- Production defaults set

---

## ESTIMATED TIMELINE

**Realistic** (with testing and polish):
- Week 1: DIB/DIBV5 ✅ Ship clipboard completeness
- Week 2: Capability probing ✅ Professional quality
- Week 3-4: Performance features ✅ Competitive
- Week 5: Cursor strategies ✅ Innovation

**Aggressive** (minimal testing):
- 2-3 weeks for all 5 priorities

**Recommended**: Realistic timeline, ship incremental value

---

## SUCCESS DEFINITION

**After all 5 priorities**:

**Technical**:
- Complete clipboard (including alpha/transparency) ✅
- Works perfectly on GNOME/KDE/Sway (auto-detected) ✅
- Optimized performance (adaptive FPS, damage-driven) ✅
- Professional modes (latency vs quality) ✅
- Unique features (predictive cursor) ✅

**Market**:
- Industry-leading Wayland RDP server
- Commercial-ready product
- Competitive moat (unique features)
- Technical excellence (Wayland-native innovations)

---

## SESSION START CHECKLIST

When starting next session:

- [ ] Read this handoff document
- [ ] Review FINAL-SUCCESS-ALL-CONFIGS-WORKING.md
- [ ] Read IMPLEMENTATION-PLAN-PRIORITY-1-DIBV5.md
- [ ] Verify git status (should be clean)
- [ ] Verify deployment (binary + config MD5s)
- [ ] Test if needed (0.93 MB/s validation)
- [ ] **Start implementation**: DIB/DIBV5 first

---

## CONTACT POINTS

**If confused about**:
- Clipboard: Read manager.rs, check lamco-clipboard-core/src/image.rs
- Config: Check config/types.rs, look for serde attributes
- AVC444: Check egfx/avc444_encoder.rs, all working now
- Overall: Read CURRENT-STATUS-AND-NEXT-STEPS.md

**Key insight**: Follow existing patterns (DIB→DIBV5, damage→adaptive FPS)

---

**Status**: Ready for implementation
**Roadmap**: Clear and documented
**Foundation**: Solid (0.93 MB/s AVC444 working)
**Next**: Execute priorities 1-5 in order

**🎯 You have a clear path forward for 2-3 weeks of high-value work!**
