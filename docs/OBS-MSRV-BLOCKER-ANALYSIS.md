# OBS MSRV Blocker Analysis
**Date:** 2026-01-18
**Issue:** All OBS builds failing due to upstream MSRV requirements

---

## Current State

**Dependencies causing issues:**
- openh264 0.9.1 - requires Rust 1.88
- zune-jpeg 0.5.8 - requires Rust 1.87

**Our Cargo.lock shows:**
```
openh264 = "0.9.1"
openh264-sys2 = "0.9.1"
zune-jpeg = "0.5.8"
```

**Distribution Rust versions (all too old):**
- Fedora 42: 1.85.1
- Fedora 41: ~1.85
- All others: < 1.85

---

## Potential Solutions

### Option 1: Downgrade Dependencies (Quick Fix)

**openh264:**
- Try 0.9.0 or 0.8.x if available
- Check if lower versions have acceptable MSRV

**zune-jpeg:**
- User reports PR merged (lower MSRV)
- Check if new version released to crates.io
- If not, wait for release or use older version

**Action:** Update Cargo.toml to pin older versions, rebuild vendor tarball

### Option 2: Remove Optional Features

**Build without h264 for OBS:**
```toml
# In spec file, use:
cargo build --release --offline --no-default-features --features "pam-auth"
```

**Trade-off:** No video encoding in native packages (not acceptable)

### Option 3: Wait for Upstream

**Wait for:**
- New zune-jpeg release (if PR merged)
- openh264 PR #91 to merge and release
- Distributions to update to Rust 1.88

**Timeline:** Unknown (could be weeks/months)

---

## Recommendation

**Immediate (v0.9.0):**
- ✅ Publish via Flatpak (already done, working)
- ❌ Skip OBS native packages
- Document blockers clearly

**Next (v0.9.1 or v1.0.0):**
- Check if zune-jpeg has new release with lower MSRV
- Check openh264 for lower MSRV versions
- Update dependencies and retry

Should we try downgrading dependencies now?
