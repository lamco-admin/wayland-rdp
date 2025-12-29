# IMPORTANT: Test Used AVC420, Not AVC444!

**Date**: 2025-12-29 01:15
**Finding**: Deblocking experiment failed to initialize, fell back to AVC420

---

## What Happened

### The Log Shows:

```
🔧 EXPERIMENT: Disabling deblocking filter for auxiliary encoder
⚠️  WARN: Failed to create AVC444 encoder: InitFailed("Failed to set encoder params with deblocking disabled: 4") - falling back to AVC420
✅ AVC420 encoder initialized for 1280×800 (4:2:0 fallback)
```

**Error Code 4**: OpenH264's `set_option()` rejected our parameters

**Result**: System fell back to **AVC420** (standard YUV420 encoding)

---

## Why AVC420 Works Perfectly

**AVC420** is standard H.264 YUV420:
- Y plane: Real luma
- U plane: Real chroma U
- V plane: Real chroma V
- **No chroma-as-luma encoding**
- **No deblocking issues** (everything is what H.264 expects)

This is why you saw perfect quality!

---

## The Problem

We haven't actually tested **AVC444 with deblocking disabled** yet.

The test that succeeded was:
- ❌ Not AVC444
- ✅ AVC420 (standard codec, no dual-stream complexity)

---

## Why set_option Failed

**Error code 4** from OpenH264 typically means:
- Called at wrong time (after encoder already initialized)
- Invalid parameter struct
- Need to use `InitializeExt()` instead of `set_option()`

The issue: `Encoder::with_api_config()` already calls `Initialize()`. We can't modify params after that with `set_option(ENCODER_OPTION_SVC_ENCODE_PARAM_EXT)`.

---

## What We Need to Do

### Option 1: Create Encoder from Scratch with Custom Params

Instead of using `with_api_config()`, manually create encoder with raw API:

```rust
unsafe {
    let api = OpenH264API::from_source();
    let raw = EncoderRawAPI::new(api)?;

    // Get default params
    let mut params: SEncParamExt = std::mem::zeroed();
    raw.get_default_params(&mut params);

    // Configure for our needs
    params.iPicWidth = width;
    params.iPicHeight = height;
    params.fMaxFrameRate = fps;
    params.iTargetBitrate = bitrate;
    params.iLoopFilterDisableIdc = 1;  // DISABLE DEBLOCKING

    // Initialize with custom params
    raw.initialize_ext(&params);

    // Wrap in high-level Encoder
    let encoder = Encoder::from_raw(raw)?;
}
```

### Option 2: Use set_option at Runtime

Try setting deblocking via `ENCODER_OPTION` at encode time (not init time):

```rust
// During encoding, before encode() call:
let disable_deblock: i32 = 1;
encoder.raw_api().set_option(
    ENCODER_OPTION_ENABLE_LOOP_FILTER,  // Different option?
    &disable_deblock as *const _ as *mut c_void
);
```

### Option 3: Modify openh264-rs to Expose Deblocking

Add `.deblocking(bool)` method to `EncoderConfig`:

```rust
// In openh264-rs fork:
impl EncoderConfig {
    pub fn disable_deblocking(mut self) -> Self {
        // Set flag that InitializeExt will use
        self
    }
}
```

Then upstream PR or keep in our fork.

---

## Next Steps

1. ✅ Good news: AVC420 works perfectly (confirms our color conversion/general pipeline is solid)
2. ⬜ Fix the initialization approach for AVC444
3. ⬜ Actually test AVC444 with deblocking disabled
4. ⬜ Verify P-frame corruption is eliminated

---

## Current Status

**Codec in use**: AVC420 (fallback due to init failure)
**Quality**: Perfect (as expected for standard YUV420)
**AVC444 status**: Not tested yet (initialization failed)
**Root cause hypothesis**: Still unconfirmed (need successful AVC444 test)

---

## The Fix Needed

I need to fix how we configure the auxiliary encoder. The `set_option()` approach doesn't work after `with_api_config()`.

Correct approach:
- Create raw encoder
- Call `InitializeExt()` with custom params
- Wrap in high-level Encoder

This requires understanding openh264-rs internals better.
