# AVC444 P-Frame Corruption - Root Cause Analysis

**Date**: 2025-12-27 Evening
**Based on**: Previous expert analysis + current findings

---

## The Problem

**Symptom**:
- All-I frames: Perfect colors, no corruption ✅
- P-frames: Lavender/brown/purple blocky corruption in changed areas ❌

**Pattern**: "Phantom changes" where static content causes P-frame differences.

---

## Root Cause (From Previous Analysis)

**Most likely**: **Padding/stride bytes are nondeterministic**

### Why This Happens

H.264 encodes in 16×16 macroblocks. If auxiliary frame dimensions aren't exact multiples of 16, encoder sees:
- Right edge padding (width → 16-aligned)
- Bottom padding (height → 16-aligned)
- Stride gap bytes at end of each row

If these bytes vary frame-to-frame (stack garbage, reused buffers), then:
- **I-frames**: Encode garbage "as is"
- **P-frames**: Chase those "changes" → blocky corruption

**Why lavender/purple?** Chroma planes (U/V) getting polluted. Even if math is correct, random padding in U/V produces purple/green macroblocks once motion compensation uses it.

---

## What We've Done So Far

### Attempt #1: Padding Initialization ✅
**Location**: `src/egfx/yuv444_packing.rs:459-466`

```rust
// Initialize with deterministic padding
let mut aux_y = vec![128u8; padded_height * width];
let mut aux_u = vec![128u8; padded_chroma_width * padded_chroma_height];
let mut aux_v = vec![128u8; padded_chroma_width * padded_chroma_height];
```

**Status**: DONE, but maybe not sufficient

### Attempt #2: Force All-I Frames 🔄
**Location**: `src/egfx/avc444_encoder.rs:325-326`

```rust
self.main_encoder.force_intra_frame();  // Main all-I
self.aux_encoder.force_intra_frame();   // Aux all-I (just added)
```

**Status**: DEPLOYED (MD5: `b1e79780138f6bd8069dbe506f0b48a9`)
**Effect**: Eliminates P-frames → corruption should disappear
**Downside**: Not sustainable (high bitrate, latency)

---

## Critical Questions (From Analysis)

### Q1: Are main+aux interleaved into one stream or separate?

**Answer**: **Separate streams**

We have:
- Two separate OpenH264 encoders (`main_encoder`, `aux_encoder`)
- Two separate bitstreams (`stream1_data`, `stream2_data`)
- Sent as separate H.264 elementary streams via EGFX

**Implication**: DPB contamination (#2 in analysis) should NOT happen, since decoders are separate.

### Q2: Is padding actually deterministic?

**Test suggested**: Hash the entire coded buffer including padding:
```
hash(stride * coded_height bytes for Y)
hash(stride * coded_height/2 bytes for U/V)
```

**We already do this!** `src/egfx/yuv444_packing.rs:495-511`:
```rust
let mut hasher = std::collections::hash_map::DefaultHasher::new();
aux_y.hash(&mut hasher);
aux_u.hash(&mut hasher);
aux_v.hash(&mut hasher);
let frame_hash = hasher.finish();

if prev == frame_hash {
    debug!("✅ TEMPORAL STABLE: Auxiliary IDENTICAL");
} else if prev != 0 {
    debug!("⚠️  TEMPORAL CHANGE: Auxiliary DIFFERENT");
}
```

**From logs**: We saw "⚠️ TEMPORAL CHANGE" even on static wallpaper!

**This confirms**: Padding IS changing frame-to-frame, even with our initialization!

---

## Why Padding Might Still Be Nondeterministic

Even though we initialize with `vec![128u8; size]`, several things could cause changes:

### Possibility #1: Stride vs Width Mismatch

We allocate:
```rust
let mut aux_y = vec![128u8; padded_height * width];  // ← width, not stride!
```

OpenH264 might be using a **stride** that's different from `width`. If OpenH264's internal stride is wider (for alignment), it could be reading/writing beyond our buffer, or we're not padding the stride gaps.

### Possibility #2: YUVSlices Stride Mismatch

When we pass to OpenH264:
```rust
let aux_strides = aux_yuv420.strides();  // Returns (width, width/2, width/2)
let aux_yuv_slices = YUVSlices::new(
    (aux_yuv420.y_plane(), aux_yuv420.u_plane(), aux_yuv420.v_plane()),
    dims,
    aux_strides,
);
```

If OpenH264 expects stride-aligned buffers but we're passing width-aligned, it might read garbage at row ends.

### Possibility #3: Encoder Internal Reuse

OpenH264 might be reusing internal buffers that have garbage from previous frames, even if our input is deterministic.

---

## Diagnostic Test Plan

### Test 1: Verify Temporal Stability (CRITICAL)

**Run server and check logs for**:
```
✅ TEMPORAL STABLE: Auxiliary IDENTICAL (hash: 0x...)
```

vs

```
⚠️  TEMPORAL CHANGE: Auxiliary DIFFERENT (prev: 0x..., curr: 0x...)
```

**On static wallpaper**, we should see STABLE, not CHANGE.

**If we see CHANGE**: Our padding isn't actually deterministic. Need to:
1. Log first-different-byte offset
2. Check if it's in padding region or actual data
3. Investigate stride vs width issue

### Test 2: Explicit Padding Clear Before Each Frame

**Add to yuv444_packing.rs before filling aux planes**:
```rust
// Explicitly zero padding region AFTER allocating
for y in height..padded_height {
    let row_start = y * width;
    for x in 0..width {
        aux_y[row_start + x] = 128;
    }
}
```

### Test 3: OpenH264 Non-Reference Marking

Even though we use separate streams, ensure aux doesn't get marked as reference:
- Check OpenH264 config for reference frame settings
- Verify `nal_ref_idc = 0` in aux bitstream

---

## Expected Results with Current Fix

**Binary**: `b1e79780138f6bd8069dbe506f0b48a9` (both encoders all-I)

**Expected**:
1. ✅ No corruption (no P-frames to corrupt)
2. ✅ Should see "TEMPORAL STABLE" (if padding is deterministic)
3. ❌ High latency/bitrate (all-I is not sustainable)

**If corruption STILL appears** with all-I:
- Something fundamentally wrong with packing
- Not a P-frame prediction issue

**If "TEMPORAL CHANGE" still appears**:
- Padding is still nondeterministic despite initialization
- Need Test 2 (explicit padding clear)

---

## Proper Fix (After Diagnostics)

Once we confirm padding is deterministic AND all-I works:

1. **Re-enable P-frames** for both encoders
2. **Ensure stride-aligned buffers** (match OpenH264's expectations)
3. **Explicit padding clear** before each frame
4. **Mark aux as non-reference** (nal_ref_idc = 0)
5. **Test thoroughly** with screen changes

---

## Next Immediate Step

**Test current binary** (`b1e79780138f6bd8069dbe506f0b48a9`):

1. Run server with colorful wallpaper
2. Open menus, move windows (create changes)
3. **Check logs** for TEMPORAL STABLE vs CHANGE
4. **Check visual** for corruption

**Report both**:
- Does corruption appear? (Should not with all-I)
- Do logs show STABLE or CHANGE? (Should be STABLE on static content)

---

**Status**: All-I workaround deployed. Diagnostic logging active. Ready to test and analyze temporal stability.
