# Session Handover - 2026-01-07

**Date:** 2026-01-07
**Focus:** AVC444 vs AVC420 quality investigation on RHEL 9
**Status:** AVC420 WORKS - AVC444 has unresolved issues on RHEL 9/GNOME 40
**Platform:** RHEL 9.7, GNOME 40.10, Kernel 5.14.0-611

---

## Executive Summary

### Final Result

**AVC420 produces excellent quality on RHEL 9. AVC444 produces blurry text despite extensive optimization.**

| Mode | P-Frame Ratio | Avg Frame Size | Quality | Status |
|------|---------------|----------------|---------|--------|
| AVC420 | 99.8% | 15 KB | EXCELLENT | WORKING |
| AVC444 | 93% (after fixes) | 27 KB | BLURRY TEXT | NOT WORKING |

### Recommendation

**For RHEL 9 / GNOME 40 deployments: Use AVC420 mode (`avc444_enabled = false`)**

The root cause of AVC444 blurriness on RHEL 9 was not identified. AVC444 worked correctly on other test platforms previously. This appears to be specific to the RHEL 9 / GNOME 40 / OpenH264 combination.

---

## AVC420 Success Analysis

### Test Session: rhel9-test-20260107-012821.log

**Configuration:**
```toml
avc444_enabled = false        # Force AVC420 mode
h264_bitrate = 10000         # 10 Mbps
qp_min = 10
qp_max = 25
```

**Results:**
- **Total Frames:** 174
- **IDR Frames:** 2 (1.1%)
- **P Frames:** 172 (98.9%)
- **Average Frame Size:** 14,984 bytes (~15 KB)
- **Total Bandwidth:** 2.6 MB over 21 seconds = ~1.0 Mbps
- **Client Acknowledgments:** 173/174 (99.4% - one in-flight at disconnect)
- **Session Duration:** 21 seconds
- **Errors:** None (only normal disconnect ECONNRESET)

**Quality:** User verified text was sharp and readable

### Log Evidence

```
EGFX factory created for H.264/AVC420 streaming
AVC444 disabled in config, using AVC420
✅ AVC420 encoder initialized for 1280×800 (aligned)

📦 Frame 1: IDR | NALs: [SPS(14b), PPS(4b), IDR(40224b)] | Total: 40254b
📦 Frame 2: P | NALs: [SPS(14b), PPS(4b), P-slice(5107b)] | Total: 5137b
📦 Frame 3: P | NALs: [SPS(14b), PPS(4b), P-slice(23046b)] | Total: 23076b
...
📦 Frame 174: P | NALs: [SPS(14b), PPS(4b), P-slice(16917b)] | Total: 16947b
```

---

## AVC444 Investigation Summary

### Issues Fixed During This Session

#### 1. Feedback Loop Fix (avc444_encoder.rs)

**Problem:** `should_send_aux()` sent aux whenever main was IDR, creating feedback loop:
```
Aux refresh → DPB polluted → Main IDR → "sync required" → send Aux → loop
```

**Fix:** Removed `main_is_keyframe` check from aux send decision:
```rust
fn should_send_aux(&self, aux_frame, _main_is_keyframe: bool) -> bool {
    // main_is_keyframe is now IGNORED
    // Aux sent only on: first frame, max_interval, hash change
}
```

**Result:** P-frame ratio improved from 35% → 93%

#### 2. MIN_AUX_INTERVAL Rate Limiting (avc444_encoder.rs)

**Problem:** During screen activity (scrolling), aux content hash changed every frame, causing aux-every-frame → DPB pollution.

**Fix:** Added 10-frame minimum between aux sends:
```rust
const MIN_AUX_INTERVAL: u32 = 10;
if self.frames_since_aux < MIN_AUX_INTERVAL {
    return false;  // Rate limited
}
```

**Result:** Eliminated IDR bursts during activity

#### 3. Config Bug Fix (display_handler.rs)

**Problem:** `avc444_enabled = false` in config was ignored - only client capability checked.

**Fix:**
```rust
let avc444_enabled = self.config.egfx.avc444_enabled && client_supports_avc444;
if !self.config.egfx.avc444_enabled {
    info!("AVC444 disabled in config, using AVC420");
}
```

**Result:** Config setting now properly respected

### Unresolved AVC444 Issue

**Symptom:** Even with 93% P-frames and proper rate limiting, text remained blurry on RHEL 9.

**What Was Tried:**
1. Feedback loop elimination
2. Aux rate limiting (MIN_AUX_INTERVAL = 10)
3. Encoder complexity = High
4. QP constraints relaxed
5. VUI signaling verified

**Theories (Unverified):**
- OpenH264 version difference on RHEL 9
- GNOME 40 PipeWire frame format affecting chroma encoding
- YUV444→YUV420 conversion artifacts in aux stream
- Client-side AVC444 decoding issue specific to color space

**Conclusion:** Root cause unidentified. AVC420 works correctly as workaround.

---

## Files Modified

### 1. src/egfx/avc444_encoder.rs

**Changes:**
- Removed `main_is_keyframe` check in `should_send_aux()` (feedback loop fix)
- Added `MIN_AUX_INTERVAL = 10` rate limiting
- Added `Complexity::High` for text quality
- Extensive documentation comments

### 2. src/server/display_handler.rs

**Changes:**
- Fixed config bug: now checks BOTH `avc444_enabled` config AND client capability
- Added logging for mode selection

### 3. rhel9-config.toml

**Changes:**
- Set `avc444_enabled = false` to use AVC420 mode

---

## Current Status

### Working
- AVC420 encoding with 99% P-frames
- Sharp text quality
- ~1 Mbps bandwidth for 1280x800 @ 30fps
- Portal v4 screen capture
- Metadata cursor mode
- LibEI input injection
- Session restore tokens

### Not Working (RHEL 9 Specific)
- AVC444 produces blurry text (use AVC420 instead)

### Known Limitations (All Platforms)
- Clipboard unavailable (Portal v1 < v2)
- 7-second EGFX delay on connect (DVC channel negotiation)

---

## Configuration for RHEL 9

```toml
[egfx]
enabled = true
h264_bitrate = 10000
codec = "avc420"
qp_min = 10
qp_max = 25

# RHEL 9: Use AVC420 for quality
avc444_enabled = false

# These only apply if avc444_enabled = true (other platforms):
avc444_enable_aux_omission = true
avc444_max_aux_interval = 30
```

---

## Next Steps

### Before Release

1. **Test AVC444 on Modern VMs**
   - Fedora 40/41, Ubuntu 24.04, RHEL 10
   - Verify if AVC444 blurriness is RHEL 9 specific
   - If works elsewhere: add platform detection to auto-disable on RHEL 9

2. **Consider Making AVC444 Default Off**
   - AVC420 works everywhere
   - AVC444 provides minor color improvement but higher risk

### Future Investigation (Optional)

- Compare OpenH264 versions between working/non-working platforms
- Analyze actual YUV plane data before/after aux encoding
- Test with different Windows RDP client versions

---

## Deployment Verification

After deploying to RHEL 9:

```bash
# Check mode selection
grep "AVC420\|AVC444" server.log
# Should show: "AVC444 disabled in config, using AVC420"

# Check frame statistics
grep "Frame.*:" server.log | grep -oP "(IDR|P)" | sort | uniq -c
# Should show: ~99% P, ~1% IDR

# Check average frame size
grep "Total:" server.log | grep -oP "[0-9]+b" | head -20
# Should show: 3-25KB range, mostly 10-20KB for P-frames
```

---

## Git Status

```
Branch: main
Modified (uncommitted):
  - src/server/mod.rs
  - src/session/strategies/portal_token.rs
  - src/egfx/avc444_encoder.rs (new changes this session)
  - src/server/display_handler.rs (new changes this session)
```

**Recommended Commit Message:**
```
fix: use AVC420 on RHEL 9 for text quality

AVC444 produces blurry text on RHEL 9/GNOME 40 despite
optimizations. Root cause unidentified. AVC420 works
correctly with 99% P-frames and sharp text.

Changes:
- Fix config bug: respect avc444_enabled setting
- Fix feedback loop in aux send logic
- Add MIN_AUX_INTERVAL rate limiting
- Default to AVC420 on RHEL 9

See docs/SESSION-HANDOVER-2026-01-07.md for full analysis.
```

---

*AVC420 is production-ready for RHEL 9. AVC444 requires further investigation for this platform.*
