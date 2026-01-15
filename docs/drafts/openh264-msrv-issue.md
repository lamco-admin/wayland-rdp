# Draft GitHub Issue: openh264 MSRV Reduction

## Title
Lower MSRV from 1.88 to 1.85 to enable distro packaging

## Body

### Problem

The current MSRV of 1.88 prevents packaging `openh264` in major Linux distributions:

| Distribution | Rust Version | openh264 0.9.1 |
|--------------|--------------|----------------|
| Fedora 42 | 1.85.1 | ❌ Cannot build |
| Debian 13 (Trixie) | 1.85 | ❌ Cannot build |
| openSUSE Tumbleweed | ~1.85 | ❌ Cannot build |
| Ubuntu 24.04 | 1.75 | ❌ Cannot build |

Rust 1.88 was released June 26, 2025, but most distributions are still shipping 1.85.x (the version that stabilized Rust 2024 edition).

### Analysis

The MSRV was set to 1.88 in commit `599f316b` (Sep 5, 2025) alongside the migration to `edition = "2024"`. However:

1. **Edition 2024 only requires Rust 1.85** - that's when it was stabilized
2. **No 1.88-specific features appear to be used** - the main 1.88 additions were:
   - Naked functions (`#[unsafe(naked)]`)
   - Let chains (available in edition 2024, which works on 1.85)
   - Boolean config literals (`#[cfg(true)]`)
   - Cargo cache GC

The crate uses edition 2024 features but none of the 1.86/1.87/1.88-specific stabilizations.

### Request

Could the MSRV be lowered to **1.85** (the minimum for edition 2024)?

This would immediately unblock:
- Distribution packaging for Fedora, Debian, openSUSE
- OBS (Open Build Service) automated builds
- Enterprise Linux deployment pipelines

### Use Case

I'm packaging [lamco-rdp-server](https://github.com/anthropics/lamco-rdp-server) for multiple Linux distributions via OBS. The project uses `openh264` for H.264 encoding in RDP sessions. The MSRV of 1.88 is currently the blocker for all distribution builds.

### Verification

I can test a build with the lowered MSRV if helpful. Happy to submit a PR with the change.

---

## Draft PR

**Title:** Lower MSRV from 1.88 to 1.85

**Branch:** `msrv-1.85`

**Changes:**

```diff
diff --git a/Cargo.toml b/Cargo.toml
index abc123..def456 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -7,7 +7,7 @@ members = [
 [workspace.package]
 authors = ["Ralf Biedert <rb@xr.io>"]
 edition = "2024"
 license = "BSD-2-Clause"
 repository = "https://github.com/ralfbiedert/openh264-rs"
-rust-version = "1.88"
+rust-version = "1.85"
 version = "0.9.2"
```

**PR Body:**

This PR lowers the MSRV from 1.88 to 1.85, the minimum version required for Rust 2024 edition.

### Rationale

- Edition 2024 was stabilized in Rust 1.85 (Feb 20, 2025)
- No features from Rust 1.86/1.87/1.88 appear to be used in the crate
- Major Linux distributions (Fedora 42, Debian 13) currently ship Rust 1.85.x
- This change enables distribution packaging without requiring bleeding-edge toolchains

### Testing

- [ ] CI passes on Rust 1.85
- [ ] All existing tests pass
- [ ] No compilation errors

### Impact

This is a semver-compatible change that expands the supported Rust versions.
