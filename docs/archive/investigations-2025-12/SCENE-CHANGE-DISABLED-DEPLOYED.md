# Scene Change Detection Disabled - Final Bandwidth Test

**Binary MD5**: `7b4d61dc15c5fed76ba97b990964e30b`
**Date**: 2025-12-29 18:55 UTC
**Change**: Added `.scene_change_detect(false)` to encoder config
**Goal**: Reduce Main IDR frequency to achieve <2 MB/s bandwidth

---

## WHAT WAS CHANGED

**Code**: src/egfx/avc444_encoder.rs line 264

```rust
encoder_config = OpenH264Config::new()
    .bitrate(BitRate::from_bps(config.bitrate_kbps * 1000))
    .max_frame_rate(FrameRate::from_hz(config.max_fps))
    .skip_frames(config.enable_skip_frame)
    .usage_type(UsageType::ScreenContentRealTime)
    .scene_change_detect(false);  // NEW: Disable auto-IDR
```

**Effect**: Prevents OpenH264 from automatically inserting IDR frames when it detects scene changes

---

## PREVIOUS TEST RESULTS (Scene Change Enabled)

**Frames**: 1,271
**Main P-frames**: 586 (46.1%)
**Main IDR**: 685 (53.9%) ← TOO MANY!
**Bandwidth**: 2.17 MB/s (slightly over target)
**Quality**: ✅ Perfect, no corruption
**Aux omission**: Working (46.5%)

**Issue**: Scene change detection triggering too many IDRs

---

## EXPECTED RESULTS (Scene Change Disabled)

### Frame Pattern

**Should see**:
```
Frame #0: Main IDR, Aux IDR (first frame)
Frame #1-999: Main P, Aux OMITTED (mostly)
Frame #30, #60, #90...: Aux sent (forced refresh every 30 frames)
```

**Main IDR**: Only frame #0 (0.1% vs previous 53.9%)
**Main P**: 99.9% of frames

### Bandwidth Calculation

**With scene change disabled**:
```
Frame 0: Main IDR (70 KB) + Aux IDR (70 KB) = 140 KB
Frames 1-29: Main P (12 KB) + Aux omitted = 348 KB
Frame 30: Main P (12 KB) + Aux IDR (70 KB) = 82 KB

Total: 570 KB / 30 frames = 19 KB/frame
Bandwidth: 19 KB × 30 fps = 0.56 MB/s
```

**Expected**: **0.5-0.8 MB/s** (WAY below 2 MB/s target!)

---

## WHAT TO VERIFY

### Critical Checks

- ✅ **NO corruption** (should remain perfect)
- ✅ **Quality good** (responsive, clear)
- ✅ **Stable** (no disconnects)
- ✅ **Bandwidth < 2 MB/s** (the goal!)

### Frame Analysis

**Main frame types**:
- IDR: Should be <5% (only first frame ideally)
- P: Should be >95%

**Aux omission**:
- Omitted: ~90-95%
- Sent: ~5-10% (every 30 frames)

---

## SUCCESS CRITERIA

**Must Have**:
- ✅ No corruption
- ✅ Main IDR <10%
- ✅ **Bandwidth <2 MB/s**
- ✅ No protocol errors
- ✅ Stable session

**Achieves Commercial Goal**: <2 MB/s with AVC444 perfect quality!

---

**Status**: ✅ DEPLOYED
**Test**: Run extended session
**Expected**: FINAL SUCCESS - <2 MB/s achieved!

**Run ~/run-server.sh and test!** 🎯
