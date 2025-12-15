# Testing Environment Recommendations

**Date:** 2025-11-18
**Purpose:** Optimal VM setup for WRD-Server integration testing
**Priority:** Critical for Phase 1 validation

---

## Executive Summary

**Recommended Primary Test Environment:**
- **OS:** Ubuntu 24.04 LTS or Fedora 40
- **Desktop:** GNOME 46+ or KDE Plasma 6+
- **Reason:** Best Portal support, most complete PipeWire integration, actively maintained

**Recommended Test Matrix:**
- Minimum 3 VMs covering major compositors
- All should have GPU passthrough or virtual GPU
- All need PipeWire 0.3.77+

---

## Tier 1: Primary Testing (MUST TEST)

### Option A: Ubuntu 24.04 LTS + GNOME 46 (RECOMMENDED)

**Why This First:**
- ✅ Most stable Wayland implementation
- ✅ Best xdg-desktop-portal integration
- ✅ GNOME Mutter is reference Wayland compositor
- ✅ PipeWire 1.0.0+ included
- ✅ Long-term support (until 2029)
- ✅ Largest user base for bug discovery
- ✅ Best documented Portal implementation

**VM Specs:**
- CPU: 4 cores minimum
- RAM: 8GB minimum (4GB for GNOME + 2GB for RDP server)
- Disk: 25GB
- GPU: VirtIO-GPU with 3D acceleration OR GPU passthrough

**Setup:**
```bash
# Install Ubuntu 24.04 Desktop
# During install: Choose "Minimal installation" + "Install third-party drivers"

# After install:
sudo apt update && sudo apt upgrade -y

# Install development dependencies
sudo apt install -y \
    build-essential \
    pkg-config \
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libssl-dev \
    libpam0g-dev \
    rustc cargo \
    pipewire \
    wireplumber \
    xdg-desktop-portal \
    xdg-desktop-portal-gnome

# Verify versions
pipewire --version  # Should be 1.0+
gnome-shell --version  # Should be 46+

# Enable PipeWire
systemctl --user enable pipewire pipewire-pulse wireplumber
systemctl --user start pipewire pipewire-pulse wireplumber

# Verify Portal
busctl --user list | grep portal
# Should show org.freedesktop.portal.Desktop
```

**Why Ubuntu 24.04:**
- GNOME 46 has mature RemoteDesktop portal
- PipeWire 1.0.0 is fully stabilized
- LTS means production representative
- Easiest to replicate issues

---

### Option B: Fedora 40 Workstation + GNOME 46

**Why Test This:**
- ✅ Bleeding-edge Wayland stack
- ✅ PipeWire developers use Fedora
- ✅ Latest GNOME features
- ✅ Best for catching future issues
- ✅ SELinux testing (security)

**VM Specs:** Same as Ubuntu

**Setup:**
```bash
# Install Fedora 40 Workstation
# During install: Choose "GNOME" desktop

sudo dnf update -y

# Install development dependencies
sudo dnf install -y \
    gcc \
    pkg-config \
    pipewire-devel \
    openssl-devel \
    pam-devel \
    rust cargo \
    pipewire \
    wireplumber \
    xdg-desktop-portal \
    xdg-desktop-portal-gnome

# Verify
pipewire --version  # Should be 1.2+
gnome-shell --version  # Should be 46+
```

**Why Fedora:**
- Newest Wayland tech
- Finds issues before they hit LTS
- PipeWire 1.2+ has latest features

---

### Option C: Arch Linux + KDE Plasma 6

**Why Test This:**
- ✅ Rolling release (catches breakage early)
- ✅ KDE Plasma has different Portal implementation
- ✅ Tests non-GNOME stack
- ✅ kwin_wayland is major compositor
- ✅ Excellent multi-monitor support in KDE

**VM Specs:** Same as above

**Setup:**
```bash
# Install Arch with archinstall, choose KDE Plasma

sudo pacman -Syu
sudo pacman -S --needed \
    base-devel \
    rust \
    pipewire \
    wireplumber \
    xdg-desktop-portal \
    xdg-desktop-portal-kde \
    lib32-pipewire

# Verify
pipewire --version
plasmashell --version  # Should be 6.0+
```

**Why KDE:**
- Different compositor (kwin vs mutter)
- Different Portal backend
- Tests code portability
- Good multi-monitor

---

## Tier 2: Secondary Testing (SHOULD TEST)

### Option D: Ubuntu 24.04 + KDE Plasma 6 (Kubuntu)

**Why:**
- Same Ubuntu base as Option A
- But KDE desktop
- Tests Portal implementation differences
- LTS stability with KDE features

**Setup:** Same as Ubuntu but install `kubuntu-desktop`

---

### Option E: Debian 13 (Trixie) + GNOME

**Why:**
- Ultra-stable
- Tests conservative versions
- Production server environment
- Good for finding minimum version issues

---

## Tier 3: Advanced Testing (NICE TO HAVE)

### Option F: Sway (wlroots compositor)

**Why:**
- Tiling window manager
- wlroots is used by many compositors
- Tests minimalist environment
- Different Portal implementation

**Setup:**
```bash
# On Arch
sudo pacman -S sway xdg-desktop-portal-wlr

# On Ubuntu
sudo apt install sway xdg-desktop-portal-wlr
```

**Challenges:**
- Less complete Portal support
- May need manual configuration
- Good for edge case testing

---

## VM Configuration Recommendations

### Hypervisor Choice

**Best: KVM/QEMU with virt-manager**
- Native Linux performance
- GPU passthrough support
- VirtIO-GPU 3D acceleration
- Industry standard

**Alternative: VirtualBox**
- Easier setup
- Guest additions for GL
- Cross-platform host

**Avoid: Docker/containers**
- Can't run Wayland properly
- No GPU access
- Portal won't work

### Critical VM Settings

#### GPU Configuration (CRITICAL)

**Option 1: VirtIO-GPU with 3D (Recommended)**
```xml
<video>
  <model type='virtio' heads='2' primary='yes'>
    <acceleration accel3d='yes'/>
  </model>
</video>
```

**Why:** PipeWire needs GL context for DMA-BUF

**Option 2: GPU Passthrough (Best Performance)**
- Pass through dedicated GPU to VM
- Best for production testing
- Requires IOMMU support

**Option 3: QXL (Minimum)**
- Works but no 3D
- PipeWire falls back to memory buffers
- Acceptable for basic testing

#### Memory
- **Minimum:** 8GB
- **Recommended:** 16GB
- **Why:** GNOME + PipeWire + RDP server + dev tools

#### CPUs
- **Minimum:** 4 vCPUs
- **Recommended:** 6-8 vCPUs
- **Why:** Encoding can be CPU-intensive

#### Disk
- **Minimum:** 25GB
- **Recommended:** 50GB
- **Why:** OS + dev tools + logs

#### Network
- **Mode:** Bridged (for RDP testing from host/other VMs)
- **Alt:** NAT with port forward 3389

---

## Recommended Test Matrix

### Minimal Testing (3 VMs)

| VM | OS | Desktop | Purpose | Priority |
|----|----|---------| --------|----------|
| **VM1** | Ubuntu 24.04 | GNOME 46 | Primary validation | MUST |
| **VM2** | Fedora 40 | GNOME 46 | Latest stack testing | MUST |
| **VM3** | Kubuntu 24.04 | KDE Plasma 6 | KDE Portal testing | SHOULD |

**Total Resources:**
- 24GB RAM
- 12 vCPUs
- 75GB disk

### Comprehensive Testing (5 VMs)

Add to above:

| VM | OS | Desktop | Purpose | Priority |
|----|----|---------| --------|----------|
| **VM4** | Arch | KDE Plasma 6 | Rolling release | NICE |
| **VM5** | Arch | Sway | wlroots testing | NICE |

---

## RDP Client Testing

### Windows Clients (Primary)

**Option 1: Windows 11 VM (Recommended)**
- Latest mstsc.exe
- Best for user testing
- Tests real-world client

**Option 2: Windows 10 VM**
- Still widely deployed
- Slightly older mstsc
- Good compatibility test

**Setup:**
```
1. Install Windows 10/11
2. Enable RDP client (built-in)
3. Add server VM to hosts file
4. Connect via mstsc.exe
```

### Linux Clients (Development)

**FreeRDP** (Command-line testing)
```bash
# Install
sudo apt install freerdp2-x11  # Ubuntu
sudo dnf install freerdp      # Fedora

# Test connection
xfreerdp /v:192.168.122.100:3389 /u:testuser /cert:ignore /gfx:rfx
```

**Remmina** (GUI testing)
```bash
sudo apt install remmina remmina-plugin-rdp
# Use GUI to configure connection
```

---

## Multi-Monitor Testing Setup

### QEMU/virt-manager Multi-Monitor

**Add multiple displays:**
```xml
<video>
  <model type='virtio' heads='2'>
    <acceleration accel3d='yes'/>
  </model>
</video>
```

**In GNOME Settings:**
- Settings → Displays
- Arrange monitors
- Test different layouts

**Test Scenarios:**
1. Single monitor (1920x1080)
2. Dual monitor horizontal (1920x1080 + 1920x1080)
3. Dual monitor different res (1920x1080 + 2560x1440)
4. Triple monitor
5. Portrait + Landscape mix

---

## Specific Feature Testing

### Portal Permissions

**First Run:**
```bash
# Run server
./target/release/wrd-server -c config.toml -vv

# Expected: Permission dialog appears
# Action: Click "Allow" / "Share"
# Verify: Logs show "Portal session started"
```

**Test:**
- Permission grant
- Permission deny (server should handle gracefully)
- Permission persistence

### PipeWire Capture

**Verify:**
```bash
# Check PipeWire is running
systemctl --user status pipewire

# Watch PipeWire nodes
pw-cli ls Node | grep wrd

# Monitor streams
pw-top
# Should show wrd-capture-* streams
```

**Test:**
- Frame capture starts
- Frame rate stable
- No buffer overruns
- Clean shutdown

### Input Injection

**Test:**
- Keyboard: Type in remote app
- Mouse: Click buttons, move cursor
- Scroll: Wheel in apps
- Multi-monitor: Move between screens
- Special keys: Alt-Tab, Ctrl-C, etc.

### Clipboard (Basic Structure Ready)

**Test when fully implemented:**
- Copy text RDP → Wayland
- Copy text Wayland → RDP
- Copy image
- Large data (>1MB)

---

## Performance Testing

### Monitoring Tools

**Install on test VM:**
```bash
sudo apt install -y \
    htop \
    iotop \
    nethogs \
    iftop \
    perf \
    linux-tools-common

# Wayland specific
sudo apt install weston-examples  # weston-info shows compositor info
```

**Monitor During Session:**
```bash
# Terminal 1: Server logs
./wrd-server -vvv

# Terminal 2: CPU/Memory
htop

# Terminal 3: Network
sudo iftop -i eth0

# Terminal 4: PipeWire status
pw-top
```

### Metrics to Capture

1. **Latency:**
   - Input: Measure keystroke to screen update
   - Video: Measure screen change to RDP update
   - Target: <100ms end-to-end

2. **FPS:**
   - Monitor RDP frame rate
   - Target: 30fps stable, 60fps ideal

3. **CPU:**
   - Idle: <5%
   - 1080p@30fps: <25%
   - Target from spec

4. **Memory:**
   - Steady state: <300MB
   - Peak: <500MB
   - Check for leaks (run for hours)

5. **Network:**
   - 1080p@30fps RemoteFX: 10-30 Mbps
   - Check bandwidth usage

---

## Test Scenario Checklist

### Basic Functionality

- [ ] Server starts without errors
- [ ] Portal permission dialog appears
- [ ] User grants permission
- [ ] PipeWire streams initialize
- [ ] RDP client connects (Windows mstsc)
- [ ] Desktop displays on client
- [ ] Keyboard input works
- [ ] Mouse movement works
- [ ] Mouse clicks work
- [ ] Scroll wheel works
- [ ] Can type in applications
- [ ] Can navigate menus
- [ ] Video playback works (YouTube test)
- [ ] Session disconnects cleanly
- [ ] Session reconnects successfully

### Multi-Monitor

- [ ] Dual monitors detected
- [ ] Both screens visible in RDP
- [ ] Mouse moves between monitors
- [ ] Window drag across monitors
- [ ] Correct monitor layout
- [ ] Hotplug: Disconnect monitor
- [ ] Hotplug: Reconnect monitor
- [ ] Resolution change

### Stress Testing

- [ ] Run for 1 hour continuous
- [ ] Run for 8 hours (overnight)
- [ ] Multiple connect/disconnect cycles
- [ ] High motion video (gaming video)
- [ ] Rapid typing
- [ ] Rapid mouse movement
- [ ] Check memory leaks
- [ ] Check CPU spikes

### Error Scenarios

- [ ] Kill PipeWire daemon (recovery?)
- [ ] Kill Portal (error handling?)
- [ ] Revoke permissions mid-session
- [ ] Network interruption
- [ ] Kill compositor
- [ ] Out of memory condition

---

## Quick Start: Recommended First Test VM

### Ubuntu 24.04 + GNOME 46

**VM Creation (virt-manager):**
```bash
# Download Ubuntu 24.04 Desktop ISO
wget https://releases.ubuntu.com/24.04/ubuntu-24.04-desktop-amd64.iso

# Create VM (assuming KVM/virt-manager)
virt-install \
  --name wrd-test-gnome \
  --ram 8192 \
  --vcpus 4 \
  --disk size=50 \
  --os-variant ubuntu24.04 \
  --cdrom ubuntu-24.04-desktop-amd64.iso \
  --graphics spice,gl.enable=yes \
  --video virtio \
  --network bridge=virbr0

# Or use virt-manager GUI
```

**Post-Install:**
```bash
# Install build deps
sudo apt install -y \
    build-essential \
    pkg-config \
    git \
    curl \
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libssl-dev \
    libpam0g-dev \
    libdbus-1-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone your repo
git clone https://github.com/lamco-admin/wayland-rdp.git
cd wayland-rdp

# Build
cargo build --release

# Generate test certificates
mkdir -p certs
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout certs/key.pem \
  -out certs/cert.pem \
  -days 365 \
  -subj "/CN=wrd-test"

# Create config
cat > config.toml <<EOF
[server]
listen_addr = "0.0.0.0:3389"
max_connections = 5

[security]
cert_path = "certs/cert.pem"
key_path = "certs/key.pem"
require_tls = true

[video]
max_fps = 30
enable_damage_tracking = true
EOF

# Run server
./target/release/wrd-server -c config.toml -vvv
```

**Expected First Run:**
1. Portal permission dialog appears
2. Click "Allow" / "Share Screen"
3. Server logs: "Portal session started"
4. Server logs: "PipeWire streams initialized"
5. Server logs: "Listening for connections on 0.0.0.0:3389"

**Connect from Windows:**
```
mstsc.exe
Computer: <VM_IP>:3389
Username: <ubuntu_user>
# Accept certificate warning
# Should see Ubuntu desktop
```

---

## Alternative: Fedora 40 + KDE Plasma 6

**Why This Second:**
- Different compositor (kwin vs mutter)
- Different Portal backend
- Tests compatibility
- KDE has excellent multi-monitor

**VM Creation:**
```bash
# Download Fedora 40 KDE Spin
wget https://download.fedoraproject.org/pub/fedora/linux/releases/40/Spins/x86_64/iso/Fedora-KDE-Live-x86_64-40.iso

virt-install \
  --name wrd-test-kde \
  --ram 8192 \
  --vcpus 4 \
  --disk size=50 \
  --os-variant fedora40 \
  --cdrom Fedora-KDE-Live-x86_64-40.iso \
  --graphics spice,gl.enable=yes \
  --video virtio \
  --network bridge=virbr0
```

**Setup:** Similar to above, use `dnf` instead of `apt`

---

## File Copying Question - ANSWERED

### Current Status

**File copying IS supported in the architecture!**

**Where it's implemented:**
1. **Clipboard Module** has file transfer infrastructure:
   - `src/clipboard/formats.rs` - CF_HDROP format (Windows file list)
   - `src/clipboard/formats.rs` - text/uri-list MIME type (Linux)
   - `src/clipboard/transfer.rs` - Chunked transfer engine (600 LOC!)
   - `src/clipboard/manager.rs` - Transfer coordination

2. **IronRDP Backend** has file transfer hooks:
   - `on_file_contents_request()` - When RDP requests file
   - `on_file_contents_response()` - When RDP provides file
   - FileContentsRequest/Response PDU types

3. **Protocol Support:**
   - IronRDP's ironrdp-cliprdr supports full file transfer
   - RDP CLIPRDR protocol includes file streaming
   - Windows clients support it natively

### How It Works

**Copy File from RDP Client to Wayland:**
```
1. User copies file in Windows (Ctrl+C on file)
2. RDP client sends format list with CF_HDROP
3. on_remote_copy() receives format list
4. User pastes in Linux app (Ctrl+V)
5. Linux app requests data
6. on_format_data_request() triggered
7. Request CF_HDROP from RDP client
8. Client sends file list metadata
9. Request file contents via FileContentsRequest
10. Client sends actual file data
11. Write to /tmp/wrd-clipboard/<filename>
12. Provide file URI to Linux app (text/uri-list)
```

**Copy File from Wayland to RDP Client:**
```
1. User copies file in Linux (Ctrl+C on file)
2. Portal provides text/uri-list MIME type
3. Read file from URI
4. Convert to CF_HDROP format
5. Send format list to RDP client
6. Client requests data
7. Send file list metadata
8. Client requests file contents
9. Send file data in chunks
10. Client writes file to clipboard
11. User pastes in Windows
```

### Current Implementation Status

**Structure:** ✅ Complete (transfer engine, formats, PDU handlers)
**Integration:** ⚠️ Partial (backend wired, full logic deferred)

**What's Ready:**
- Transfer engine with chunking (600 LOC)
- Format mapping (CF_HDROP ↔ text/uri-list)
- Event handlers in backend
- Error handling

**What Needs Completion (2-3 hours):**
- Wire file request/response to actual file I/O
- Implement temp directory management
- Add progress tracking
- Test with large files (100MB+)

**Recommendation:** Get basic RDP working first, then come back to file transfer

---

## Testing Priority Recommendations

### Phase 1: Basic Validation (Week 1)

**VM:** Ubuntu 24.04 + GNOME 46

**Tests:**
1. Server starts
2. Portal permission works
3. RDP client connects
4. Desktop visible
5. Keyboard works
6. Mouse works
7. Can use applications
8. Disconnect/reconnect
9. Check logs for errors
10. Measure basic performance

**Goal:** Prove core functionality

### Phase 2: Multi-Monitor (Week 2)

**VM:** Ubuntu 24.04 or Fedora 40 (dual display)

**Tests:**
1. Two monitors detected
2. Layout correct
3. Mouse crosses boundary
4. Windows drag across
5. Resolution changes
6. Hotplug add/remove

**Goal:** Validate P1-09 implementation

### Phase 3: Compatibility (Week 3)

**VMs:** All tier 1 + tier 2

**Tests:**
1. Same tests on each compositor
2. Document differences
3. Fix compositor-specific issues
4. Performance comparison

**Goal:** Multi-compositor support

### Phase 4: Stress & Performance (Week 4)

**VM:** Ubuntu 24.04 (known good)

**Tests:**
1. 8-hour session
2. High motion video
3. Memory leak check
4. CPU profiling
5. Network bandwidth
6. Latency measurement

**Goal:** Production readiness validation

---

## File Copying - Full Implementation

**Estimated Effort:** 2-3 hours

**Implementation Steps:**

1. **In ironrdp_backend.rs:**
```rust
fn on_format_data_request(&mut self, request: FormatDataRequest) {
    if request.format == ClipboardFormatId::CF_HDROP {
        // File list requested
        let file_list = self.get_file_list_from_portal();
        // Send FileListResponse
    } else {
        // Regular data
    }
}

fn on_file_contents_request(&mut self, request: FileContentsRequest) {
    let file_path = self.get_file_path(request.list_index());
    let contents = std::fs::read(file_path)?;

    // Send via FileContentsResponse
    // Handle streaming for large files
}
```

2. **Add Portal file operations:**
```rust
// In portal/clipboard.rs or portal/remote_desktop.rs
pub async fn read_clipboard_file(&self, uri: &str) -> Result<Vec<u8>> {
    // Read file from URI
    let path = uri.strip_prefix("file://").unwrap();
    tokio::fs::read(path).await
}

pub async fn write_clipboard_file(&self, filename: &str, data: Vec<u8>) -> Result<String> {
    // Write to temp dir
    let path = format!("/tmp/wrd-clipboard/{}", filename);
    tokio::fs::write(&path, data).await?;
    Ok(format!("file://{}", path))
}
```

3. **Test with:**
- Copy text file Windows → Linux
- Copy image file Windows → Linux
- Copy file Linux → Windows
- Copy multiple files
- Large file (100MB+)

---

## Debugging Tools

### On Test VM

**PipeWire debugging:**
```bash
# Watch PipeWire graph
pw-dot | dot -Tpng > pipewire-graph.png

# Monitor nodes
watch -n 1 'pw-cli ls Node'

# Dump stream info
pw-dump
```

**Portal debugging:**
```bash
# Watch D-Bus
dbus-monitor --session "interface='org.freedesktop.portal.ScreenCast'"

# Check portal backends
ls /usr/libexec/*portal*
```

**RDP debugging:**
```bash
# Server logs (verbose)
RUST_LOG=trace ./wrd-server

# Network capture
sudo tcpdump -i any port 3389 -w rdp-capture.pcap

# Analyze later
wireshark rdp-capture.pcap
```

---

## Recommended Testing Flow

### Day 1: Setup
1. Create Ubuntu 24.04 VM
2. Install dependencies
3. Build wrd-server
4. Generate certificates
5. Run server, verify startup

### Day 2: First Connection
1. Connect from Windows client
2. Grant Portal permission
3. See desktop in RDP
4. Test basic input
5. Document any issues

### Day 3: Stability
1. Run 1-hour session
2. Use applications
3. Check logs
4. Check resource usage
5. Fix any crashes

### Day 4: Multi-Monitor
1. Add second display
2. Configure layout
3. Test cross-monitor behavior
4. Test hotplug

### Day 5: Performance
1. Measure FPS
2. Measure latency
3. Measure bandwidth
4. Compare to targets
5. Profile if needed

---

## Common Issues & Solutions

### Issue: Portal permission dialog doesn't appear

**Cause:** Portal not running or not installed
**Fix:**
```bash
systemctl --user status xdg-desktop-portal
sudo apt install xdg-desktop-portal-gnome
systemctl --user restart xdg-desktop-portal
```

### Issue: PipeWire connection fails

**Cause:** PipeWire not running or wrong version
**Fix:**
```bash
pipewire --version  # Must be 0.3.77+
systemctl --user restart pipewire wireplumber
```

### Issue: Black screen in RDP

**Possible causes:**
1. PipeWire not capturing
2. Format negotiation failed
3. Bitmap conversion error

**Debug:**
```bash
# Check logs for "Frame extraction" messages
# Should see frames being processed
grep "Processing buffer" server.log
```

### Issue: Input doesn't work

**Possible causes:**
1. Portal RemoteDesktop not enabled
2. Permission not granted
3. Input injection failing

**Debug:**
```bash
# Check Portal capabilities
busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop
# Should show RemoteDesktop interface
```

---

## Summary Recommendations

### Best First Test Environment

**Winner: Ubuntu 24.04 LTS + GNOME 46**

**Why:**
1. Most stable Wayland stack
2. Best Portal implementation
3. Easiest to set up
4. Most documentation
5. Largest community
6. Representative of production

**VM Specs:**
- 8GB RAM
- 4 vCPUs
- 50GB disk
- VirtIO-GPU with 3D
- Bridged network

**Timeline:**
- Setup: 1-2 hours
- First connection: 2-4 hours
- Basic testing: 1-2 days
- Multi-monitor: 1-2 days
- Performance validation: 1-2 days

**Total:** ~1 week for comprehensive validation

---

### File Copying Answer

**Short answer:** YES, file copying is supported in the architecture!

**Current status:**
- Protocol support: ✅ Full (IronRDP has it)
- Transfer engine: ✅ Complete (600 LOC)
- Format mapping: ✅ Present (CF_HDROP ↔ text/uri-list)
- Event handlers: ✅ Wired up
- File I/O: ⏳ Needs 2-3 hours to wire actual file operations

**Recommendation:**
1. Get basic RDP working (video + input)
2. Validate core functionality
3. THEN implement file transfer complete logic
4. Test with real files

**Priority:** Medium (after basic RDP validation)

---

### Testing Priority Order

1. ✅ **Build verification** (Done)
2. 🔄 **Basic RDP connection** (Next - 1 day)
3. 🔄 **Input validation** (Next - 1 day)
4. 🔄 **Multi-monitor** (Week 2)
5. 🔄 **Performance** (Week 2)
6. ⏳ **File copying** (Week 3)
7. ⏳ **Stress testing** (Week 3)

---

**RECOMMENDATION: Start with Ubuntu 24.04 + GNOME 46 VM for first integration test.**

