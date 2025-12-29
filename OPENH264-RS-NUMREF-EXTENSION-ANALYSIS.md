# OpenH264-RS Extension: NUM_REF in Context of Existing VUI PR

**Date**: 2025-12-29
**Existing PR**: VUI support (feature/vui-support branch)
**New Need**: NUM_REF configuration
**Question**: Same PR or separate?

---

## EXISTING VUI PR ANALYSIS

### What It Does

**Commit**: 37fc5b8 "feat(encoder): add VUI support for color space signaling"

**Purpose**: Expose VUI (Visual Usability Information) for color space signaling
- Color primaries (BT.709, BT.601, BT.2020, etc.)
- Transfer characteristics (gamma)
- Matrix coefficients
- Full range flag

**Implementation**:
```rust
// Added to EncoderConfig:
pub struct VuiConfig { ... }

impl EncoderConfig {
    pub fn vui(mut self, vui: VuiConfig) -> Self { ... }
}

// Maps to SSpatialLayerConfig fields during init
```

**File Modified**: `openh264/src/encoder.rs`
**Lines Added**: 257 lines
**Category**: EncoderConfig API extension

---

## PROPOSED NUM_REF EXTENSION

### What It Would Do

**Purpose**: Expose number of reference frames configuration
- Controls DPB (decoded picture buffer) size
- Critical for AVC444 dual-subframe encoding
- Maps to `iNumRefFrame` in SEncParamExt

**Implementation** (similar pattern):
```rust
impl EncoderConfig {
    pub fn num_ref_frames(mut self, num: i32) -> Self {
        self.num_ref_frames = Some(num);
        self
    }
}

// Maps to params.iNumRefFrame during init
```

**File Modified**: `openh264/src/encoder.rs` (SAME file as VUI)
**Lines Added**: ~20-30 lines
**Category**: EncoderConfig API extension (SAME category as VUI)

---

## COMPARISON: VUI vs NUM_REF

| Aspect | VUI Support | NUM_REF Support |
|--------|-------------|-----------------|
| **Purpose** | Color space signaling | Reference frame management |
| **File Modified** | encoder.rs | encoder.rs (SAME) |
| **Target Struct** | SSpatialLayerConfig | SEncParamExt |
| **API Pattern** | `.vui(config)` | `.num_ref_frames(num)` |
| **Complexity** | High (257 lines, VuiConfig struct) | Low (20-30 lines, simple i32) |
| **Use Case** | Color reproduction | P-frame encoding |
| **Breaking Change?** | No (Option field) | No (Option field) |
| **Related?** | Both expose hidden params | Both expose hidden params |

---

## RECOMMENDATION: Add to Same PR

### Why Bundle Together

✅ **Same Category**: Both are "EncoderConfig API improvements"
- Exposing parameters that exist in C API but not Rust API
- Following same fluent builder pattern
- Backward compatible additions

✅ **Same File**: Both modify `openh264/src/encoder.rs`
- No merge conflicts
- Easier review together
- Shows comprehensive improvement

✅ **Stronger Case to Maintainer**:
- "We're systematically improving EncoderConfig"
- Not just one-off additions
- Demonstrates understanding of the API
- More valuable contribution

✅ **Efficient**:
- One PR review instead of two
- One merge instead of two
- Less overhead for maintainer

✅ **Thematically Related**:
- VUI: "How to interpret the pixels" (color space)
- NUM_REF: "How to reference previous pixels" (temporal)
- Both are fundamental encoder configuration
- Both needed for production-quality encoding

### Proposed PR Title

**Before** (just VUI):
> "feat(encoder): add VUI support for color space signaling"

**After** (VUI + NUM_REF):
> "feat(encoder): extend EncoderConfig API (VUI + reference frame control)"

Or:
> "feat(encoder): expose VUI and reference frame configuration"

---

## IMPLEMENTATION PLAN

### Step 1: Add NUM_REF to Feature Branch

**On feature/vui-support branch**:
```bash
cd ~/openh264-rs
git checkout feature/vui-support

# Add NUM_REF support
# Commit with clear message
git commit -m "feat(encoder): add num_ref_frames configuration"

# Test
cargo test --package openh264
```

### Step 2: Update PR Description

**Add to PR description**:
```markdown
## Reference Frame Configuration

Added `num_ref_frames()` configuration method to control DPB size:

```rust
let config = EncoderConfig::new()
    .num_ref_frames(2);  // Keep 2 reference frames in DPB
```

Maps to `iNumRefFrame` in `SEncParamExt`, essential for:
- Multi-reference P-frame encoding
- Dual-stream encoding (AVC444)
- Temporal prediction control
```

### Step 3: Submit/Update PR

If PR not yet submitted → Submit with both features
If PR already in review → Add NUM_REF as additional commit

---

## ALTERNATIVE: Separate PRs

### Arguments For Separate

⚠️ **Different Purposes**:
- VUI: Color signaling (metadata)
- NUM_REF: Encoding behavior (functional)

⚠️ **Independent Features**:
- Can use VUI without NUM_REF
- Can use NUM_REF without VUI

⚠️ **Easier Review**:
- Smaller focused changes
- Clearer scope

### Arguments Against Separate

❌ **More Overhead**:
- Two PR reviews
- Two merge processes
- Fragments contribution

❌ **Artificial Split**:
- Both are EncoderConfig extensions
- Same implementation pattern
- Related to "making openh264-rs production-ready"

---

## MY RECOMMENDATION

### Add to Same PR (feature/vui-support)

**Rationale**:
1. Both are EncoderConfig API surface improvements
2. Same implementation pattern and file
3. Shows comprehensive understanding
4. More valuable as combined contribution
5. VUI establishes pattern, NUM_REF follows it

**PR Theme**: "Comprehensive EncoderConfig API Improvements"
- VUI for color correctness
- NUM_REF for temporal correctness
- Both essential for production use

### Branch Strategy

```
feature/vui-support (current branch)
  ├─ Commit 1: Add VUI support (existing)
  ├─ Commit 2: Add #[must_use] to VUI types (existing)
  └─ Commit 3: Add num_ref_frames configuration (NEW)
```

**Clean history**, thematically related, comprehensive PR.

---

## WHAT TO ADD

### Minimal Addition (20-30 lines)

```rust
// In openh264/src/encoder.rs

pub struct EncoderConfig {
    // ... existing fields ...
    /// Number of reference frames to keep in DPB
    /// Default: None (use OpenH264 default, typically 1)
    /// Range: 1-16 (limited by level)
    pub num_ref_frames: Option<i32>,
}

impl EncoderConfig {
    pub fn num_ref_frames(mut self, num: i32) -> Self {
        self.num_ref_frames = Some(num.clamp(1, 16));
        self
    }
}

// In with_api_config(), before InitializeExt():
if let Some(num) = config.num_ref_frames {
    params.iNumRefFrame = num;
}
```

**That's it!** Very simple, follows VUI pattern exactly.

---

## DECISION

**Your choice**:

**A) Add to VUI PR** (Recommended):
- I'll implement NUM_REF in feature/vui-support branch
- One cohesive PR: "EncoderConfig improvements"
- Submit/update PR with both features

**B) Separate PR**:
- Create feature/num-ref-frames branch
- Keep VUI PR as-is
- Submit separate PR

**C) Direct PR to upstream** (if you prefer):
- Create branch directly in our fork
- Reference PR #86 (the original VUI PR ralfbiedert/openh264-rs#86)
- Show we're building on that work

**Which approach do you prefer?**

My strong recommendation is **A** - they're thematically related enough (both EncoderConfig surface area improvements) and make a stronger case together.