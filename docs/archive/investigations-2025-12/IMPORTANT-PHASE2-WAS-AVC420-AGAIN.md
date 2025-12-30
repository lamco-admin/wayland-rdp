# IMPORTANT: Phase 2 Test Was AVC420, Not AVC444

**Date**: 2025-12-29 03:03 UTC
**Finding**: NUM_REF configuration failed, fell back to AVC420

---

## What Happened

### From the Log

```
⚠️  WARN: Failed to create AVC444 encoder: InitFailed("Failed to set num_ref_frames to 2: error code 4") - falling back to AVC420
✅ AVC420 encoder initialized for 1280×800 (4:2:0 fallback)
```

**Error Code 4**: Same as deblocking experiment - set_option() doesn't work after encoder creation

**Result**: System fell back to **AVC420** again

---

## Why AVC420 Has No Corruption

**AVC420** is standard YUV420 H.264:
- ONE stream (not dual)
- Normal luma and chroma (no chroma-as-luma)
- P-frames work perfectly
- **This is why you saw no lavender!**

---

## What This Confirms

✅ **AVC420 P-frames work perfectly** (confirmed again)
✅ **No corruption with standard P-frame encoding**
✅ **Our basic H.264 encoding is solid**

❌ **Haven't tested AVC444 with P-frames yet** (keep falling back to AVC420)

---

## The Configuration Problem

### Same Issue as Deblocking

**What we're doing**:
```rust
let encoder = Encoder::with_api_config(api, config)?;  // Creates and initializes

// Try to modify after creation
set_option(NUM_REF, 2)?;  // ❌ Fails with error code 4
```

**Why it fails**:
- `with_api_config()` calls `InitializeExt()` internally
- After initialization, certain parameters can't be changed via `set_option()`
- NUM_REF is one of those parameters

---

## Solutions

### Solution A: Configure via SEncParamExt Before Init

**Need to create encoder differently**:

Instead of:
```rust
Encoder::with_api_config(api, config)?
```

Do:
```rust
// Create raw encoder
// Get default SEncParamExt
// Set iNumRefFrame = 2
// Call InitializeExt() with modified params
// Wrap in Encoder
```

**Challenge**: `openh264-rs` doesn't expose this easily

---

### Solution B: Modify openh264-rs EncoderConfig

**Add to EncoderConfig**:
```rust
pub struct EncoderConfig {
    // ... existing fields ...
    pub num_ref_frames: Option<i32>,
}

impl EncoderConfig {
    pub fn num_ref_frames(mut self, num: i32) -> Self {
        self.num_ref_frames = Some(num);
        self
    }
}
```

**Then in with_api_config()**:
```rust
// Before InitializeExt():
if let Some(num) = config.num_ref_frames {
    params.iNumRefFrame = num;
}
```

**This is the clean solution but requires modifying openh264-rs**

---

### Solution C: Test with Default NUM_REF

**Question**: What's the default value?

If default is already ≥2, we might not need to configure it!

**Test**: Try Phase 2 WITHOUT the NUM_REF configuration
- Just use single encoder
- Let it use default ref frame count
- Might work

---

### Solution D: Accept AVC420 for Now

**Observation**: AVC420 with P-frames works perfectly

**Consider**:
- Is full 4:4:4 chroma critical for your use case?
- AVC420 has very good quality (you confirmed)
- Much simpler (no dual-stream complexity)
- Could be acceptable interim solution

---

## My Recommendation

### Quick Test: Remove NUM_REF Configuration

**Try Phase 2 without trying to set NUM_REF**:
- Single encoder (already done)
- P-frames enabled (already done)
- Let OpenH264 use default ref frame count
- **Might already be ≥2 by default!**

**If that works**: Don't need to configure NUM_REF at all!
**If that fails**: Then we know we need Solution B (modify openh264-rs)

---

## Next Steps

1. **Immediate**: Remove NUM_REF configuration attempt
2. **Deploy**: Single encoder with P-frames, no custom NUM_REF
3. **Test**: See if default settings work
4. **If successful**: AVC444 P-frames work with default config!
5. **If fail**: Extend openh264-rs properly

Should I try removing the NUM_REF configuration and test with defaults?
