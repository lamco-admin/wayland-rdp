# Draft GitHub Issue: zune-jpeg MSRV Reduction

## Title
Lower zune-jpeg MSRV - 1.87 only needed for optional `portable_simd` feature

## Body

### Problem

`zune-jpeg` 0.5.8 has MSRV 1.87, but:
- The crate uses `edition = "2021"` (not 2024)
- The `portable_simd` feature requiring 1.87 is **optional** (not in default features)
- Default features are: `["x86", "neon", "std"]`

This prevents packaging in distributions shipping Rust < 1.87:

| Distribution | Rust Version | zune-jpeg 0.5.8 |
|--------------|--------------|-----------------|
| Fedora 42 | 1.85.1 | ❌ Cannot build |
| Debian 13 | 1.85 | ❌ Cannot build |

### Analysis

The MSRV was set to 1.87 in commit `73b2703d` (Dec 17, 2025) with message "[#325] adding rust-version to all Cargo.toml".

Looking at the code:
```rust
#![cfg_attr(feature = "portable_simd", feature(portable_simd))]
```

The `portable_simd` feature is gated behind a feature flag. Users who don't enable it shouldn't need Rust 1.87.

### Request

The standard practice for optional features requiring newer Rust is:

1. **Set MSRV to minimum for default features** (likely ~1.60-1.65 for edition 2021)
2. **Document feature-specific requirements**: "The `portable_simd` feature requires Rust 1.87+"
3. **Let Cargo handle enforcement** - it will fail naturally if someone enables the feature on old Rust

This pattern is used by major crates like `tokio`, `serde`, `rayon`, etc. - they don't raise MSRV for the entire crate based on optional features.

The `portable_simd` code is already properly gated:
```rust
#![cfg_attr(feature = "portable_simd", feature(portable_simd))]
```

Users who don't enable this feature never compile that code path, so they shouldn't be blocked by its Rust version requirement.

### Use Case

I'm packaging software that depends on `image` crate, which pulls in `zune-jpeg`. The MSRV of 1.87 blocks all OBS (Open Build Service) builds for Fedora 42 and Debian 13.

### Workaround

Currently considering pinning to `zune-jpeg = "0.4.x"` to work around this, but would prefer to use the latest version.
