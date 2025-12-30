# Proper Incremental Test Plan - Phase 1

**Issue Identified**: Config.toml values not wired to encoder, deployment incomplete

---

## PROBLEMS TO FIX

### Problem 1: Configuration Not Wired

**Current**: Encoder uses hardcoded defaults (line 315 in code)
**Config.toml**: Has values but they're not passed to encoder
**Location**: `src/server/display_handler.rs` - marked with TODO

**Need**: Wire EgfxConfig values through to encoder initialization

### Problem 2: Config.toml Not Deployed

**Current**: Only binary deployed
**Missing**: Updated config.toml with Phase 1 settings
**Need**: Deploy config.toml to server

### Problem 3: All-I Blocks Omission Testing

**Current**: Lines 370-371 force all frames to IDR
**Effect**: Aux can never be omitted (keyframes require aux sync)
**Need**: Can't test omission until P-frames enabled

---

## PROPER INCREMENTAL ORDER

### Step 1: Wire Configuration (Fix Code)

**Change**: src/server/display_handler.rs
**What**: Pass config values to encoder via configure_aux_omission()
**Why**: Config.toml control instead of hardcoded values
**Test**: Verify config loads, no crashes

### Step 2: Deploy Complete Environment

**Deploy**:
1. Updated binary (with config wiring)
2. Updated config.toml (with Phase 1 settings)
3. Verify both on server

**Test**: Config values appear in logs

### Step 3: Test with All-I + Omission Enabled

**Config**: avc444_enable_aux_omission = true
**Code**: All-I still active (lines 370-371)
**Expected**: Still [BOTH SENT] every frame (keyframes block omission)
**Purpose**: Verify config wiring works, no crashes

### Step 4: Enable P-Frames

**Change**: Comment out lines 370-371 (all-I workaround)
**Rebuild and deploy**
**Expected**:
  - Main uses P-frames
  - Aux omission activates
  - Bandwidth drops dramatically
  - **CRITICAL**: Check for corruption!

**Purpose**: Full Phase 1 validation

---

## IMPLEMENTATION

### Fix 1: Wire Configuration

Need to pass egfx_config through display_handler to encoder initialization.

Options:
A. Pass config to configure_aux_omission() (already have method)
B. Access egfx_config in display_handler where encoder created
C. Pass config to Avc444Encoder::new() directly

**Best**: Option B - use existing configure_aux_omission() method, just need to access config

Let me find where egfx_config is available in display_handler...

---

**Status**: Need to implement proper config wiring first, then deploy systematically
**Next**: Fix config wiring, then follow incremental test steps
