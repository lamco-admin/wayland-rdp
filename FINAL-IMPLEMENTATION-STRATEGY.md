# AVC444 Single Encoder - Final Implementation Strategy

**Date**: 2025-12-29 02:05 UTC
**Status**: Ready for implementation
**Approach**: Natural multi-reference, NOT LTR micromanagement

---

## THE CORRECT UNDERSTANDING (Thanks to Expert Session)

### Why LTR Approach Was Wrong

**LTR is designed for**:
- Packet loss recovery
- Feedback-driven resilience
- Error concealment

**LTR is NOT designed for**:
- Deterministic "always reference slot X" control
- Per-frame reference list micromanagement
- Architectural reference chain separation

**Using LTR as I proposed would be fighting the API design**

---

### The Natural Solution: Multi-Reference DPB

**Encoding sequence**: Main(t), Aux(t), Main(t+1), Aux(t+1), ...

**With DPB size ≥ 2**:
```
After Main(t):   DPB = [Main(t)]
After Aux(t):    DPB = [Main(t), Aux(t)]
After Main(t+1): DPB = [Aux(t), Main(t+1)] or [Main(t), Aux(t), Main(t+1)]

Main(t+1) P-frame needs to predict from "2 frames back" = Main(t)
```

**OpenH264's motion search**:
- Searches all available references
- Finds best match (lowest SAD/SATD)
- **Main(t) is much better match for Main(t+1) than Aux(t) is**
  - Same content type (luma patterns)
  - Similar pixel values
  - Motion vectors make sense
- **Naturally selects Main(t)** ✓

**Same logic for Aux**:
- Aux(t+1) naturally prefers Aux(t) as reference
- Chroma patterns match
- Better prediction than Main would give

**We don't need to control it - just provide the right references!**

---

## THREE-PHASE IMPLEMENTATION

### PHASE 1: Structural Refactor (All-I) - 2 hours

**Objective**: Change from two encoders to one, maintain stability

**Code Changes**:

```rust
// File: src/egfx/avc444_encoder.rs

// OLD:
pub struct Avc444Encoder {
    main_encoder: Encoder,
    aux_encoder: Encoder,
    main_cached_sps_pps: Option<Vec<u8>>,
    aux_cached_sps_pps: Option<Vec<u8>>,
}

// NEW:
pub struct Avc444Encoder {
    encoder: Encoder,  // Single encoder
    cached_sps_pps: Option<Vec<u8>>,  // Single cache
    num_ref_frames: i32,  // Track DPB size
}

impl Avc444Encoder {
    pub fn new(config: EncoderConfig) -> EncoderResult<Self> {
        let encoder = Encoder::with_api_config(api, encoder_config)?;

        Ok(Self {
            encoder,
            cached_sps_pps: None,
            num_ref_frames: 1,  // Default, will increase in Phase 2
            // ...
        })
    }

    pub fn encode_bgra(...) -> EncoderResult<Option<Avc444Frame>> {
        let yuv444 = bgra_to_yuv444(...);
        let (main_yuv420, aux_yuv420) = pack_dual_views(&yuv444);

        // All-I workaround (same as current)
        self.encoder.force_intra_frame();
        let main_bitstream = self.encoder.encode(&main_yuv420)?;

        self.encoder.force_intra_frame();
        let aux_bitstream = self.encoder.encode(&aux_yuv420)?;

        // Unified SPS/PPS handling
        let stream1_data = self.handle_sps_pps(
            main_bitstream.to_vec(),
            matches!(main_bitstream.frame_type(), FrameType::IDR | FrameType::I)
        );

        let stream2_data = self.handle_sps_pps(
            aux_bitstream.to_vec(),
            matches!(aux_bitstream.frame_type(), FrameType::IDR | FrameType::I)
        );

        // Create AVC444Frame (same as before)
        Ok(Some(Avc444Frame { ... }))
    }

    // Unified SPS/PPS handling (merge main/aux versions)
    fn handle_sps_pps(&mut self, mut data: Vec<u8>, is_keyframe: bool) -> Vec<u8> {
        if is_keyframe {
            if let Some(sps_pps) = Self::extract_sps_pps(&data) {
                self.cached_sps_pps = Some(sps_pps);
            }
        } else if let Some(ref sps_pps) = self.cached_sps_pps {
            let mut combined = sps_pps.clone();
            combined.extend_from_slice(&data);
            data = combined;
        }
        data
    }
}
```

**Testing**:
- Compile and run
- Verify same quality as current
- Check bandwidth (~4.3 MB/s)
- No regressions

**Success = Foundation ready for Phase 2**

---

### PHASE 2: Multi-Ref P-Frames + Instrumentation - 3 hours

**Objective**: Enable P-frames with visibility into reference behavior

#### Step 2.1: Configure Multi-Reference

```rust
impl Avc444Encoder {
    pub fn new(config: EncoderConfig) -> EncoderResult<Self> {
        let mut encoder = Encoder::with_api_config(api, encoder_config)?;

        // Configure to keep ≥2 reference frames
        unsafe {
            Self::configure_num_refs(&mut encoder, 2)?;
        }

        Ok(Self {
            encoder,
            num_ref_frames: 2,
            // ...
        })
    }

    unsafe fn configure_num_refs(encoder: &mut Encoder, num: i32) -> EncoderResult<()> {
        let raw_api = encoder.raw_api();

        // ENCODER_OPTION_NUMBER_REF = 13
        const ENCODER_OPTION_NUMBER_REF: i32 = 13;

        let result = raw_api.set_option(
            ENCODER_OPTION_NUMBER_REF,
            &num as *const i32 as *mut std::ffi::c_void
        );

        if result != 0 {
            return Err(EncoderError::InitFailed(
                format!("Failed to set num_ref_frames: {}", result)
            ));
        }

        debug!("✅ Encoder DPB configured to keep {} reference frames", num);
        Ok(())
    }
}
```

#### Step 2.2: Add NAL Instrumentation

```rust
/// Minimal NAL unit for analysis
struct NalUnit {
    nal_type: u8,
    nal_ref_idc: u8,
}

impl Avc444Encoder {
    /// Parse Annex B bitstream into NAL units
    fn parse_annex_b(data: &[u8]) -> Vec<NalUnit> {
        let mut nals = Vec::new();
        let mut i = 0;

        while i < data.len() {
            // Find start code
            let start_code_len = if i + 4 <= data.len() && data[i..i+4] == [0,0,0,1] {
                4
            } else if i + 3 <= data.len() && data[i..i+3] == [0,0,1] {
                3
            } else {
                i += 1;
                continue;
            };

            i += start_code_len;
            if i >= data.len() { break; }

            let header = data[i];
            let nal_type = header & 0x1F;
            let nal_ref_idc = (header >> 5) & 0x03;

            nals.push(NalUnit { nal_type, nal_ref_idc });

            // Find next NAL (simplified - just skip to next start code)
            while i < data.len() {
                if i + 3 <= data.len() && (data[i..i+3] == [0,0,1] || data[i..i+3] == [0,0,0]) {
                    break;
                }
                i += 1;
            }
        }

        nals
    }

    /// Log NAL structure for diagnostics
    fn log_nal_structure(bitstream: &[u8], label: &str, frame_num: u64) {
        let nals = Self::parse_annex_b(bitstream);

        for (i, nal) in nals.iter().enumerate() {
            let nal_type_str = match nal.nal_type {
                1 => "P-slice",
                5 => "IDR",
                7 => "SPS",
                8 => "PPS",
                _ => "Other",
            };

            let ref_str = if nal.nal_ref_idc > 0 { "REFERENCE" } else { "NON-REF" };

            debug!("  [Frame #{} {} NAL#{}] type={} ({}), ref_idc={} ({})",
                   frame_num, label, i, nal.nal_type, nal_type_str,
                   nal.nal_ref_idc, ref_str);
        }
    }
}
```

#### Step 2.3: Enable P-Frames with Logging

```rust
pub fn encode_bgra(...) -> EncoderResult<Option<Avc444Frame>> {
    let yuv444 = bgra_to_yuv444(...);
    let (main_yuv420, aux_yuv420) = pack_dual_views(&yuv444);

    // NO force_intra - let encoder decide (will use P-frames)
    let main_bitstream = self.encoder.encode(&main_yuv420)?;
    let main_data = main_bitstream.to_vec();

    // Log what we encoded
    Self::log_nal_structure(&main_data, "MAIN", self.frame_count);

    let aux_bitstream = self.encoder.encode(&aux_yuv420)?;
    let aux_data = aux_bitstream.to_vec();

    // Log what we encoded
    Self::log_nal_structure(&aux_data, "AUX", self.frame_count);

    // Handle SPS/PPS (unified)
    // Create AVC444Frame
    // ...
}
```

**The Logs Will Show**:
```
[Frame #0 MAIN NAL#0] type=7 (SPS), ref_idc=3 (REFERENCE)
[Frame #0 MAIN NAL#1] type=8 (PPS), ref_idc=3 (REFERENCE)
[Frame #0 MAIN NAL#2] type=5 (IDR), ref_idc=3 (REFERENCE)

[Frame #0 AUX NAL#0] type=5 (IDR), ref_idc=3 (REFERENCE)

[Frame #1 MAIN NAL#0] type=1 (P-slice), ref_idc=2 (REFERENCE)
[Frame #1 AUX NAL#0] type=1 (P-slice), ref_idc=? (REFERENCE or NON-REF?)
```

**Critical observation**: Is Aux marked as REF or NON-REF?
- If NON-REF → Perfect! Main can't ref Aux
- If REF → Might be issue, need Phase 3

**Testing**:
- Run with P-frames enabled
- Check for corruption
- Analyze logs

**Outcomes**:
- **No corruption** → Natural refs work! Done! ✅
- **Corruption + logs show Main refs Aux** → Need Phase 3 (make Aux non-ref)
- **Corruption + logs show correct refs** → Something else (unlikely)

---

### PHASE 3: Aux Non-Reference (If Needed) - 2-4 hours

**Only proceed if Phase 2 logs show problematic references**

**Research needed**:
- How to mark frames as non-reference in OpenH264
- Is there an encoder option?
- Or per-frame setting?

**Not a post-processing hack** - must be done inside encoder

---

## WHY THIS IS THE ROBUST APPROACH

### Natural Over Forced

- ✅ Uses motion search as designed
- ✅ Leverages H.264's built-in intelligence
- ✅ Simpler and more maintainable
- ✅ Easier to debug and validate

### Instrumentation Over Assumptions

- ✅ See exactly what's happening
- ✅ Validate behavior empirically
- ✅ Quick diagnosis if issues arise
- ✅ Can't be fooled by wrong assumptions

### Phased Over Big Bang

- ✅ Validate each step
- ✅ Catch issues early
- ✅ Reduce integration risk
- ✅ Multiple fallback points

**This embodies "best, most robust, innovative" - but done RIGHT**

---

## READY TO PROCEED

**All documents ready**:
- ✅ Research complete
- ✅ Plan documented
- ✅ Risks assessed
- ✅ Fallbacks identified

**System stable**: All-I workaround active

**Next step**: Begin Phase 1 implementation

Shall I proceed with Phase 1 (single encoder structural refactor)?
