# Public Repo Export Manifest
**Target:** ~/lamco-rdp-server → github.com/lamco-admin/lamco-rdp-server
**Version:** 0.9.0
**Purpose:** Define EXACTLY what goes in public repo

---

## ✅ INCLUDE (Essential Files Only)

### Root Files
- README.md
- LICENSE
- LICENSE-APACHE
- CHANGELOG.md
- INSTALL.md
- CONTRIBUTING.md
- Cargo.toml
- Cargo.lock
- build.rs
- config.toml.example
- .gitignore

### Source Code
- src/ (entire directory, all *.rs files)

### Bundled Dependencies
- bundled-crates/lamco-clipboard-core/
- bundled-crates/lamco-rdp-clipboard/

### Packaging
- packaging/io.lamco.rdp-server.yml (Flatpak manifest)
- packaging/flatpak.yaml (Flatpak alternate)
- packaging/lamco-rdp-server.spec (RPM spec)
- packaging/lamco-rdp-server.dsc (Debian source control)
- packaging/debian/ (all Debian packaging files)
- packaging/systemd/ (systemd service files)
- packaging/create-vendor-tarball.sh (vendoring script)

### Scripts
- scripts/generate-certs.sh
- scripts/build.sh
- scripts/setup.sh
- scripts/verify-dependencies.sh

### Benchmarks
- benches/ (all benchmark files)

### Tests
- tests/ (integration tests)

### Examples
- examples/ (code examples)

### Build Configuration
- .cargo/config.toml (vendor configuration)

---

## ❌ EXCLUDE (Internal/Development Only)

### Internal Documentation
- docs/archive/
- docs/implementation/
- docs/testing/
- docs/strategy/
- docs/handover/
- docs/bugfixes/
- docs/diagnostics/
- docs/decisions/
- docs/design/
- docs/drafts/
- docs/analysis/
- docs/*SESSION*.md
- docs/*AUDIT*.md
- docs/*ANALYSIS*.md
- docs/*ASSESSMENT*.md
- docs/*RECOMMENDATIONS*.md
- docs/*PLAN*.md
- docs/*SUMMARY*.md
- docs/*COMPLETE*.md
- docs/*HANDOVER*.md
- docs/*STATUS*.md

### Build Artifacts
- target/
- vendor/
- packaging/vendor/
- packaging/build-dir/
- packaging/repo/
- packaging/.flatpak-builder/
- packaging/*.flatpak
- packaging/*.tar.xz
- packaging/*.log

### Development Files
- .claude/
- logs/
- test-package/
- config/*.toml (except config.toml.example)
- certs/*.pem (test certificates)
- *.local.*
- *.orig
- *.rej

### Archive Content
- archive/ (ALL of it - contains CCW prompts, old logs, internal history)

---

## 🤔 UNCERTAIN (Need Your Decision)

### Documentation - Which to Include?

**Technical Architecture Docs (Very Detailed):**
- docs/architecture/SESSION-PERSISTENCE-ARCHITECTURE.md (129K - comprehensive)
- docs/architecture/NVENC-AND-COLOR-INFRASTRUCTURE.md (15K)
- docs/architecture/CLIPBOARD-ARCHITECTURE-FINAL.md (34K)
- docs/architecture/*.md (11 files total)

**Implementation Guides:**
- docs/WLR-FULL-IMPLEMENTATION.md (21K - wlroots guide)
- docs/SERVICE-REGISTRY-TECHNICAL.md (technical reference)
- docs/DISTRO-TESTING-MATRIX.md (compatibility matrix)

**IronRDP Integration:**
- docs/ironrdp/ (8 files - IronRDP integration docs)

**User Guides:**
- docs/guides/QUICKSTART.md
- docs/guides/TESTING-ENVIRONMENT-RECOMMENDATIONS.md
- docs/guides/*.md (9 files total)

**Configuration & Deployment:**
- docs/CONFIGURATION.md
- docs/FLATPAK-DEPLOYMENT.md
- docs/HARDWARE-ENCODING-BUILD-GUIDE.md

**Website Content:**
- docs/website/ (website drafts - PRODUCT-PAGE.md, TECHNOLOGY-*.md, etc.)

### Config Examples
- config/rhel9-config.toml (platform-specific example)
- config/test-egfx.toml (test configuration)
- config/archive/ (old configs)

### GNOME Extension
- extension/ (GNOME Shell clipboard extension)

---

## MY RECOMMENDATION

**Include ONLY:**
- ✅ Root files (README, LICENSE, CHANGELOG, INSTALL, CONTRIBUTING, config.toml.example)
- ✅ src/, bundled-crates/, packaging/, scripts/, benches/, tests/, examples/
- ✅ Build files (Cargo.toml, Cargo.lock, build.rs, .cargo/)
- ✅ **MAYBE:** docs/DISTRO-TESTING-MATRIX.md (useful for users to see supported platforms)
- ✅ **MAYBE:** docs/WLR-FULL-IMPLEMENTATION.md (wlroots users need this)
- ❌ **EXCLUDE:** Everything else in docs/ (rely on website per your strategy)
- ❌ **EXCLUDE:** extension/ (GNOME-specific, publish separately if needed)
- ❌ **EXCLUDE:** config/rhel9-config.toml, config/test-*.toml (testing artifacts)
- ❌ **EXCLUDE:** examples/ (unless you want code examples public)

**Rationale:** Minimal essential files + source code + packaging. All other docs go on website (lamco.ai).

---

## YOUR DECISION NEEDED

**Option A: Minimal (Recommended)**
- Root docs only (README, LICENSE, CHANGELOG, INSTALL, CONTRIBUTING, config.toml.example)
- Source code (src/, bundled-crates/)
- Packaging (packaging/, scripts/, .cargo/)
- Build (Cargo.*, build.rs, .gitignore)
- NO docs/, NO examples/, NO extension/, NO extra configs

**Option B: With Key Docs**
- Everything from Option A
- PLUS: docs/DISTRO-TESTING-MATRIX.md, docs/WLR-FULL-IMPLEMENTATION.md
- Still NO internal docs

**Option C: You Tell Me Exactly**
- List exactly what you want

**Which option? Or specify exactly what to include?**
