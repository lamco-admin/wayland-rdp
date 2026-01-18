# MSRV Fork Solution for OBS Builds
**Date:** 2026-01-18
**Issue:** openh264 0.9.1 requires Rust 1.88, OBS has ≤1.85

---

## Solution: Fork and Patch MSRV

### openh264-rs Fork

**Location:** ~/openh264-rs-fork (local)
**Branch:** lamco-lower-msrv
**Changes:**
- rust-version: 1.88 → 1.77
- edition: 2024 → 2021

**To complete:**
1. Create github.com/lamco-admin/openh264-rs repository
2. Push lamco-lower-msrv branch
3. Update lamco-rdp-server Cargo.toml to use fork

**Patch in Cargo.toml:**
```toml
[patch.crates-io]
openh264 = { git = "https://github.com/lamco-admin/openh264-rs", branch = "lamco-lower-msrv" }
openh264-sys2 = { git = "https://github.com/lamco-admin/openh264-rs", branch = "lamco-lower-msrv" }
```

### zune-jpeg Fork (if needed)

Similar approach if new version not released with lower MSRV.

---

## Next Steps

1. Create GitHub repo: lamco-admin/openh264-rs
2. Push fork
3. Update Cargo.toml patches
4. Revendor dependencies
5. Re-upload to OBS
6. Monitor builds
