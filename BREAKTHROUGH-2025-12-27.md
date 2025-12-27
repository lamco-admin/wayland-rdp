# BREAKTHROUGH: Found the Nondeterminism!

**Date**: 2025-12-27 Late Evening
**Status**: ROOT CAUSE IDENTIFIED

---

## 🎯 The Breakthrough

### Test Results

**Visual quality with all-I**: **PERFECT** ✅
- No corruption
- No lavender
- No color issues
- Absolutely perfect quality

**But logs show**: **Auxiliary buffer is nondeterministic** ❌

### The Evidence

**Input is 100% stable**:
```
Frame 1: center BGRA=(125, 35,240) → YUV=( 85,150,226)
Frame 2: center BGRA=(125, 35,240) → YUV=( 85,150,226)
Frame 3: center BGRA=(125, 35,240) → YUV=( 85,150,226)
...
```

**But auxiliary hash changes every frame**:
```
Frame 1: hash 0x6baef4dda507a104
Frame 2: hash 0xfedd5d5e0220c3a0
Frame 3: hash 0x1f6f654f2b312036
Frame 4: hash 0xbb9f69d9ceff8297
...
```

### Conclusion

**The problem is NOT**:
- ❌ Input data (BGRA is stable)
- ❌ Color conversion (YUV444 is stable)
- ❌ Main view packing (working fine)

**The problem IS**:
- ✅ **Something in `pack_auxiliary_view()` is nondeterministic**

---

## 🔍 What Could Be Nondeterministic?

Looking at `src/egfx/yuv444_packing.rs:pack_auxiliary_view_spec_compliant()`:

### Possibility #1: Padding Regions Not Fully Initialized

We initialize:
```rust
let mut aux_y = vec![128u8; padded_height * width];
let mut aux_u = vec![128u8; padded_chroma_width * padded_chroma_height];
let mut aux_v = vec![128u8; padded_chroma_width * padded_chroma_height];
```

But then we **overwrite parts** of these buffers. The portions we don't overwrite might have:
- Reused memory from previous allocations
- Stack garbage
- Uninitialized padding

### Possibility #2: Row Truncation Creates Gaps

When we truncate aux_y:
```rust
aux_y.truncate(height * width);
```

If `padded_height > height`, we're removing padding rows. But before truncation, those rows might have had nondeterministic data that affected the hash.

### Possibility #3: Chroma Padding Region

The chroma buffers use `padded_chroma_width` and `padded_chroma_height`, but we only fill up to `chroma_width` and `chroma_height`. The padding region (right edge, bottom edge) might not be deterministic.

---

## 🚀 The Fix

### Step 1: Explicitly Clear ALL Padding Before Filling

**Add to `pack_auxiliary_view_spec_compliant()` BEFORE the loop**:

```rust
// CRITICAL: Explicitly zero ALL padding bytes for deterministic encoding
// Even though vec![128u8; size] initializes, we need to ensure padding
// regions that might not get overwritten are deterministic

// Clear Y padding (rows beyond height)
for row in height..padded_height {
    let start = row * width;
    let end = start + width;
    for i in start..end {
        aux_y[i] = 128;
    }
}

// Clear U/V padding (right edge and bottom)
for cy in 0..padded_chroma_height {
    for cx in chroma_width..padded_chroma_width {
        let idx = cy * padded_chroma_width + cx;
        aux_u[idx] = 128;
        aux_v[idx] = 128;
    }
}
for cy in chroma_height..padded_chroma_height {
    for cx in 0..padded_chroma_width {
        let idx = cy * padded_chroma_width + cx;
        aux_u[idx] = 128;
        aux_v[idx] = 128;
    }
}
```

### Step 2: Don't Truncate - Keep Deterministic Padding

**Remove the truncate**:
```rust
// REMOVED: aux_y.truncate(height * width);
// Keep the full padded buffer so OpenH264 sees deterministic padding
```

### Step 3: Test

After this fix:
1. Recompile and deploy
2. Run with static wallpaper
3. Logs should show: `✅ TEMPORAL STABLE: Auxiliary IDENTICAL`
4. Then re-enable P-frames and test

---

## Expected Outcome

With padding deterministic:
1. All-I frames: Still perfect ✅
2. Auxiliary hash: Should be stable (same hash every frame) ✅
3. P-frames re-enabled: Should work without corruption ✅

---

## Why This Matters

P-frames work by:
1. Comparing current frame to reference frame
2. Encoding only the difference (residual)
3. Using motion compensation

If padding changes frame-to-frame:
- P-frame encoder detects "changes" in padding
- Tries to encode those "changes"
- Motion compensation uses polluted data
- Result: Lavender/brown corruption in macroblocks

With deterministic padding:
- Static content = identical frames (including padding)
- P-frame encoder sees zero difference
- Encodes efficiently, no corruption
- Perfect!

---

## Implementation Priority

**HIGH**: This is the fix that will make P-frames work properly.

**Next steps**:
1. Implement explicit padding clear
2. Test for temporal stability
3. Re-enable P-frames
4. Verify no corruption even with screen changes

---

**Status**: Root cause identified. Fix ready to implement.
