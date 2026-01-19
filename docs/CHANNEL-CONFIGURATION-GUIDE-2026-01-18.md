# Distribution Channel Configuration Guide
**Date:** 2026-01-18
**Project:** lamco-rdp-server v0.9.0
**Status:** Step-by-step implementation procedures
**Goal:** Configure Flathub, AppImage, and GitHub Releases for production downloads

---

## Current Status

**What's Complete:**
- ✅ v0.9.0 published to GitHub (commit ca53612, 2a541ce)
- ✅ Flatpak bundle built (io.lamco.rdp-server.flatpak, 6.7 MB)
- ✅ Source tarball built (lamco-rdp-server-0.9.0-final.tar.xz, 65 MB)
- ✅ Flatpak manifest exists (packaging/io.lamco.rdp-server.yml)

**What's Needed:**
- ⏳ Flathub submission (MetaInfo XML, desktop file, icons)
- ⏳ AppImage automation (recipe, GitHub Actions)
- ⏳ GitHub Releases enhancement (SHA256SUMS, improved notes)
- ⏳ Website download integration

---

## Phase 1: Asset Preparation (No Screenshots Needed)

### Important Note: CLI Application

lamco-rdp-server is a **CLI application with no graphical interface**. This affects Flathub submission:
- ✅ Screenshots are OPTIONAL for console applications
- ✅ Can use terminal output screenshots instead
- ✅ Focus on MetaInfo quality and description

### Asset Checklist

**Required for All Channels:**
- [x] Icon (check if exists on lamco.ai website)
- [ ] Desktop file (io.lamco.rdp-server.desktop)
- [ ] MetaInfo XML (io.lamco.rdp-server.metainfo.xml)

**Optional:**
- [ ] Terminal screenshots (--help output, --show-capabilities)
- [ ] Connection demo screenshot (RDP client connected)

---

## Phase 2: Flathub Submission Configuration

### Step 2.1: Extract Icon from Website

**Action:**
```bash
cd ~/lamco-admin/projects/lamco-rdp-server
mkdir -p assets/icons

# Download icon from lamco.ai website
# (You'll need to provide the icon file or URL)

# If SVG available:
cp /path/to/lamco-rdp-icon.svg assets/icons/io.lamco.rdp-server.svg

# Generate PNG sizes
for size in 48 64 128 256; do
  convert -resize ${size}x${size} -background none \
    assets/icons/io.lamco.rdp-server.svg \
    assets/icons/io.lamco.rdp-server-${size}.png
done

# Copy to public repo
mkdir -p ~/lamco-rdp-server/packaging/icons
cp assets/icons/* ~/lamco-rdp-server/packaging/icons/
```

**If you provide the icon location, I can complete this step.**

---

### Step 2.2: Create Desktop File

**File:** `~/lamco-rdp-server/packaging/io.lamco.rdp-server.desktop`

**Content:**
```desktop
[Desktop Entry]
Type=Application
Name=lamco RDP Server
GenericName=Remote Desktop Server
Comment=Remote access to your Linux desktop via RDP
Icon=io.lamco.rdp-server
Exec=lamco-rdp-server --help
Terminal=true
Categories=Network;RemoteAccess;System;
Keywords=rdp;remote;desktop;server;wayland;screenshare;
StartupNotify=false
```

**Action:**
```bash
cd ~/lamco-rdp-server

cat > packaging/io.lamco.rdp-server.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=lamco RDP Server
GenericName=Remote Desktop Server
Comment=Remote access to your Linux desktop via RDP
Icon=io.lamco.rdp-server
Exec=lamco-rdp-server --help
Terminal=true
Categories=Network;RemoteAccess;System;
Keywords=rdp;remote;desktop;server;wayland;screenshare;
StartupNotify=false
EOF

# Validate
desktop-file-validate packaging/io.lamco.rdp-server.desktop
```

---

### Step 2.3: Create MetaInfo XML (Using Website Content)

**File:** `~/lamco-rdp-server/packaging/io.lamco.rdp-server.metainfo.xml`

**Content (based on lamco.ai product page):**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<component type="console-application">
  <id>io.lamco.rdp-server</id>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>LicenseRef-proprietary=https://mariadb.com/bsl11/</project_license>

  <name>lamco RDP Server</name>
  <summary>Remote access to your existing Linux desktop via RDP</summary>

  <description>
    <p>
      lamco RDP Server provides remote access to your existing Linux desktop session
      via the industry-standard Remote Desktop Protocol (RDP). Built specifically for
      Wayland compositors, it delivers hardware-accelerated H.264 video encoding,
      full keyboard and mouse support, and clipboard synchronization.
    </p>
    <p>
      Perfect for remote work, system administration, or accessing your Linux desktop
      from Windows, macOS, or other Linux machines. Works with GNOME, KDE Plasma,
      Sway, and Hyprland through XDG Desktop Portals and PipeWire integration.
    </p>
    <p>
      <strong>Licensing:</strong> Free for personal use, non-profits, and small businesses
      (≤3 employees OR &lt;$1M revenue). Commercial use requires license from office@lamco.io.
      Business Source License converts to Apache-2.0 on December 31, 2028.
    </p>
    <p>
      Visit https://lamco.ai for licensing details and commercial options.
    </p>
  </description>

  <launchable type="desktop-id">io.lamco.rdp-server.desktop</launchable>

  <screenshots>
    <screenshot type="default">
      <caption>Service capabilities and runtime detection</caption>
      <image>https://raw.githubusercontent.com/lamco-admin/lamco-rdp-server/main/docs/screenshots/capabilities.png</image>
    </screenshot>
  </screenshots>

  <url type="homepage">https://lamco.ai/products/lamco-rdp-server/</url>
  <url type="bugtracker">https://github.com/lamco-admin/lamco-rdp-server/issues</url>
  <url type="help">https://github.com/lamco-admin/lamco-rdp-server/blob/main/README.md</url>
  <url type="vcs-browser">https://github.com/lamco-admin/lamco-rdp-server</url>
  <url type="contact">https://lamco.ai/contact/</url>

  <provides>
    <binary>lamco-rdp-server</binary>
  </provides>

  <releases>
    <release version="0.9.0" date="2026-01-18">
      <description>
        <p>Initial public release with core remote desktop functionality.</p>
        <ul>
          <li>Wayland-native screen capture via XDG Desktop Portals</li>
          <li>H.264 video encoding (AVC420, AVC444)</li>
          <li>Hardware acceleration support (NVENC, VA-API)</li>
          <li>Full keyboard and mouse input via Portal RemoteDesktop</li>
          <li>Bidirectional clipboard synchronization</li>
          <li>Multi-monitor support</li>
          <li>Adaptive frame rate (5-60 FPS)</li>
          <li>Runtime service discovery (18 Wayland services)</li>
          <li>Multi-strategy session persistence architecture</li>
          <li>TLS 1.3 encryption</li>
          <li>Tested on GNOME (Ubuntu 24.04, RHEL 9)</li>
        </ul>
      </description>
      <url>https://github.com/lamco-admin/lamco-rdp-server/releases/tag/v0.9.0</url>
    </release>
  </releases>

  <content_rating type="oars-1.1">
    <content_attribute id="social-info">moderate</content_attribute>
  </content_rating>

  <developer id="io.lamco">
    <name>Lamco Development</name>
  </developer>

  <update_contact>office@lamco.io</update_contact>

  <categories>
    <category>Network</category>
    <category>RemoteAccess</category>
  </categories>

  <keywords>
    <keyword>rdp</keyword>
    <keyword>remote desktop</keyword>
    <keyword>wayland</keyword>
    <keyword>screen sharing</keyword>
    <keyword>server</keyword>
  </keywords>
</component>
```

**Action:**
```bash
cd ~/lamco-rdp-server

cat > packaging/io.lamco.rdp-server.metainfo.xml << 'XMLEOF'
[paste XML content above]
XMLEOF

# Validate
appstream-util validate packaging/io.lamco.rdp-server.metainfo.xml
```

---

### Step 2.4: Create Optional Screenshot

**Option A: Capabilities Output Screenshot**

```bash
cd ~/lamco-rdp-server

# Run and capture capabilities output
flatpak run io.lamco.rdp-server --show-capabilities > capabilities.txt

# Create terminal screenshot of output
# (Use terminal emulator screenshot feature or script below)

mkdir -p docs/screenshots

# If you want to create a clean terminal screenshot programmatically:
# Use 'termshot' or capture manually
```

**Option B: Skip Screenshots**

For console applications, Flathub doesn't strictly require screenshots. You can:
- Skip the `<screenshots>` section in MetaInfo XML
- Or provide one screenshot showing terminal usage
- Flathub reviewers may request screenshots, you can add then

**Recommendation:** Start without screenshots, add if reviewers request.

---

### Step 2.5: Update Flatpak Manifest for Flathub

**File:** `~/lamco-rdp-server/packaging/io.lamco.rdp-server.yml`

**Add MetaInfo and Desktop File Installation:**

```bash
cd ~/lamco-rdp-server

# Edit manifest to add installation of MetaInfo and desktop file
# After the cargo install command, add:
```

Check current manifest and add these lines after the cargo install:

```yaml
    # Install MetaInfo
    - install -Dm644 packaging/io.lamco.rdp-server.metainfo.xml /app/share/metainfo/io.lamco.rdp-server.metainfo.xml

    # Install desktop file
    - install -Dm644 packaging/io.lamco.rdp-server.desktop /app/share/applications/io.lamco.rdp-server.desktop

    # Install icons
    - install -Dm644 packaging/icons/io.lamco.rdp-server.svg /app/share/icons/hicolor/scalable/apps/io.lamco.rdp-server.svg
    - install -Dm644 packaging/icons/io.lamco.rdp-server-128.png /app/share/icons/hicolor/128x128/apps/io.lamco.rdp-server.png
    - install -Dm644 packaging/icons/io.lamco.rdp-server-64.png /app/share/icons/hicolor/64x64/apps/io.lamco.rdp-server.png
    - install -Dm644 packaging/icons/io.lamco.rdp-server-48.png /app/share/icons/hicolor/48x48/apps/io.lamco.rdp-server.png
```

---

### Step 2.6: Test Flatpak Build with MetaInfo

```bash
cd ~/lamco-rdp-server

# Build Flatpak with new MetaInfo/desktop/icons
flatpak-builder --force-clean --repo=repo build-dir packaging/io.lamco.rdp-server.yml

# Verify MetaInfo installed
ls -la build-dir/files/share/metainfo/
# Should show: io.lamco.rdp-server.metainfo.xml

# Verify desktop file
ls -la build-dir/files/share/applications/
# Should show: io.lamco.rdp-server.desktop

# Validate MetaInfo
appstream-util validate build-dir/files/share/metainfo/io.lamco.rdp-server.metainfo.xml

# Create test bundle
flatpak build-bundle repo lamco-rdp-server-flathub-test.flatpak io.lamco.rdp-server

# Install and test
flatpak install --user lamco-rdp-server-flathub-test.flatpak
flatpak run io.lamco.rdp-server --help
```

---

### Step 2.7: Fork and Submit to Flathub

**Flathub Submission Process:**

**1. Fork Flathub Repository**
```bash
# On GitHub: https://github.com/flathub/flathub
# Click "Fork" button
```

**2. Clone Your Fork**
```bash
cd ~/lamco-admin/projects/lamco-rdp-server
mkdir -p flathub-submission
cd flathub-submission

git clone https://github.com/YOUR-GITHUB-USERNAME/flathub.git
cd flathub
```

**3. Create Application Branch**
```bash
git checkout -b add-io-lamco-rdp-server
```

**4. Add Manifest**
```bash
# Create app directory
mkdir io.lamco.rdp-server

# Copy manifest
cp ~/lamco-rdp-server/packaging/io.lamco.rdp-server.yml io.lamco.rdp-server/

# Copy MetaInfo
cp ~/lamco-rdp-server/packaging/io.lamco.rdp-server.metainfo.xml io.lamco.rdp-server/

# Note: Icons will be installed from manifest, don't need to copy separately
```

**5. Verify Manifest for Flathub**

Ensure manifest uses HTTPS source URLs (not local paths):

```bash
cd io.lamco.rdp-server

# Check source section points to GitHub release
grep -A5 "sources:" io.lamco.rdp-server.yml
```

**Expected:**
```yaml
sources:
  - type: archive
    url: https://github.com/lamco-admin/lamco-rdp-server/releases/download/v0.9.0/lamco-rdp-server-0.9.0-final.tar.xz
    sha256: [actual SHA256 hash]
```

**If manifest uses local path, update it:**
```bash
# Calculate SHA256 of tarball
sha256sum ~/lamco-rdp-server/packaging/lamco-rdp-server-0.9.0-final.tar.xz

# Update manifest with GitHub URL and hash
```

**6. Test Build with Flathub Tools**
```bash
cd ~/lamco-admin/projects/lamco-rdp-server/flathub-submission/flathub

# Use flatpak-builder to test
flatpak-builder --force-clean --repo=test-repo test-build io.lamco.rdp-server/io.lamco.rdp-server.yml

# If successful, proceed to PR
```

**7. Commit and Push**
```bash
git add io.lamco.rdp-server/
git commit -m "Add io.lamco.rdp-server

lamco RDP Server provides remote access to existing Linux desktop
sessions via the industry-standard Remote Desktop Protocol.

- License: Business Source License 1.1 (converts to Apache-2.0 on 2028-12-31)
- Free for personal use, non-profits, small businesses
- Commercial license required for businesses >3 employees or >$1M revenue
- Homepage: https://lamco.ai/products/lamco-rdp-server/
- Source: https://github.com/lamco-admin/lamco-rdp-server

Tested on Ubuntu 24.04 (GNOME 46) and RHEL 9 (GNOME 40)."

git push origin add-io-lamco-rdp-server
```

**8. Create Pull Request**

Go to: https://github.com/flathub/flathub

- Click "New Pull Request"
- Select your fork and branch: `YOUR-USERNAME:add-io-lamco-rdp-server`
- Title: **"Add io.lamco.rdp-server"**
- Description:

```markdown
This PR adds lamco RDP Server, a Wayland-native RDP server for Linux.

**Application Details:**
- **Name:** lamco RDP Server
- **App ID:** io.lamco.rdp-server
- **Category:** Network, Remote Access
- **License:** Business Source License 1.1 (proprietary, source-available)
- **Free Use:** Personal, non-profit, small businesses (≤3 employees OR <$1M revenue)
- **Commercial Use:** Requires license from office@lamco.io
- **Future:** Converts to Apache-2.0 on December 31, 2028

**Description:**
Remote desktop server for Linux with native Wayland integration. Enables remote
access to GNOME, KDE, and wlroots desktop sessions via RDP. Uses XDG Desktop
Portals for screen capture and input injection, PipeWire for video streaming,
and H.264 encoding for efficient bandwidth usage.

**Testing:**
- Built and tested locally on Fedora 42
- Manifest validated with appstream-util
- MetaInfo XML validated
- Desktop file validated
- Tested on Ubuntu 24.04 (GNOME 46) and RHEL 9 (GNOME 40)

**Screenshots:**
Console application (CLI). Screenshots optional per Flathub guidelines for console apps.
Can provide terminal output screenshots if requested.

**Homepage:** https://lamco.ai/products/lamco-rdp-server/
**Source:** https://github.com/lamco-admin/lamco-rdp-server
**Documentation:** https://github.com/lamco-admin/lamco-rdp-server/blob/main/README.md

**BSL License Note:**
This application uses Business Source License 1.1, which allows free redistribution
and use for non-production purposes. Flathub guidelines permit proprietary licenses
that allow redistribution. The license will convert to Apache-2.0 (fully open source)
on December 31, 2028.
```

**9. Respond to Reviewer Feedback**

Flathub reviewers typically respond within 3-7 days. Common requests:
- MetaInfo improvements
- Icon quality
- License clarification
- Build verification

Respond promptly to maintain review momentum.

---

### Step 2.8: Create Flathub Update Procedure

**File:** `~/lamco-admin/projects/lamco-rdp-server/FLATHUB-RELEASE-PROCEDURE.md`

**Content:**
```markdown
# Flathub Release Procedure

## Per-Release Updates (30 minutes)

### 1. Update Flatpak Manifest

Edit `io.lamco.rdp-server.yml` in Flathub repo:

```yaml
sources:
  - type: archive
    url: https://github.com/lamco-admin/lamco-rdp-server/releases/download/v{VERSION}/lamco-rdp-server-{VERSION}.tar.xz
    sha256: [new hash]
```

Calculate new SHA256:
```bash
sha256sum lamco-rdp-server-{VERSION}.tar.xz
```

### 2. Update MetaInfo XML

Add new release entry:

```xml
<releases>
  <release version="{VERSION}" date="{YYYY-MM-DD}">
    <description>
      <p>[Release summary]</p>
      <ul>
        <li>[Feature 1]</li>
        <li>[Feature 2]</li>
      </ul>
    </description>
    <url>https://github.com/lamco-admin/lamco-rdp-server/releases/tag/v{VERSION}</url>
  </release>
  <!-- Previous releases below -->
</releases>
```

### 3. Submit Flathub PR

```bash
cd ~/lamco-admin/projects/lamco-rdp-server/flathub-submission/flathub
git checkout main
git pull upstream main
git checkout -b update-{VERSION}

# Make changes
git add io.lamco.rdp-server/
git commit -m "Update io.lamco.rdp-server to {VERSION}"
git push origin update-{VERSION}

# Create PR on GitHub
```

### 4. Monitor Build

Flathub buildbot automatically builds your update. Check build logs in PR.

### 5. Merge and Publish

Once approved and merged, app updates on Flathub within 24 hours.

Users update via: `flatpak update io.lamco.rdp-server`
```

---

## Phase 3: AppImage Automation Configuration

### Step 3.1: Create AppImageBuilder Recipe

**File:** `~/lamco-rdp-server/packaging/appimage.yml`

**Content:**
```yaml
version: 1

AppDir:
  path: ./AppDir

  app_info:
    id: io.lamco.rdp-server
    name: lamco-rdp-server
    icon: io.lamco.rdp-server
    version: !ENV ${VERSION}
    exec: usr/bin/lamco-rdp-server
    exec_args: $@

  apt:
    arch: amd64
    sources:
      - sourceline: 'deb http://archive.ubuntu.com/ubuntu/ jammy main restricted universe multiverse'
      - sourceline: 'deb http://archive.ubuntu.com/ubuntu/ jammy-updates main restricted universe multiverse'

    include:
      # Core libraries
      - libc6
      - libgcc-s1
      - libstdc++6
      - libssl3
      # PipeWire
      - libpipewire-0.3-0
      - libspa-0.2-0
      # Wayland
      - libwayland-client0
      # Portal/D-Bus
      - libdbus-1-3
      # PAM (if enabled)
      - libpam0g
      # Additional
      - libfuse3-3

  files:
    exclude:
      - usr/share/man
      - usr/share/doc
      - usr/share/lintian
      - usr/share/icons/hicolor/icon-theme.cache
      - usr/lib/x86_64-linux-gnu/gconv
      - usr/share/locale

  runtime:
    env:
      APPDIR_LIBRARY_PATH: '$APPDIR/usr/lib/x86_64-linux-gnu:$APPDIR/lib/x86_64-linux-gnu'

  test:
    fedora:
      image: appimagecrafters/tests-env:fedora-latest
      command: ./AppRun --help
    debian:
      image: appimagecrafters/tests-env:debian-stable
      command: ./AppRun --help
    arch:
      image: appimagecrafters/tests-env:archlinux-latest
      command: ./AppRun --help

AppImage:
  update-information: 'gh-releases-zsync|lamco-admin|lamco-rdp-server|latest|lamco-rdp-server-*x86_64.AppImage.zsync'
  sign-key: None
  arch: x86_64
```

**Action:**
```bash
cd ~/lamco-rdp-server

cat > packaging/appimage.yml << 'YAMLEOF'
[paste YAML content above]
YAMLEOF
```

---

### Step 3.2: Create GitHub Actions Workflow for AppImage

**File:** `~/lamco-rdp-server/.github/workflows/appimage.yml`

**Content:**
```yaml
name: Build AppImage

on:
  release:
    types: [published]
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to build (e.g., 0.9.1)'
        required: false

jobs:
  build-appimage:
    runs-on: ubuntu-22.04

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Set version
        id: version
        run: |
          if [ "${{ github.event_name }}" = "release" ]; then
            echo "VERSION=${GITHUB_REF#refs/tags/v}" >> $GITHUB_OUTPUT
          elif [ -n "${{ github.event.inputs.version }}" ]; then
            echo "VERSION=${{ github.event.inputs.version }}" >> $GITHUB_OUTPUT
          else
            echo "VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)" >> $GITHUB_OUTPUT
          fi

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: stable

      - name: Install build dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libssl-dev \
            libpipewire-0.3-dev \
            libdbus-1-dev \
            libpam0g-dev \
            pkg-config \
            nasm

      - name: Build binary
        run: |
          cargo build --release --no-default-features --features "h264,libei"

      - name: Prepare AppDir
        run: |
          mkdir -p AppDir/usr/bin
          mkdir -p AppDir/usr/share/applications
          mkdir -p AppDir/usr/share/icons/hicolor/scalable/apps
          mkdir -p AppDir/usr/share/icons/hicolor/128x128/apps
          mkdir -p AppDir/usr/share/icons/hicolor/64x64/apps
          mkdir -p AppDir/usr/share/icons/hicolor/48x48/apps

          # Copy binary
          cp target/release/lamco-rdp-server AppDir/usr/bin/

          # Copy desktop file
          cp packaging/io.lamco.rdp-server.desktop AppDir/usr/share/applications/

          # Copy icons
          cp packaging/icons/io.lamco.rdp-server.svg AppDir/usr/share/icons/hicolor/scalable/apps/
          cp packaging/icons/io.lamco.rdp-server-128.png AppDir/usr/share/icons/hicolor/128x128/apps/io.lamco.rdp-server.png
          cp packaging/icons/io.lamco.rdp-server-64.png AppDir/usr/share/icons/hicolor/64x64/apps/io.lamco.rdp-server.png
          cp packaging/icons/io.lamco.rdp-server-48.png AppDir/usr/share/icons/hicolor/48x48/apps/io.lamco.rdp-server.png

      - name: Build AppImage
        run: |
          # Install appimage-builder
          sudo apt-get install -y python3-pip python3-setuptools patchelf desktop-file-utils libgdk-pixbuf2.0-dev fakeroot strace
          sudo pip3 install appimage-builder

          # Build AppImage
          VERSION=${{ steps.version.outputs.VERSION }} appimage-builder --recipe packaging/appimage.yml --skip-test

      - name: Rename AppImage
        run: |
          mv *.AppImage lamco-rdp-server-${{ steps.version.outputs.VERSION }}-x86_64.AppImage || true
          mv *.AppImage.zsync lamco-rdp-server-${{ steps.version.outputs.VERSION }}-x86_64.AppImage.zsync || true

      - name: Upload to Release
        if: github.event_name == 'release'
        uses: softprops/action-gh-release@v2
        with:
          files: |
            lamco-rdp-server-*.AppImage
            lamco-rdp-server-*.AppImage.zsync
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Upload as Artifact
        uses: actions/upload-artifact@v4
        with:
          name: AppImage
          path: |
            lamco-rdp-server-*.AppImage
            lamco-rdp-server-*.AppImage.zsync
```

**Action:**
```bash
cd ~/lamco-rdp-server

mkdir -p .github/workflows

cat > .github/workflows/appimage.yml << 'WORKFLOWEOF'
[paste YAML content above]
WORKFLOWEOF
```

---

### Step 3.3: Test AppImage Build Locally

**Option A: Using Docker (Recommended)**

```bash
cd ~/lamco-rdp-server

# Build binary first
cargo build --release --no-default-features --features "h264,libei"

# Prepare AppDir
mkdir -p AppDir/usr/bin
cp target/release/lamco-rdp-server AppDir/usr/bin/

# Copy desktop integration files
mkdir -p AppDir/usr/share/applications
mkdir -p AppDir/usr/share/icons/hicolor/{scalable,128x128,64x64,48x48}/apps

cp packaging/io.lamco.rdp-server.desktop AppDir/usr/share/applications/
cp packaging/icons/io.lamco.rdp-server.svg AppDir/usr/share/icons/hicolor/scalable/apps/
cp packaging/icons/io.lamco.rdp-server-128.png AppDir/usr/share/icons/hicolor/128x128/apps/io.lamco.rdp-server.png
cp packaging/icons/io.lamco.rdp-server-64.png AppDir/usr/share/icons/hicolor/64x64/apps/io.lamco.rdp-server.png
cp packaging/icons/io.lamco.rdp-server-48.png AppDir/usr/share/icons/hicolor/48x48/apps/io.lamco.rdp-server.png

# Build AppImage with Docker
docker run --rm -v $(pwd):/build -w /build \
  appimagecrafters/appimage-builder:latest \
  appimage-builder --recipe packaging/appimage.yml --skip-test

# Test AppImage
chmod +x *.AppImage
./lamco-rdp-server-*-x86_64.AppImage --help
```

**Option B: Local Installation**

```bash
# Install appimage-builder
sudo apt-get install python3-pip python3-setuptools patchelf desktop-file-utils libgdk-pixbuf2.0-dev fakeroot strace
sudo pip3 install appimage-builder

# Build
VERSION=0.9.0 appimage-builder --recipe packaging/appimage.yml --skip-test

# Test
chmod +x *.AppImage
./lamco-rdp-server-*-x86_64.AppImage --help
```

---

### Step 3.4: Commit AppImage Configuration

```bash
cd ~/lamco-rdp-server

git add packaging/appimage.yml .github/workflows/appimage.yml
git commit -m "Add AppImage build automation

- AppImageBuilder recipe for Ubuntu 22.04 base
- GitHub Actions workflow for automatic builds on release
- Includes desktop integration (icon, .desktop file)
- Generates zsync for delta updates
- Zero maintenance per release (fully automated)"

git push origin main
```

---

### Step 3.5: Trigger Test Build

```bash
cd ~/lamco-rdp-server

# Option A: Manual workflow dispatch (test without release)
# Go to GitHub Actions → AppImage workflow → Run workflow

# Option B: Create test tag
git tag -a v0.9.1-appimage-test -m "Test AppImage automation"
git push origin v0.9.1-appimage-test

# Monitor build at: https://github.com/lamco-admin/lamco-rdp-server/actions

# Option C: Wait for next actual release (v0.9.1)
```

---

## Phase 4: GitHub Releases Enhancement

### Step 4.1: Create SHA256SUMS File

**For Current v0.9.0:**

```bash
cd ~/lamco-rdp-server/packaging

# Calculate hashes
sha256sum \
  lamco-rdp-server-0.9.0-final.tar.xz \
  lamco-rdp-server-0.9.0.flatpak \
  > SHA256SUMS-0.9.0.txt

# View
cat SHA256SUMS-0.9.0.txt
```

**Upload to existing v0.9.0 release:**

```bash
cd ~/lamco-rdp-server/packaging

# Using gh CLI
gh release upload v0.9.0 SHA256SUMS-0.9.0.txt

# Or manually via GitHub web UI:
# https://github.com/lamco-admin/lamco-rdp-server/releases/edit/v0.9.0
```

---

### Step 4.2: Improve Release Notes Template

**File:** `~/lamco-admin/projects/lamco-rdp-server/GITHUB-RELEASE-NOTES-TEMPLATE.md`

**Content:**
```markdown
# lamco-rdp-server v{VERSION}

Remote access to your existing Linux desktop via RDP.

## 🎉 What's New in v{VERSION}

{CHANGELOG_HIGHLIGHTS}

## 📦 Installation

### Flatpak (Recommended - Universal)

**Via Flathub:**
```bash
flatpak install flathub io.lamco.rdp-server
flatpak run io.lamco.rdp-server
```

**Via Direct Bundle:**
```bash
wget https://github.com/lamco-admin/lamco-rdp-server/releases/download/v{VERSION}/lamco-rdp-server-{VERSION}.flatpak
flatpak install lamco-rdp-server-{VERSION}.flatpak
flatpak run io.lamco.rdp-server
```

### AppImage (Portable)

```bash
wget https://github.com/lamco-admin/lamco-rdp-server/releases/download/v{VERSION}/lamco-rdp-server-{VERSION}-x86_64.AppImage
chmod +x lamco-rdp-server-{VERSION}-x86_64.AppImage
./lamco-rdp-server-{VERSION}-x86_64.AppImage --help
```

### Native Packages

**Fedora 40+:**
```bash
sudo dnf install lamco-rdp-server
```

**RHEL 9 / AlmaLinux 9 / Rocky 9:**
```bash
sudo dnf install lamco-rdp-server
```

**openSUSE Tumbleweed / Leap 15.6:**
```bash
sudo zypper install lamco-rdp-server
```

**Debian 13 (Trixie):**
```bash
sudo apt install lamco-rdp-server
```

**Ubuntu 24.04 / Debian 12:** Use Flatpak (Rust toolchain incompatibility)

**Arch Linux:** Build from source or wait for AUR package

### From Source

```bash
# Download source
wget https://github.com/lamco-admin/lamco-rdp-server/releases/download/v{VERSION}/lamco-rdp-server-{VERSION}.tar.xz
tar xf lamco-rdp-server-{VERSION}.tar.xz
cd lamco-rdp-server-{VERSION}

# Build
cargo build --release --features=h264,libei

# Install
sudo cp target/release/lamco-rdp-server /usr/local/bin/
```

## 📋 Release Assets

- **lamco-rdp-server-{VERSION}.tar.xz** - Source code with vendored dependencies (65 MB)
- **lamco-rdp-server-{VERSION}.flatpak** - Universal Flatpak bundle (6.5 MB)
- **lamco-rdp-server-{VERSION}-x86_64.AppImage** - Portable AppImage (TBD MB)
- **SHA256SUMS-{VERSION}.txt** - Checksums for verification

## 📄 Licensing

**Business Source License 1.1**

- ✅ **Free** for personal use, non-profits, small businesses (≤3 employees OR <$1M revenue)
- 📧 **Commercial use** requires license from **office@lamco.io**
- 🔄 **Converts to Apache-2.0** on December 31, 2028 (fully open source)

See [Pricing Details](https://lamco.ai/pricing/) for commercial licensing options.

## 📖 Documentation

- **Installation Guide:** [INSTALL.md](https://github.com/lamco-admin/lamco-rdp-server/blob/main/INSTALL.md)
- **Feature Support Matrix:** [docs/FEATURE-SUPPORT-MATRIX.md](https://github.com/lamco-admin/lamco-rdp-server/blob/main/docs/FEATURE-SUPPORT-MATRIX.md)
- **Platform Compatibility:** [Product Page](https://lamco.ai/products/lamco-rdp-server/)
- **Configuration Reference:** [config.toml.example](https://github.com/lamco-admin/lamco-rdp-server/blob/main/config.toml.example)

## 🐛 Support

- **Bug Reports:** [GitHub Issues](https://github.com/lamco-admin/lamco-rdp-server/issues)
- **Commercial Licensing:** office@lamco.io
- **General Questions:** office@lamco.io
- **Website:** https://lamco.ai/products/lamco-rdp-server/

## 🔍 Verification

```bash
# Verify checksums
sha256sum -c SHA256SUMS-{VERSION}.txt
```

---

**Full changelog available in [CHANGELOG.md](https://github.com/lamco-admin/lamco-rdp-server/blob/main/CHANGELOG.md)**
```

---

### Step 4.3: Update GitHub Release v0.9.0

**Add SHA256SUMS and improve notes:**

```bash
cd ~/lamco-rdp-server/packaging

# Create SHA256SUMS if not done
sha256sum lamco-rdp-server-0.9.0-final.tar.xz lamco-rdp-server-0.9.0.flatpak > SHA256SUMS-0.9.0.txt

# Upload
gh release upload v0.9.0 SHA256SUMS-0.9.0.txt

# Edit release notes
gh release edit v0.9.0 --notes-file /path/to/improved-notes.md
```

Or update manually via GitHub web UI.

---

## Phase 5: Website Integration

### Step 5.1: Update Website Download Page

**Content to provide to website team:**

```html
<div class="download-section">
  <h2>Download lamco-rdp-server v0.9.0</h2>

  <div class="download-options">

    <div class="download-option recommended">
      <h3>Flatpak (Recommended)</h3>
      <p>Universal package, works on all Linux distributions.</p>

      <h4>Via Flathub:</h4>
      <pre><code>flatpak install flathub io.lamco.rdp-server
flatpak run io.lamco.rdp-server</code></pre>

      <h4>Direct Bundle:</h4>
      <pre><code>wget https://github.com/lamco-admin/lamco-rdp-server/releases/download/v0.9.0/lamco-rdp-server-0.9.0.flatpak
flatpak install lamco-rdp-server-0.9.0.flatpak</code></pre>

      <p><strong>Features:</strong> Software encoding (OpenH264), sandboxed security, automatic updates</p>
      <p><a href="https://github.com/lamco-admin/lamco-rdp-server/releases/download/v0.9.0/lamco-rdp-server-0.9.0.flatpak" class="download-button">Download Flatpak Bundle (6.5 MB)</a></p>
    </div>

    <div class="download-option">
      <h3>AppImage (Portable)</h3>
      <p>Single file, no installation required. Coming in v0.9.1.</p>
      <p class="status-upcoming">🚧 Available in next release (v0.9.1)</p>

      <p><em>AppImage provides portable single-file deployment, perfect for USB drives and quick testing.</em></p>
    </div>

    <div class="download-option">
      <h3>Native Packages</h3>
      <p>Best for production servers. Hardware acceleration support.</p>

      <h4>Fedora 40, 41, 42:</h4>
      <pre><code>sudo dnf install lamco-rdp-server</code></pre>

      <h4>RHEL 9 / AlmaLinux 9 / Rocky 9:</h4>
      <pre><code>sudo dnf install lamco-rdp-server</code></pre>

      <h4>openSUSE Tumbleweed / Leap 15.6:</h4>
      <pre><code>sudo zypper install lamco-rdp-server</code></pre>

      <h4>Debian 13 (Trixie):</h4>
      <pre><code>sudo apt install lamco-rdp-server</code></pre>

      <p><strong>Features:</strong> Hardware encoding (NVENC, VA-API), PAM authentication, zero-dialog session persistence</p>
      <p><strong>Note:</strong> Ubuntu 24.04 / Debian 12 not available (use Flatpak)</p>
    </div>

    <div class="download-option">
      <h3>Source Code</h3>
      <p>Build from source with full control.</p>

      <pre><code>wget https://github.com/lamco-admin/lamco-rdp-server/releases/download/v0.9.0/lamco-rdp-server-0.9.0-final.tar.xz
tar xf lamco-rdp-server-0.9.0-final.tar.xz
cd lamco-rdp-server-0.9.0
cargo build --release --features=h264,libei
sudo cp target/release/lamco-rdp-server /usr/local/bin/</code></pre>

      <p><strong>Requirements:</strong> Rust 1.77+, development packages (libssl-dev, libpipewire-dev)</p>
      <p><a href="https://github.com/lamco-admin/lamco-rdp-server/releases/download/v0.9.0/lamco-rdp-server-0.9.0-final.tar.xz" class="download-button secondary">Download Source (65 MB)</a></p>
    </div>

  </div>

  <div class="verification">
    <h3>Verify Downloads</h3>
    <pre><code># Download checksums
wget https://github.com/lamco-admin/lamco-rdp-server/releases/download/v0.9.0/SHA256SUMS-0.9.0.txt

# Verify
sha256sum -c SHA256SUMS-0.9.0.txt</code></pre>
  </div>

  <div class="all-releases">
    <p><a href="https://github.com/lamco-admin/lamco-rdp-server/releases">View All Releases on GitHub →</a></p>
  </div>
</div>
```

---

### Step 5.2: Add Download Badges to README

**File:** `~/lamco-rdp-server/README.md`

**Add at top (after title):**

```markdown
[![GitHub Release](https://img.shields.io/github/v/release/lamco-admin/lamco-rdp-server)](https://github.com/lamco-admin/lamco-rdp-server/releases/latest)
[![Flathub](https://img.shields.io/flathub/v/io.lamco.rdp-server)](https://flathub.org/apps/io.lamco.rdp-server)
[![License: BSL-1.1](https://img.shields.io/badge/License-BSL--1.1-blue.svg)](https://mariadb.com/bsl11/)
[![Downloads](https://img.shields.io/github/downloads/lamco-admin/lamco-rdp-server/total)](https://github.com/lamco-admin/lamco-rdp-server/releases)
```

---

## Phase 6: Per-Release Automation Checklist

### For v0.9.1 and Beyond

**File:** `~/lamco-admin/projects/lamco-rdp-server/RELEASE-CHECKLIST.md`

**Content:**
```markdown
# Release Checklist for lamco-rdp-server

## Pre-Release (1 hour)

### 1. Version Bump
- [ ] Update version in Cargo.toml
- [ ] Update CHANGELOG.md with new version and changes
- [ ] Commit: `git commit -m "Bump version to vX.Y.Z"`

### 2. Build and Test
- [ ] `cargo build --release --features=h264,libei`
- [ ] `cargo test`
- [ ] `./target/release/lamco-rdp-server --help` (verify runs)
- [ ] `./target/release/lamco-rdp-server --show-capabilities` (verify output)

### 3. Create Tag
- [ ] `git tag -a vX.Y.Z -m "Release vX.Y.Z"`
- [ ] `git push origin vX.Y.Z`

---

## Build Artifacts (30 minutes)

### 4. Create Source Tarball
```bash
cd ~/lamco-rdp-server
./packaging/create-vendor-tarball.sh
# Creates: packaging/lamco-rdp-server-X.Y.Z.tar.xz
```

### 5. Build Flatpak Bundle
```bash
cd ~/lamco-rdp-server

flatpak-builder --force-clean --repo=repo build-dir packaging/io.lamco.rdp-server.yml

flatpak build-bundle repo lamco-rdp-server-X.Y.Z.flatpak io.lamco.rdp-server \
  --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo

# Test install
flatpak install --user lamco-rdp-server-X.Y.Z.flatpak
flatpak run io.lamco.rdp-server --help
```

### 6. AppImage (Automated via GitHub Actions)
- [ ] GitHub Actions will build automatically when release is published
- [ ] Monitor: https://github.com/lamco-admin/lamco-rdp-server/actions
- [ ] Verify AppImage attached to release after build completes

### 7. Generate SHA256SUMS
```bash
cd ~/lamco-rdp-server/packaging

sha256sum \
  lamco-rdp-server-X.Y.Z.tar.xz \
  lamco-rdp-server-X.Y.Z.flatpak \
  > SHA256SUMS-X.Y.Z.txt

# AppImage hash will be added after GitHub Actions completes
```

---

## GitHub Release (15 minutes)

### 8. Create GitHub Release

**Via gh CLI:**
```bash
gh release create vX.Y.Z \
  packaging/lamco-rdp-server-X.Y.Z.tar.xz \
  packaging/lamco-rdp-server-X.Y.Z.flatpak \
  packaging/SHA256SUMS-X.Y.Z.txt \
  --title "lamco-rdp-server vX.Y.Z" \
  --notes-file ~/lamco-admin/projects/lamco-rdp-server/release-notes-X.Y.Z.md
```

**Via Web UI:**
1. Go to: https://github.com/lamco-admin/lamco-rdp-server/releases/new
2. Choose tag: vX.Y.Z
3. Release title: "lamco-rdp-server vX.Y.Z"
4. Description: Use template from GITHUB-RELEASE-NOTES-TEMPLATE.md
5. Attach files:
   - lamco-rdp-server-X.Y.Z.tar.xz
   - lamco-rdp-server-X.Y.Z.flatpak
   - SHA256SUMS-X.Y.Z.txt
6. Publish release

### 9. Wait for AppImage Build

- [ ] GitHub Actions AppImage workflow completes (~10-15 min)
- [ ] AppImage automatically attached to release
- [ ] Download and test: `./lamco-rdp-server-X.Y.Z-x86_64.AppImage --help`
- [ ] Update SHA256SUMS with AppImage hash

---

## Flathub Update (30 minutes)

### 10. Update Flathub Manifest

```bash
cd ~/lamco-admin/projects/lamco-rdp-server/flathub-submission/flathub

git checkout main
git pull upstream main
git checkout -b update-vX.Y.Z

cd io.lamco.rdp-server

# Update manifest version and hash
# Calculate hash: sha256sum ~/lamco-rdp-server/packaging/lamco-rdp-server-X.Y.Z.tar.xz

# Edit io.lamco.rdp-server.yml:
# - Update URL to vX.Y.Z
# - Update sha256 hash
```

### 11. Update MetaInfo Release Entry

Edit `io.lamco.rdp-server.metainfo.xml` and add new release:

```xml
<releases>
  <release version="X.Y.Z" date="YYYY-MM-DD">
    <description>
      <p>[Summary of changes]</p>
      <ul>
        <li>[Change 1]</li>
        <li>[Change 2]</li>
      </ul>
    </description>
    <url>https://github.com/lamco-admin/lamco-rdp-server/releases/tag/vX.Y.Z</url>
  </release>
  <!-- v0.9.0 entry below -->
</releases>
```

### 12. Submit Flathub PR

```bash
git add io.lamco.rdp-server/
git commit -m "Update io.lamco.rdp-server to vX.Y.Z

- [List key changes]
- Updated source to vX.Y.Z
- Updated MetaInfo with changelog"

git push origin update-vX.Y.Z

# Create PR on GitHub
```

---

## Verification (10 minutes)

### 13. Test All Installation Methods

- [ ] Flatpak: `flatpak install [bundle]` → `flatpak run io.lamco.rdp-server --help`
- [ ] AppImage: `chmod +x [file]` → `./[file] --help`
- [ ] Source: `tar xf [file]` → `cd` → `cargo build --release`
- [ ] Native (if available): `sudo dnf install lamco-rdp-server`

### 14. Verify Download Links

- [ ] GitHub Release page has all assets
- [ ] SHA256SUMS file present
- [ ] All download links work
- [ ] Flathub PR submitted (if applicable)

### 15. Update Website

- [ ] Notify website team of new version
- [ ] Provide download links
- [ ] Update version numbers on product page

---

## Total Time Per Release

**After automation complete:**
- Pre-release: 1 hour (version bump, testing, tag)
- Build artifacts: 30 minutes (tarball, Flatpak)
- GitHub Release: 15 minutes (automated upload)
- Flathub update: 30 minutes (manifest PR)
- Verification: 10 minutes (test installs)

**Total: ~2 hours 25 minutes per release**

**Automated portions:**
- AppImage: 0 minutes (GitHub Actions)
- Total savings: ~1-2 hours vs manual AppImage build

---

**END OF CHECKLIST**
```

---

## Summary: Channel Configuration Status

### What Needs Configuration NOW

**Channel 1: Flathub**
- [ ] Create MetaInfo XML (template provided)
- [ ] Create desktop file (template provided)
- [ ] Get icon from lamco.ai website
- [ ] Update Flatpak manifest to install MetaInfo/desktop/icons
- [ ] Test build with MetaInfo
- [ ] Fork Flathub repo
- [ ] Submit PR

**Blockers:** Need icon file from website

---

**Channel 2: AppImage**
- [ ] Create appimage.yml recipe (template provided)
- [ ] Create GitHub Actions workflow (template provided)
- [ ] Test local AppImage build
- [ ] Commit to repo
- [ ] Trigger test build

**Blockers:** None (can proceed immediately after icon)

---

**Channel 3: GitHub Releases**
- [ ] Create SHA256SUMS for v0.9.0 (command provided)
- [ ] Upload SHA256SUMS to v0.9.0 release
- [ ] Improve release notes (template provided)
- [ ] Add badges to README (template provided)

**Blockers:** None (can proceed immediately)

---

## Next Steps

**Immediate:**
1. Provide icon file from lamco.ai (or confirm location)
2. Create desktop file and MetaInfo XML
3. Configure AppImage automation
4. Enhance GitHub Release v0.9.0

**This Week:**
5. Submit Flathub PR
6. Test AppImage automation
7. Update website with download links

**All templates and commands provided above. Ready to execute.**

---

**END OF CONFIGURATION GUIDE**
