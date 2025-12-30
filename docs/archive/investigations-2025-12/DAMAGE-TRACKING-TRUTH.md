# The Truth About Damage Tracking Configuration

**Date**: 2025-12-29 20:20 UTC
**Finding**: Config.toml `damage_tracking.enabled` is **IGNORED**
**Reality**: Damage tracking is **ALWAYS ON** (hardcoded)
**Reason**: Config was never wired to display_handler

---

## THE EVIDENCE

### Code Location: src/server/display_handler.rs (Line ~667)

```rust
// Damage detection for bandwidth optimization
let mut damage_detector = DamageDetector::new(DamageConfig::default());
```

**Analysis**:
- ✅ Creates DamageDetector UNCONDITIONALLY
- ✅ Uses `DamageConfig::default()` (hardcoded)
- ❌ Does NOT check `config.damage_tracking.enabled`
- ❌ Does NOT receive config values

**Proof**: No `if` statement, no config parameter, always created.

---

### Why Config Isn't Checked

**Display handler constructor** (src/server/display_handler.rs):
```rust
pub async fn new(
    initial_width: u16,
    initial_height: u16,
    pipewire_fd: i32,
    stream_info: Vec<StreamInfo>,
    graphics_tx: Option<mpsc::Sender<GraphicsFrame>>,
    gfx_server_handle: Option<Arc<RwLock<Option<GfxServerHandle>>>>,
    gfx_handler_state: Option<Arc<RwLock<Option<HandlerState>>>>,
) -> Result<Self>
```

**What it receives**: Width, height, PipeWire FD, stream info, EGFX handles

**What it does NOT receive**: `Config` or `damage_tracking` config!

**Result**: Display handler has NO ACCESS to config values, can't check `enabled` flag

---

### Server Initialization (src/server/mod.rs)

**Where display_handler is created**:
```rust
let display_handler = Arc::new(
    WrdDisplayHandler::new(
        initial_size.0,
        initial_size.1,
        pipewire_fd,
        stream_info.to_vec(),
        Some(graphics_tx),
        Some(gfx_server_handle),
        Some(gfx_handler_state),
    )
    .await?
);
```

**Config is NOT passed** - it's used earlier for Portal setup, but not forwarded.

---

## WHAT'S ACTUALLY HARDCODED

### DamageConfig::default() Values

**From src/damage/mod.rs**:
```rust
impl Default for DamageConfig {
    fn default() -> Self {
        Self {
            tile_size: 64,
            diff_threshold: 0.05,
            pixel_threshold: 4,
            merge_distance: 32,
            min_region_area: 256,
        }
    }
}
```

**These exact values are ALWAYS used**, regardless of config.toml settings.

---

## HISTORICAL IMPACT

### All Our Tests Had Damage Tracking

**Test 1** (4.40 MB/s, all-I): Had damage tracking ✅
**Test 2** (dual encoder): Had damage tracking ✅
**Single encoder tests**: Had damage tracking ✅
**Final 0.81 MB/s**: Had damage tracking ✅

**The `enabled = false` setting had ZERO effect** on any test!

**This is actually GOOD** - it's been working and optimizing bandwidth the whole time.

---

## THE REAL CONFIG.TOML STATE

### What Actually Works

**These config sections WORK** (values are used):
- ✅ `[egfx]` - Most values used (h264_level, h264_bitrate, etc.)
- ✅ `[egfx]` Phase 1 aux omission - **NOT wired yet** (but defaults in code)
- ✅ `[server]` - All values used
- ✅ `[security]` - All values used
- ✅ `[input]` - Values used
- ✅ `[clipboard]` - Values used

### What DOESN'T Work

**These config sections are IGNORED**:
- ❌ `[damage_tracking]` - **ALL values ignored** (hardcoded)
- ❌ `[egfx]` aux omission fields - Not wired to encoder
- ❌ `[hardware_encoding]` - Partially wired (some values unused)

---

## OPTIONS GOING FORWARD

### Option 1: Wire All Configs Properly (Comprehensive)

**Scope**: Wire damage_tracking AND aux omission configs

**Changes needed**:
1. Pass `Arc<Config>` to WrdDisplayHandler::new()
2. Store config in display_handler struct
3. Create DamageDetector conditionally based on enabled flag
4. Pass config values to DamageConfig
5. Similar for aux omission (configure_aux_omission with config values)

**Effort**: 3-4 hours
**Benefit**: Config.toml fully functional
**Testing**: Verify all config combinations

### Option 2: Document As-Is (Quick)

**Update config.toml with reality**:

```toml
[damage_tracking]
# NOTE: Damage tracking is currently ALWAYS ENABLED (hardcoded)
# The following settings have NO EFFECT in current implementation
# Damage tracking uses hardcoded defaults: tile_size=64, diff_threshold=0.05
# To disable or configure: requires code changes (see DAMAGE-TRACKING-TRUTH.md)
#
# This will be fixed in Phase 2 configuration enhancement
enabled = true  # Informational - actually always enabled
tile_size = 64  # Informational - actual value hardcoded
diff_threshold = 0.05  # Informational - actual value hardcoded
merge_distance = 32  # Informational - actual value hardcoded
```

**Effort**: 15 minutes
**Benefit**: Honest documentation
**Downside**: Config still not functional

### Option 3: Remove Misleading Config (Honest)

**Remove damage_tracking section entirely**:

```toml
# Damage tracking is currently hardcoded as always-on
# Configuration will be added in Phase 2
# For details see: DAMAGE-TRACKING-TRUTH.md
```

**Effort**: 5 minutes
**Benefit**: No misleading settings
**Downside**: Less discoverable

---

## MY RECOMMENDATION

**Do Option 1** (Wire configs properly):

**Why**:
1. It's the right fix
2. You're building a commercial product
3. Config should actually work
4. Users expect config.toml to control behavior
5. Only 3-4 hours to fix properly

**Wire BOTH**:
- damage_tracking config
- aux omission config (also not wired)

**Make config.toml actually functional** for your commercial product.

---

**What would you like to do?**

1. **Option 1**: Wire configs properly (3-4 hours, right fix)
2. **Option 2**: Document as-is (15 min, honest but incomplete)
3. **Option 3**: Remove misleading config (5 min, clean)

Given you're building a commercial product, I'd recommend Option 1 - let's make the configuration system actually work properly.

**Should I implement Option 1 (wire all configs)?**
