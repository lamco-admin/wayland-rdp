# Current Status - Quick Reference
**Updated**: 2025-12-27 22:40

---

## ✅ WHAT WORKS

**AVC420 (Standard 4:2:0)**: ✅ PERFECT
- No corruption
- Colors correct
- Good performance
- **This is our baseline - everything works**

**AVC444 Both All-I Frames**: ✅ PERFECT (but slow)
- No corruption
- Colors correct
- Extreme latency (all keyframes = huge bandwidth)
- **This proves our packing is 100% correct**

---

## ❌ WHAT'S BROKEN

**AVC444 with ANY P-Frames**: ❌ ISSUES

| Main | Aux | Result |
|------|-----|--------|
| P | P | ❌ Heavy lavender corruption |
| I | P | 🟡 Readable but **colors wrong** |
| P | I | ❌ Still corrupted |
| I | I | ✅ Perfect (but too slow) |

---

## 🎯 THE TWO PROBLEMS

### Problem 1: Main P-Frames + Auxiliary = Corruption
- **Workaround**: Force main to all-I
- **Result**: Corruption gone, readable text
- **Cost**: Higher bandwidth (but acceptable)

### Problem 2: Colors Wrong in AVC444 (Even Without Corruption)
- **When**: Even with main all-I + aux P (no corruption)
- **Not affected by**: Color matrix (BT.709 vs OpenH264)
- **AVC420**: Colors perfect
- **AVC444**: Colors slightly off

---

## 🔧 CURRENTLY DEPLOYED

**MD5**: `b37d8a07ea27da8274fd4a7597297bff`

**Config**:
- AVC444 enabled
- Main: Forced all-I
- Auxiliary: Normal P-frames
- Colors: Wrong but usable
- Performance: Good
- Corruption: None (readable)

---

## 📚 CODE COMPARISON: Our Implementation vs FreeRDP

### Auxiliary Y Plane (B4/B5)
**FreeRDP**:
```c
if (y % 16 < 8)
    memcpy(pY, pSrcU + (2*uY++ + 1)*srcStep, width);  // U444 odd rows
else
    memcpy(pY, pSrcV + (2*vY++ + 1)*srcStep, width);  // V444 odd rows
```

**Ours**:
```rust
if macroblock_row < 8 {
    aux_y[aux_start..].copy_from_slice(&yuv444.u[src_start..]);  // U444 odd rows
} else {
    aux_y[aux_start..].copy_from_slice(&yuv444.v[src_start..]);  // V444 odd rows
}
```
✅ **IDENTICAL**

### Auxiliary U/V Planes (B6/B7)
**FreeRDP**:
```c
for (size_t y = 0; y < halfHeight; y++) {
    for (size_t x = 0; x < halfWidth; x++) {
        pU[x] = pSrcU[2*x + 1];  // U444 at (odd_col, even_row)
        pV[x] = pSrcV[2*x + 1];  // V444 at (odd_col, even_row)
    }
}
```

**Ours**:
```rust
for cy in 0..chroma_height {
    let y = cy * 2;  // Even row
    for cx in 0..chroma_width {
        let x = cx * 2 + 1;  // Odd column
        aux_u[out_idx] = yuv444.u[idx];  // U444 at (odd_col, even_row)
        aux_v[out_idx] = yuv444.v[idx];  // V444 at (odd_col, even_row)
    }
}
```
✅ **IDENTICAL**

**Conclusion**: Our packing matches FreeRDP exactly!

---

## 🤔 WHY ARE COLORS WRONG?

Since our packing matches FreeRDP and all-I works perfectly:

**Hypothesis**: When main uses all-I but auxiliary uses P-frames:
- Auxiliary P-frames encode "changes" in chroma
- But main has NO temporal reference (all fresh I-frames)
- Client might expect main and auxiliary to have matching reference structure
- This temporal mismatch could cause color reconstruction errors

**Test needed**: Force auxiliary to match main's frame types
- When main sends I-frame, force auxiliary to send I-frame too
- When main sends P-frame, let auxiliary send P-frame
- This synchronizes the reference structures

---

## 🚀 NEXT TEST TO TRY

Force auxiliary to use **SAME frame type as main**:

```rust
// In avc444_encoder.rs, encode_bgra():

// Force main to I-frame
self.main_encoder.force_intra_frame();

// Force auxiliary to ALSO be I-frame (synchronized)
self.aux_encoder.force_intra_frame();

// Result: Both streams have matching frame types
// This is what "both all-I" was, and it worked perfectly
// But we need to reduce bandwidth somehow...
```

**Alternative**: Use lower keyframe interval
- Both encoders send I-frame every 30 frames
- Use P-frames between keyframes
- This might work if temporal synchronization is the issue

---

## 📖 REFERENCES

**Microsoft Spec**: [MS-RDPEGFX YUV420p Stream Combination](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/8131c1bc-1af8-4907-a05a-f72f4581160f)

**FreeRDP Source**:
- [prim_YUV.c](https://github.com/FreeRDP/FreeRDP/blob/master/libfreerdp/primitives/prim_YUV.c)
- [AVC444 Implementation Commit](https://github.com/FreeRDP/FreeRDP/commit/5bc333c626f1db493a2c2e3c49d91cc6fb145309)

**Known Issues**:
- [Issue #11040](https://github.com/FreeRDP/FreeRDP/issues/11040) - Reverse filter bugs (Jan 2025)
- [PR #11358](https://github.com/FreeRDP/FreeRDP/pull/11358) - YUV420 fix (Mar 2025)

---

## 🎯 IMMEDIATE NEXT STEP

Since our packing matches FreeRDP exactly, the color issue is likely:

1. **Frame type synchronization**: Main and auxiliary must use same frame types
2. **Encoder settings mismatch**: Some OpenH264 setting differs between main/aux
3. **Client expects synchronized keyframes**: Both streams need matching I-frames

**Test**: Reduce keyframe interval (e.g., I-frame every 30 frames) instead of all-I.
