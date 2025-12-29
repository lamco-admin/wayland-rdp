# Phase 1: Single Encoder Structural Refactor - Implementation Details

**Date**: 2025-12-29
**Estimated Time**: 2 hours
**Risk**: Low (maintains all-I behavior)
**Goal**: Migrate from two encoders to one without behavior change

---

## WHAT WILL CHANGE

### File: `src/egfx/avc444_encoder.rs`

**Lines to modify**: ~150 lines across the file

---

## DETAILED CHANGES

### Change 1: Struct Definition (Lines 138-173)

**BEFORE**:
```rust
pub struct Avc444Encoder {
    main_encoder: Encoder,       // ← Remove
    aux_encoder: Encoder,        // ← Remove
    config: EncoderConfig,
    color_matrix: ColorMatrix,
    frame_count: u64,
    bytes_encoded: u64,
    total_encode_time_ms: f64,
    main_cached_sps_pps: Option<Vec<u8>>,  // ← Remove
    aux_cached_sps_pps: Option<Vec<u8>>,   // ← Remove
    current_level: Option<super::h264_level::H264Level>,
    force_all_keyframes: bool,
}
```

**AFTER**:
```rust
pub struct Avc444Encoder {
    encoder: Encoder,            // ← Single encoder for BOTH subframes
    config: EncoderConfig,
    color_matrix: ColorMatrix,
    frame_count: u64,
    bytes_encoded: u64,
    total_encode_time_ms: f64,
    cached_sps_pps: Option<Vec<u8>>,  // ← Single unified cache
    current_level: Option<super::h264_level::H264Level>,
    force_all_keyframes: bool,       // ← Keep for compatibility
}
```

---

### Change 2: Constructor (Lines 186-250)

**BEFORE**:
```rust
pub fn new(config: EncoderConfig) -> EncoderResult<Self> {
    // ... encoder_config setup ...

    // Create main encoder
    let main_encoder = Encoder::with_api_config(api, encoder_config.clone())?;

    // Create auxiliary encoder
    let aux_encoder = Encoder::with_api_config(api, encoder_config)?;

    Ok(Self {
        main_encoder,
        aux_encoder,
        // ...
        main_cached_sps_pps: None,
        aux_cached_sps_pps: None,
    })
}
```

**AFTER**:
```rust
pub fn new(config: EncoderConfig) -> EncoderResult<Self> {
    // ... encoder_config setup ...

    // Create SINGLE encoder for both main and auxiliary subframes
    // MS-RDPEGFX requirement: Same encoder instance maintains unified DPB
    let encoder = Encoder::with_api_config(api, encoder_config)?;

    debug!(
        "Created AVC444 encoder: {:?} matrix, {}kbps, level={:?}",
        color_matrix, config.bitrate_kbps, level
    );
    debug!("   Using SINGLE encoder for both subframes (MS-RDPEGFX compliant)");

    Ok(Self {
        encoder,
        // ...
        cached_sps_pps: None,  // Unified cache
    })
}
```

---

### Change 3: encode_bgra() Function (Lines ~280-410)

**Key sections to update**:

#### 3.1: Force Intra Calls

**BEFORE**:
```rust
if self.force_all_keyframes {
    self.main_encoder.force_intra_frame();
    self.aux_encoder.force_intra_frame();
}

// All-I workaround
self.main_encoder.force_intra_frame();
self.aux_encoder.force_intra_frame();
```

**AFTER**:
```rust
if self.force_all_keyframes {
    self.encoder.force_intra_frame();
}

// All-I workaround (force TWICE - once before each encode)
// Note: force_intra_frame() is "next frame will be I"
// We call it before each subframe encode
```

#### 3.2: Encoding Calls

**BEFORE**:
```rust
let main_bitstream = self.main_encoder.encode(&main_yuv_slices)?;
// ...
let aux_bitstream = self.aux_encoder.encode(&aux_yuv_slices)?;
```

**AFTER**:
```rust
// Encode main subframe
self.encoder.force_intra_frame();  // All-I workaround
let main_bitstream = self.encoder.encode(&main_yuv_slices)?;

// Encode auxiliary subframe (SAME encoder instance)
self.encoder.force_intra_frame();  // All-I workaround
let aux_bitstream = self.encoder.encode(&aux_yuv_slices)?;
```

#### 3.3: SPS/PPS Handling

**BEFORE**:
```rust
stream1_data = self.handle_sps_pps_main(stream1_data, main_is_keyframe);
stream2_data = self.handle_sps_pps_aux(stream2_data, aux_is_keyframe);
```

**AFTER**:
```rust
// Unified SPS/PPS handling for both subframes
stream1_data = self.handle_sps_pps(stream1_data, main_is_keyframe);
stream2_data = self.handle_sps_pps(stream2_data, aux_is_keyframe);
```

---

### Change 4: SPS/PPS Functions (Lines ~411-440)

**REMOVE**:
- `handle_sps_pps_main()` function
- `handle_sps_pps_aux()` function

**ADD**:
```rust
/// Handle SPS/PPS for subframe bitstreams
/// With single encoder, both subframes share same SPS/PPS
fn handle_sps_pps(&mut self, mut data: Vec<u8>, is_keyframe: bool) -> Vec<u8> {
    if is_keyframe {
        // Extract and cache SPS/PPS from this IDR
        if let Some(sps_pps) = Self::extract_sps_pps(&data) {
            self.cached_sps_pps = Some(sps_pps);
        }
    } else {
        // Prepend cached SPS/PPS to P-frame
        if let Some(ref sps_pps) = self.cached_sps_pps {
            let mut combined = sps_pps.clone();
            combined.extend_from_slice(&data);
            data = combined;
        }
    }
    data
}
```

---

### Change 5: force_keyframe() Method (Lines ~515-521)

**BEFORE**:
```rust
pub fn force_keyframe(&mut self) {
    self.main_encoder.force_intra_frame();
    self.aux_encoder.force_intra_frame();
    debug!("Forced keyframe in both AVC444 streams");
}
```

**AFTER**:
```rust
pub fn force_keyframe(&mut self) {
    self.encoder.force_intra_frame();
    debug!("Forced keyframe for both AVC444 subframes (single encoder)");
}
```

---

## TESTING STRATEGY

### Compile-Time Validation

```bash
cargo build --release
```

**Expected**: Clean build (warnings OK, no errors)

---

### Runtime Validation

**Deploy and test**:
1. Connect via RDP
2. Scroll text, move windows, interact
3. Verify perfect quality (same as current)

**Check logs for**:
```
Created AVC444 encoder: ... SINGLE encoder for both subframes
[AVC444 Frame #0] Main: IDR, Aux: IDR
[AVC444 Frame #1] Main: IDR, Aux: IDR  (all-I active)
```

**Success Criteria**:
- ✅ No crashes
- ✅ Perfect visual quality
- ✅ Similar bandwidth (~4.3 MB/s)
- ✅ Responsive performance

**If any issues**: Debug before proceeding to Phase 2

---

### Bandwidth Comparison

**Current** (two encoders, all-I):
- Main IDR: ~74KB
- Aux IDR: ~70KB
- Total: ~144KB per frame @ 30fps = 4.3 MB/s

**After Phase 1** (one encoder, all-I):
- Should be IDENTICAL or very close
- Any significant difference indicates issue

---

## ROLLBACK PLAN

**If Phase 1 causes issues**:

```bash
# Revert the commit
git revert HEAD

# Or restore specific file
git checkout HEAD~1 -- src/egfx/avc444_encoder.rs

# Rebuild and redeploy
cargo build --release
# ... deploy ...
```

**Fallback**: Previous working version still available

---

## WHAT DOESN'T CHANGE (Stays Same)

✅ Packing algorithm (`yuv444_packing.rs`)
✅ Color conversion (`color_convert.rs`)
✅ All-I behavior (force_intra_frame still active)
✅ SPS/PPS extraction logic
✅ Frame packaging and PDU creation
✅ External API (display_handler still calls same methods)

**Only internal structure changes**

---

## PHASE 1 COMPLETION CHECKLIST

Before proceeding to Phase 2:

- [ ] Code compiles cleanly
- [ ] Visual quality perfect (user verification)
- [ ] Bandwidth ~4.3 MB/s
- [ ] No crashes or errors
- [ ] Logs show single encoder initialization
- [ ] Can scroll text smoothly
- [ ] Can move windows
- [ ] Right-click menus work

**When all checked**: Phase 1 SUCCESS → Ready for Phase 2

---

## ESTIMATED BREAKDOWN

- **Struct changes**: 15 min
- **Constructor refactor**: 20 min
- **encode_bgra() refactor**: 30 min
- **SPS/PPS merge**: 20 min
- **Helper function updates**: 15 min
- **Compile and fix errors**: 15 min
- **Deploy and test**: 15 min

**Total**: ~2 hours

---

## READY TO PROCEED

**Current**: Clean stable state, committed to git, pushed to GitHub
**Next**: Begin Phase 1 implementation
**After Phase 1**: Phase 2 (P-frames with multi-ref + instrumentation)

**Shall I begin Phase 1 implementation now?**
