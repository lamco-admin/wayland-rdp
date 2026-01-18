# Publishing Pipeline Recommendations
**Date:** 2026-01-18
**Based On:** Comprehensive research across wrd-server-specs and lamco-admin
**Purpose:** Define immediate publishing strategy for lamco-rdp-server

---

## Executive Summary

**Current State:** You have **complete packaging infrastructure** ready to go:
- ✅ Flatpak bundle built and tested (6.9 MB)
- ✅ OBS configured with 7 working targets (Fedora, openSUSE, Debian, AlmaLinux)
- ✅ RPM and DEB specs complete
- ✅ Code humanization protocols documented
- ✅ Vendored tarball ready (65 MB)

**Missing:** Only metadata and final publication steps remain.

---

## RECOMMENDATION: 2-Tier Publishing Strategy

### Tier 1: Launch Immediately (Essential Channels)

**These 4 channels give you 95%+ coverage with minimal overhead:**

#### 1. **OBS (Open Build Service)** ✅ ALREADY WORKING
**Why:** 7 distributions, automatic rebuilds, existing infrastructure
**Effort:** 30 min/release (upload tarball, trigger builds)
**Users Reached:** RHEL/Fedora/openSUSE/Debian users
**Status:** READY - Just need to upload latest tarball

**Action Items:**
- [ ] Upload `lamco-rdp-server-0.1.0.tar.xz` to OBS
- [ ] Trigger builds for all 7 targets
- [ ] Monitor build results
- [ ] Publish successful builds

**Time:** 30 minutes

---

#### 2. **Flatpak (Flathub)** ✅ MANIFEST READY
**Why:** Largest reach (4M+ users), universal Linux compatibility, single submission
**Effort:** 2-4 hrs first time, 30 min/release after
**Users Reached:** All Linux distros
**Status:** Need MetaInfo XML + icons, then submit

**Action Items (First-Time Setup):**
- [ ] Create `io.lamco.rdp-server.metainfo.xml` (app metadata)
- [ ] Create desktop file `io.lamco.rdp-server.desktop`
- [ ] Add icons (128x128 minimum)
- [ ] Fork flathub/flathub repo
- [ ] Submit PR with manifest

**Ongoing (Per Release):**
- [ ] Update manifest source URL/hash
- [ ] Commit to Flathub fork
- [ ] Auto-builds on Flathub

**Time:** 2-4 hours first time, 30 min per release

**BSL Compatibility:** YES - Flathub allows proprietary licenses (shown as "Proprietary" badge)

---

#### 3. **GitHub Releases** ✅ TARBALL READY
**Why:** Direct downloads, official source, trusted by users
**Effort:** 15-30 min/release
**Users Reached:** Advanced users, developers
**Status:** Just need to create public repo and first release

**Action Items:**
- [ ] Verify public repo exists (github.com/lamco-admin/lamco-rdp-server)
- [ ] Create v0.1.0 release tag
- [ ] Attach source tarball
- [ ] Attach Flatpak bundle
- [ ] Write release notes

**Time:** 30 minutes

---

#### 4. **crates.io** ⏳ OPTIONAL (Binary Crate)
**Why:** Rust ecosystem visibility, `cargo install` support
**Effort:** 1 hour first time, 15 min/release
**Users Reached:** Rust developers
**Status:** Need to decide if publishing binary crate makes sense

**Pros:**
- `cargo install lamco-rdp-server` works
- Rust community visibility
- Automatic dependency resolution

**Cons:**
- Users must have Rust installed (defeats point of binary distribution)
- Build time on user machine (defeats pre-built packages)
- Less useful for end users than Flatpak/RPM

**Recommendation:** **SKIP for now** - Focus on pre-built packages (Flatpak/RPM/DEB)

---

### Tier 2: Add Later (Expanded Reach)

**Add these when Tier 1 is stable:**

#### 5. **AppImage** (Universal Binary)
**Why:** Single-file downloads, no installation required
**Effort:** 1-2 hours setup, then automated via GitHub Actions
**Users Reached:** Users who prefer portable executables

**Setup:**
- Create AppImageBuilder recipe
- GitHub Actions workflow
- Automatic builds on release tags

**Time:** 1-2 hours setup, then automatic

---

#### 6. **AUR (Arch User Repository)**
**Why:** Arch Linux users expect packages here
**Effort:** 30 min setup, 15 min/release (or community maintains)
**Users Reached:** Arch/Manjaro users

**Setup:**
- Generate PKGBUILD with `cargo-aur`
- Create AUR account
- Publish package
- (Often community takes over maintenance)

**Time:** 30 minutes

---

#### 7. **Copr (Fedora/RHEL)**
**Why:** Alternative to OBS for Fedora ecosystem
**Effort:** 1-2 hours setup, 30 min/release

**Setup:**
- FAS (Fedora Account System) account
- Adapt RPM spec from OBS
- Configure automated builds

**Time:** 1-2 hours

---

## RECOMMENDED IMMEDIATE ACTION PLAN

### Week 1: Tier 1 Launch (Essential Channels)

**Day 1-2: Code Publication Protocol**
- [ ] Run humanization audit on src/ (use lamco-admin/shared/humanization/QUICK-CHECKLIST.md)
- [ ] Remove AI attributions
- [ ] Fix any "critical" tells (helper soup, generic names)
- [ ] Create LICENSE file with BSL 1.1 text
- [ ] Verify all Cargo.toml metadata is public-ready

**Day 3: GitHub Publication**
- [ ] Create clean public repo (if not exists)
- [ ] Push clean code
- [ ] Create v0.1.0 release with tarball + Flatpak bundle

**Day 4: OBS Upload**
- [ ] Upload tarball to OBS lamco project
- [ ] Trigger builds for all 7 targets
- [ ] Monitor build logs
- [ ] Verify packages work

**Day 5: Flathub Preparation**
- [ ] Create MetaInfo XML (app description, screenshots, changelog)
- [ ] Create desktop file
- [ ] Add application icons
- [ ] Test locally with appstream validation
- [ ] Submit Flathub PR

**Result:** lamco-rdp-server available via:
- ✅ OBS repositories (Fedora, RHEL, openSUSE, Debian)
- ✅ GitHub releases (direct downloads)
- ⏳ Flathub (pending review, 1-2 week approval)

---

### Week 2+: Tier 2 Expansion (Optional)

- AppImage automation
- AUR publication
- Copr setup

---

## CODE PUBLICATION PROTOCOL (lamco-admin Framework)

### Required Steps Before ANY Public Code

**Based on:** lamco-admin/shared/humanization framework

#### 1. Humanization Audit (MANDATORY)

**Quick scan:**
```bash
cd ~/wayland/wrd-server-specs

# Check for helper soup
rg -l "utils|helpers|common" src/

# Check for generic function names
rg "fn (process|handle|do_|run)\s*\(" src/

# Check for AI attributions
rg "claude|anthropic|generated|co-authored" src/ docs/
```

**Full checklist:** Use `~/lamco-admin/shared/humanization/QUICK-CHECKLIST.md`

**Critical fixes (always required):**
- Remove ALL AI attributions
- Fix "what" comments (restate code) → "why" comments (explain decisions)
- Eliminate helper soup modules
- Replace generic function names with domain-specific names
- Remove excessive logging (>3 debug statements per function)

#### 2. Code Quality Verification

```bash
cargo fmt --check
cargo clippy --all-features -- -D warnings
cargo build --release --all-features
cargo test
cargo doc --no-deps
```

#### 3. Metadata Audit

- [ ] Cargo.toml: authors, description, repository, keywords correct
- [ ] LICENSE file exists with correct terms
- [ ] README.md is public-appropriate (no internal references)
- [ ] No .claude/, SESSION-*.md, or internal docs in public code

#### 4. Final Verification

```bash
# Grep scan for AI references
rg "AI generated|generated by|generated with" --type rust

# Check package contents
cargo package --list
```

---

## PACKAGING REQUIREMENTS CHECKLIST

### For ALL Packages (Universal)

**Required in Repository:**
- [ ] LICENSE (with BSL 1.1 text)
- [ ] LICENSE-APACHE (for future conversion)
- [ ] README.md (user-focused, installation instructions)
- [ ] config.toml.example (comprehensive configuration reference)
- [ ] bundled-crates/ (lamco-clipboard-core, lamco-rdp-clipboard)
- [ ] src/ (all source code, humanized)
- [ ] Cargo.toml (complete metadata)
- [ ] Cargo.lock (reproducible builds)
- [ ] build.rs (build-time metadata capture)

**Excluded from Repository:**
- certs/*.pem (test certificates)
- target/ (build artifacts)
- vendor/ (recreated during package build)
- .claude/ (AI session data)
- logs/ (runtime logs)
- docs/SESSION-*.md (internal handover docs)
- docs/archive/ccw/ (AI conversation archives)

---

### Flatpak-Specific Requirements

**Required Files:**
- [ ] `io.lamco.rdp-server.yml` (manifest) ✅ EXISTS
- [ ] `io.lamco.rdp-server.metainfo.xml` (app metadata) ❌ MISSING
- [ ] `io.lamco.rdp-server.desktop` (desktop integration) ❌ MISSING
- [ ] Icons: 128x128 PNG minimum ❌ MISSING
- [ ] Screenshots (for Flathub listing) ❌ MISSING

**MetaInfo XML Template Location:** Will need to create

---

### RPM-Specific Requirements

**Required Files:**
- [ ] `lamco-rdp-server.spec` ✅ EXISTS at packaging/lamco-rdp-server.spec
- [ ] Source tarball with vendor/ ✅ EXISTS (65 MB)
- [ ] systemd service file ✅ EXISTS at packaging/systemd/

**RPM Build Process:**
```bash
# Upload to OBS or build locally
rpmbuild -ba packaging/lamco-rdp-server.spec
```

---

### DEB-Specific Requirements

**Required Files:**
- [ ] `debian/control` ✅ EXISTS
- [ ] `debian/rules` ✅ EXISTS
- [ ] `debian/changelog` ✅ EXISTS
- [ ] `debian/copyright` ✅ EXISTS
- [ ] `debian/*.service` ✅ EXISTS

**DEB Build Process:**
```bash
dpkg-buildpackage -us -uc -b
```

---

## DISTRIBUTION CHANNEL COMPARISON

| Channel | Setup Time | Per-Release Time | User Reach | Automation | Recommend |
|---------|------------|------------------|------------|------------|-----------|
| **OBS** | ✅ Done | 30 min | Medium | Manual | ✅ YES (Tier 1) |
| **Flathub** | 2-4 hrs | 30 min | Very High | Auto-build | ✅ YES (Tier 1) |
| **GitHub Releases** | 30 min | 15 min | High | Manual | ✅ YES (Tier 1) |
| **crates.io** | 1 hr | 15 min | Low (binary) | Manual | ⚠️ SKIP (binary crate) |
| **AppImage** | 1-2 hrs | 0 min | Medium | Auto (CI) | 🟡 Later (Tier 2) |
| **AUR** | 30 min | 15 min | Medium | Manual | 🟡 Later (Tier 2) |
| **Copr** | 1-2 hrs | 30 min | Low | Manual | 🟡 Later (Tier 2) |

---

## MY RECOMMENDATIONS

### Immediate Focus (Next 1-2 Weeks)

**Publish via these 3 channels FIRST:**

1. **OBS** (Already configured, lowest effort)
   - Upload tarball
   - Trigger builds
   - Packages available in hours

2. **GitHub Releases** (Direct downloads, source distribution)
   - Create public repo
   - Tag v0.1.0
   - Attach tarball and Flatpak bundle

3. **Flathub** (Maximum user reach)
   - Create MetaInfo XML and icons
   - Submit to Flathub
   - Wait for review (1-2 weeks)

**Total effort:** ~1 week for all three

### Code Publication Protocol

**Before pushing to GitHub:**

1. **Run humanization scan** (30-60 min)
   - Use ~/lamco-admin/shared/humanization/QUICK-CHECKLIST.md
   - Fix critical tells (AI attributions, helper soup, generic names)
   - Verify code quality (fmt, clippy, build, test)

2. **Create LICENSE file** (15 min)
   - Add BSL 1.1 text with your parameters
   - Add LICENSE-APACHE for reference

3. **Clean export** (30 min)
   - Remove .claude/, docs/archive/ccw/, SESSION-*.md
   - Keep only public-appropriate documentation

**Total effort:** 1-2 hours of cleanup

### Skip for Now

- ❌ crates.io (binary crate has little value)
- ❌ AppImage (add later if demand)
- ❌ AUR (add later if Arch users request)
- ❌ Copr (OBS already covers Fedora/RHEL)

---

## NEXT STEPS (In Order)

Based on your plan and these findings:

**Step 1: Code Humanization** (1-2 hours)
- Run humanization checklist on src/
- Remove AI attributions
- Fix critical tells

**Step 2: Licensing** (30 min)
- Create LICENSE file
- Update Cargo.toml

**Step 3: OBS Publication** (30 min)
- Upload tarball to OBS
- Trigger builds
- Verify packages

**Step 4: GitHub Release** (1 hour)
- Clean code export
- Create v0.1.0 release
- Attach artifacts

**Step 5: Flathub Submission** (2-4 hours)
- Create MetaInfo and icons
- Submit PR
- Wait for review

**Total Time:** 5-8 hours spread over 1-2 weeks

---

## DECISION POINTS FOR YOU

### Question 1: Publishing Channels

Which channels do you want to activate NOW?

**My recommendation:**
- ✅ OBS (lowest effort, already configured)
- ✅ GitHub Releases (essential for direct downloads)
- ✅ Flathub (maximum reach)
- ❌ Skip: crates.io, AppImage, AUR, Copr (for now)

**Your decision:** _____________

### Question 2: Code Humanization Depth

How thoroughly should I humanize the code?

**Options:**
A. **Minimal** (1 hour) - Remove AI attributions only, fix obvious tells
B. **Standard** (2-3 hours) - Full checklist, all critical + moderate fixes
C. **Comprehensive** (1-2 days) - Deep audit, all tells fixed including subtle ones

**My recommendation:** B (Standard) - Good enough for v0.1.0, refine later

**Your decision:** _____________

### Question 3: Public Repo Strategy

How should I handle the public repository?

**Options:**
A. **Clean export** - New repo, clean history, humanized code only
B. **Full history** - Push entire wrd-server-specs history to public
C. **Squashed** - Squash dev history into clean initial commits

**My recommendation:** A (Clean export) - Follows lamco-admin protocol

**Your decision:** _____________

---

## DETAILED WORKFLOWS

### Workflow 1: OBS Publication

**Prerequisites:**
- ✅ Tarball exists: `packaging/lamco-rdp-server-0.1.0.tar.xz`
- ✅ RPM spec exists: `packaging/lamco-rdp-server.spec`
- ✅ DEB files exist: `packaging/debian/*`

**Steps:**
1. Log into OBS web UI (https://192.168.10.8, Admin/opensuse)
2. Navigate to project `lamco`
3. Upload `lamco-rdp-server-0.1.0.tar.xz`
4. Update package manifest (if needed)
5. Trigger builds for all targets
6. Monitor build results
7. Publish successful builds to repositories

**Alternative (CLI):**
```bash
# Upload tarball
osc -A https://192.168.10.8 checkout lamco lamco-rdp-server
cd lamco/lamco-rdp-server
cp ~/wayland/wrd-server-specs/packaging/lamco-rdp-server-0.1.0.tar.xz .
osc add lamco-rdp-server-0.1.0.tar.xz
osc commit -m "Update to v0.1.0"
```

**Time:** 30 minutes

---

### Workflow 2: Flathub Submission

**Prerequisites:**
- ✅ Manifest ready: `packaging/io.lamco.rdp-server.yml`
- ❌ MetaInfo XML (need to create)
- ❌ Desktop file (need to create)
- ❌ Icons (need to create)

**Step 1: Create MetaInfo XML** (15-30 min)

Create `packaging/io.lamco.rdp-server.metainfo.xml`:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<component type="console-application">
  <id>io.lamco.rdp-server</id>
  <name>Lamco RDP Server</name>
  <summary>Professional RDP server for Wayland/Linux desktop sharing</summary>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>BUSL-1.1</project_license>

  <description>
    <p>
      lamco-rdp-server is a production-grade RDP server for Linux desktops
      running Wayland. It provides secure remote desktop access using XDG
      Desktop Portal and PipeWire, with no X11 dependencies.
    </p>
    <p>Features:</p>
    <ul>
      <li>Multi-strategy session persistence (5 strategies for different compositors)</li>
      <li>Hardware-accelerated H.264 video encoding (VA-API, NVENC)</li>
      <li>Bidirectional clipboard with file transfer support</li>
      <li>Multi-monitor layout management</li>
      <li>Damage-based bandwidth optimization</li>
    </ul>
  </description>

  <url type="homepage">https://lamco.ai</url>
  <url type="bugtracker">https://github.com/lamco-admin/lamco-rdp-server/issues</url>

  <launchable type="desktop-id">io.lamco.rdp-server.desktop</launchable>

  <releases>
    <release version="0.1.0" date="2026-01-18">
      <description>
        <p>Initial production release</p>
      </description>
    </release>
  </releases>

  <content_rating type="oars-1.1"/>
</component>
```

**Step 2: Create Desktop File** (5 min)

Create `packaging/io.lamco.rdp-server.desktop`:
```ini
[Desktop Entry]
Type=Application
Name=Lamco RDP Server
Comment=Wayland RDP Server
Exec=lamco-rdp-server
Icon=io.lamco.rdp-server
Terminal=true
Categories=Network;RemoteAccess;
Keywords=rdp;remote;desktop;wayland;
```

**Step 3: Add Icons** (15 min)
- Create or find icon (128x128 minimum, SVG preferred)
- Add to packaging/icons/

**Step 4: Update Manifest** (5 min)
- Add MetaInfo to manifest sources
- Add desktop file to manifest sources

**Step 5: Submit to Flathub** (30 min)
```bash
# Fork flathub/flathub
# Clone your fork
git clone https://github.com/YOUR-USERNAME/flathub
cd flathub

# Create application directory
mkdir io.lamco.rdp-server
cd io.lamco.rdp-server

# Copy files
cp ~/wayland/wrd-server-specs/packaging/io.lamco.rdp-server.yml .
cp ~/wayland/wrd-server-specs/packaging/io.lamco.rdp-server.metainfo.xml .

# Create PR to flathub/flathub
git checkout -b add-lamco-rdp-server
git add io.lamco.rdp-server/
git commit -m "Add io.lamco.rdp-server"
git push origin add-lamco-rdp-server

# Create PR on GitHub
```

**Step 6: Wait for Review**
- Flathub team reviews (typically 1-2 weeks)
- Address any feedback
- Once merged, auto-builds on Flathub infrastructure

**Total Time:** 1.5-2.5 hours first time

---

### Workflow 3: GitHub Release

**Prerequisites:**
- ✅ Tarball ready
- ✅ Flatpak bundle ready
- ⏳ Public repo (verify exists)

**Steps:**
1. Verify public repo: `cd ~/lamco-rdp-server && git remote -v`
2. Create release tag: `git tag -a v0.1.0 -m "Release v0.1.0"`
3. Push tag: `git push origin v0.1.0`
4. Create GitHub release via web UI or CLI:
   ```bash
   gh release create v0.1.0 \
     packaging/lamco-rdp-server-0.1.0.tar.xz \
     packaging/lamco-rdp-server.flatpak \
     --title "lamco-rdp-server v0.1.0" \
     --notes-file RELEASE-NOTES.md
   ```

**Time:** 30 minutes

---

## INFRASTRUCTURE READINESS MATRIX

| Component | Status | Location | Action Required |
|-----------|--------|----------|-----------------|
| **OBS Appliance** | ✅ Ready | 192.168.10.8 | Upload tarball |
| **OBS Project** | ✅ Configured | lamco/lamco-rdp-server | None |
| **RPM Spec** | ✅ Complete | packaging/lamco-rdp-server.spec | None |
| **DEB Control** | ✅ Complete | packaging/debian/* | None |
| **Flatpak Manifest** | ✅ Complete | packaging/io.lamco.rdp-server.yml | None |
| **Flatpak Bundle** | ✅ Built | packaging/lamco-rdp-server.flatpak | None |
| **Vendor Tarball** | ✅ Created | packaging/lamco-rdp-server-0.1.0.tar.xz | None |
| **systemd Services** | ✅ Complete | packaging/systemd/* | None |
| **Flathub MetaInfo** | ❌ Missing | Need to create | Create XML + desktop file |
| **LICENSE File** | ❌ Missing | Need to create | Add BSL 1.1 text |
| **Public Repo** | ⏳ Verify | ~/lamco-rdp-server exists | Verify state |

---

## MY FINAL RECOMMENDATIONS

### For Immediate Publication (This Week)

**Priority 1: OBS** (30 minutes)
- You have 7 working build targets
- Just upload tarball and trigger builds
- Packages available same day

**Priority 2: GitHub Releases** (1 hour after humanization)
- Essential for direct downloads
- Required for Flathub source reference
- Shows professional project management

**Priority 3: Flathub** (2-4 hours setup)
- Maximum user reach
- One-time setup, then automatic builds
- Worth the initial investment

**SKIP (For Now):**
- crates.io (little value for binary crate)
- AppImage (add later if users request)
- AUR (add later if Arch users request)

### Code Publication Priority

**Before ANYTHING goes public:**
1. Humanization audit (use QUICK-CHECKLIST.md)
2. Remove ALL AI attributions
3. Add BSL LICENSE file
4. Clean export to public repo

**Time Investment:** 2-3 hours of cleanup for professional public release

---

## WHAT DO YOU WANT TO DO?

I'm ready to execute based on your decisions:

A. **Start with humanization audit** - Scan code for AI tells, create fix list
B. **Create Flathub files** - MetaInfo XML, desktop file, icons
C. **Create LICENSE file** - Add BSL 1.1 text
D. **Review OBS status** - Check what's actually deployed
E. **Something else** - What's your priority?

Tell me your decisions on the 3 questions above and which action to start with.
