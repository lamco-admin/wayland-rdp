# OpenH264-RS Extension Proposal: Deblocking Filter Configuration

**Date**: 2025-12-29
**Repository**: https://github.com/glamberson/openh264-rs (feature/vui-support branch)
**Purpose**: Enable deblocking filter configuration for AVC444 auxiliary stream

---

## Problem Statement

### Current Limitation

`openh264-rs` doesn't expose deblocking filter configuration in `EncoderConfig`.

**Available**: bitrate, frame_rate, level, usage_type
**Missing**: iLoopFilterDisableIdc, iLoopFilterAlphaC0Offset, iLoopFilterBetaOffset

### Why We Need This

**AVC444 encodes chroma as luma** in the auxiliary stream. H.264's deblocking filter is tuned for luma statistics and corrupts chroma values in P-frames, causing lavender/brown artifacts.

**Solution**: Disable (or tune) deblocking for auxiliary encoder while keeping it enabled for main encoder.

**Current Workaround**: All-I frames (works but 3x bandwidth of P-frames)

---

## Test Results Confirming Root Cause

### Test 1: AVC444 with Deblocking Enabled (Default)
- **Result**: Lavender corruption in P-frames ❌
- **Log**: `Current iLoopFilterDisableIdc = 0` (enabled)

### Test 2: Attempted set_option() After Init
- **Result**: Failed with error code 4 ❌
- **Why**: Can't modify SEncParamExt after initialization

### Test 3: AVC420 Fallback
- **Result**: Perfect quality ✅
- **Why**: Standard YUV420, no chroma-as-luma issues

**Conclusion**: We need to configure deblocking DURING initialization via `InitializeExt()`.

---

## Proposed API Extension

### Option A: EncoderConfig Fluent API (RECOMMENDED)

Add deblocking configuration to `EncoderConfig`:

```rust
// In openh264/src/encoder.rs

pub struct EncoderConfig {
    // ... existing fields ...

    /// Deblocking filter control
    /// - None: Use default (enabled)
    /// - Some(0): Enable deblocking
    /// - Some(1): Disable completely
    /// - Some(2): Disable across slice boundaries only
    pub loop_filter_disable_idc: Option<i32>,

    /// Alpha C0 offset for deblocking filter (-6 to +6)
    /// - Negative: Less filtering
    /// - Positive: More filtering
    pub loop_filter_alpha_c0_offset: Option<i32>,

    /// Beta offset for deblocking filter (-6 to +6)
    pub loop_filter_beta_offset: Option<i32>,
}

impl EncoderConfig {
    /// Disable deblocking filter completely
    pub fn disable_deblocking(mut self) -> Self {
        self.loop_filter_disable_idc = Some(1);
        self
    }

    /// Set deblocking filter offsets for fine-tuning
    pub fn deblocking_offsets(mut self, alpha_c0: i32, beta: i32) -> Self {
        self.loop_filter_alpha_c0_offset = Some(alpha_c0.clamp(-6, 6));
        self.loop_filter_beta_offset = Some(beta.clamp(-6, 6));
        self
    }
}
```

**Usage**:
```rust
// For auxiliary encoder (chroma-as-luma)
let aux_config = EncoderConfig::new()
    .bitrate(BitRate::from_bps(5_000_000))
    .max_frame_rate(FrameRate::from_hz(30))
    .disable_deblocking();  // ← THE FIX

let aux_encoder = Encoder::with_api_config(api, aux_config)?;

// For main encoder (normal luma)
let main_config = EncoderConfig::new()
    .bitrate(BitRate::from_bps(5_000_000))
    .max_frame_rate(FrameRate::from_hz(30));
    // Default deblocking (enabled)

let main_encoder = Encoder::with_api_config(api, main_config)?;
```

**Implementation in openh264-rs**:

Modify `Encoder::with_api_config()` to apply deblocking settings:

```rust
// In openh264/src/encoder.rs, inside with_api_config():

// After getting default params but before InitializeExt():
if let Some(idc) = config.loop_filter_disable_idc {
    params.iLoopFilterDisableIdc = idc;
}
if let Some(alpha) = config.loop_filter_alpha_c0_offset {
    params.iLoopFilterAlphaC0Offset = alpha;
}
if let Some(beta) = config.loop_filter_beta_offset {
    params.iLoopFilterBetaOffset = beta;
}

// Then call InitializeExt with modified params
raw_api.initialize_ext(&params)?;
```

---

### Option B: Separate Config Struct

```rust
pub struct DebloockingConfig {
    pub disable_idc: i32,
    pub alpha_c0_offset: i32,
    pub beta_offset: i32,
}

impl EncoderConfig {
    pub fn deblocking(mut self, config: DeblockingConfig) -> Self {
        self.deblocking_config = Some(config);
        self
    }
}
```

**Pro**: More structured
**Con**: More verbose to use

---

### Option C: Raw Params Access

```rust
impl EncoderConfig {
    pub fn with_raw_params<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut SEncParamExt),
    {
        self.raw_params_modifier = Some(Box::new(f));
        self
    }
}
```

**Usage**:
```rust
let aux_config = EncoderConfig::new()
    .bitrate(...)
    .with_raw_params(|params| {
        params.iLoopFilterDisableIdc = 1;
        params.iComplexityMode = LOW_COMPLEXITY;
    });
```

**Pro**: Maximum flexibility
**Con**: Exposes raw structs, less type-safe

---

## Recommendation

**Option A** (Fluent API) is cleanest:
- Type-safe
- Ergonomic
- Follows existing pattern (`.bitrate()`, `.max_frame_rate()`, etc.)
- Easy to document
- Backward compatible (all fields Option<>)

---

## Implementation Plan

### Step 1: Modify openh264-rs Fork

**File**: `openh264/src/encoder.rs`

**Changes**:
1. Add 3 fields to `EncoderConfig` struct
2. Add `.disable_deblocking()` method
3. Add `.deblocking_offsets()` method
4. Modify `with_api_config()` to apply settings before `InitializeExt()`

**Lines changed**: ~20-30
**Risk**: Low (additive change, no breaking modifications)

### Step 2: Test Locally

```bash
cd /path/to/openh264-rs
cargo test --package openh264
```

### Step 3: Use in Our Project

```rust
// src/egfx/avc444_encoder.rs

let aux_config = OpenH264Config::new()
    .bitrate(BitRate::from_bps(config.bitrate_kbps * 1000))
    .max_frame_rate(FrameRate::from_hz(config.max_fps))
    .disable_deblocking();  // ← New method

let aux_encoder = Encoder::with_api_config(api, aux_config)?;
```

### Step 4: Test AVC444 with Deblocking Actually Disabled

Expected: NO lavender corruption, P-frames work perfectly

---

## Alternative: Quick Hack for Testing

If you want to test immediately without clean API:

**Directly modify encoder creation in openh264-rs**:

```rust
// In openh264-rs/openh264/src/encoder.rs, with_api_config():
// Around line 880-900, after get_default_params():

params.iLoopFilterDisableIdc = 1;  // ← HARDCODE for testing

// Then InitializeExt()
```

Build openh264-rs locally, point Cargo.toml to local path, test.

**Pro**: 1-minute test
**Con**: Not a real solution, just for validation

---

## Estimated Effort

**Clean API Extension** (Option A):
- Implementation: 30-60 minutes
- Testing: 15 minutes
- Total: 1 hour

**Quick Hack**:
- Implementation: 5 minutes
- Testing: 5 minutes
- Total: 10 minutes

---

## Your Decision

**Question 1**: Do you want the **quick hack first** to 100% confirm deblocking is the issue?
- Hardcode `iLoopFilterDisableIdc = 1` in openh264-rs
- Build and test immediately
- If successful, then do clean API

**Question 2**: Or go straight to **clean API extension**?
- Proper `.disable_deblocking()` method
- Takes 1 hour but done right
- No intermediate step

**Question 3**: Do you have the openh264-rs fork locally?
- If yes: Easy to modify
- If no: Need to clone first

**My Recommendation**: Quick hack first (10 min) to validate 100%, then clean API (1 hour) for production.
