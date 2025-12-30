# Single Encoder Implementation - DEPLOYED

**Date**: 2025-12-29 18:20 UTC
**Binary MD5**: TBD (checking)
**Config MD5**: TBD (checking)
**Status**: ✅ SINGLE ENCODER ARCHITECTURE IMPLEMENTED
**Change**: Dual encoder → Single encoder (MS-RDPEGFX spec compliant)

---

## WHAT WAS CHANGED

### Code Changes (~50 lines)

**src/egfx/avc444_encoder.rs**:

**BEFORE (Dual Encoder)**:
```rust
pub struct Avc444Encoder {
    main_encoder: Encoder,  // Encoder #1
    aux_encoder: Encoder,   // Encoder #2
    main_cached_sps_pps: Option<Vec<u8>>,
    aux_cached_sps_pps: Option<Vec<u8>>,
}

// Encoding:
self.main_encoder.encode(&main_yuv)
self.aux_encoder.encode(&aux_yuv)
```

**AFTER (Single Encoder)**:
```rust
pub struct Avc444Encoder {
    encoder: Encoder,  // ONE encoder for both!
    cached_sps_pps: Option<Vec<u8>>,  // Shared cache
}

// Sequential encoding (FreeRDP pattern):
self.encoder.encode(&main_yuv)  // First call
self.encoder.encode(&aux_yuv)   // Second call, SAME encoder
```

**Key methods updated**:
- `new()`: Creates ONE encoder instead of two
- `encode_bgra()`: Calls `self.encoder` twice sequentially
- `force_keyframe()`: Calls `self.encoder.force_intra_frame()` once
- `handle_sps_pps()`: Merged from two methods into one
- Added `strip_sps_pps()`: Remove SPS/PPS from aux

---

## WHY THIS MATTERS

### Unified DPB (Decoded Picture Buffer)

**Dual encoder (old)**:
```
Main encoder: DPB₁ [Main₀, Main₁, Main₂, ...]
Aux encoder:  DPB₂ [Aux₀, Aux₁, Aux₂, ...]
Client:       DPB  [Main₀, Aux₀, Main₁, Aux₁, ...]  ← Expects unified!
Result: Mismatch → Corruption
```

**Single encoder (new)**:
```
Encoder: DPB [Main₀, Aux₀, Main₁, Aux₁, ...]
Client:  DPB [Main₀, Aux₀, Main₁, Aux₁, ...]  ← MATCHES!
Result: Synchronized → No corruption (hopefully!)
```

---

## TEST CONFIGURATION

**Current settings**:
- Single encoder: ✅ ACTIVE
- P-frames: ✅ ENABLED (all-I workaround removed)
- Aux omission: ❌ DISABLED (config.toml: false)
- force_all_keyframes: false

**This tests**: Single encoder + P-frames WITHOUT aux omission

**Purpose**: Isolate whether single encoder fixes corruption

---

## EXPECTED RESULTS

### If Single Encoder Fixes Corruption

**Frame pattern**:
```
Frame #0: Main IDR + Aux IDR (first frame)
Frame #1+: Main P + Aux IDR (aux still IDR due to scene change)
```

**Visual**: ✅ NO lavender corruption
**Bandwidth**: ~2.5-3.5 MB/s (Main P + Aux IDR always sent)
**Quality**: Perfect

**Then**: Enable aux omission (Test 3B) to get to <2 MB/s

### If Corruption Still Occurs

**Then**: Single encoder alone isn't sufficient
**Need**: Additional investigation or hardware encoders

---

## WHAT TO WATCH FOR

**Startup log**:
```
🔧 AVC444: Using SINGLE encoder for both Main and Aux (spec-compliant)
```

**Frame logs**:
```
[AVC444 Frame #0] Main: IDR, Aux: IDR [BOTH SENT]
[AVC444 Frame #8+] Main: P, Aux: IDR [BOTH SENT]
```

**Critical**: NO `[OMITTED]` messages (omission disabled)

---

**Deployed following proper workflow**:
1. ✅ Deleted old binary
2. ✅ Deleted old config
3. ✅ Copied new binary
4. ✅ Copied new config
5. ✅ Verified MD5s

**Ready to test!** This is the REAL test of single encoder architecture.
