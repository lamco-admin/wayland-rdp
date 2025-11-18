# IRONRDP METACRATE SOLUTION - AUTHORITATIVE
**Issue:** Dependency conflicts with ironrdp 0.13 metacrate
**Resolution:** Use server-only features to avoid client components
**Status:** VERIFIED AND TESTED

## THE SOLUTION

Use metacrate with explicit server-only features:

```toml
ironrdp = { version = "0.13.0", default-features = false, features = ["server", "tokio", "svc"] }
```

This avoids sspi/picky-krb conflicts by excluding the `connector` feature.

Complete Cargo.toml in repo at: 02-TECHNOLOGY-STACK.md (updated)
