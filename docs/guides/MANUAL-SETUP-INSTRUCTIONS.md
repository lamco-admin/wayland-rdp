# Manual Setup Instructions - VM at 192.168.10.205

**VM:** Ubuntu 24.04 at 192.168.10.205
**Status:** Ready for manual setup
**Estimated Time:** 20-30 minutes

---

## Quick Setup (Copy-Paste Ready)

Open a terminal on the Ubuntu VM (192.168.10.205) and run these commands:

### Step 1: Install Dependencies

```bash
# Update package list
sudo apt update

# Install all required packages
sudo apt install -y \
    build-essential \
    pkg-config \
    git \
    curl \
    libpipewire-0.3-dev \
    libspa-0.2-dev \
    libssl-dev \
    libpam0g-dev \
    libdbus-1-dev \
    pipewire \
    wireplumber \
    xdg-desktop-portal \
    xdg-desktop-portal-gnome
```

### Step 2: Verify Services

```bash
# Enable and start PipeWire
systemctl --user enable pipewire pipewire-pulse wireplumber
systemctl --user start pipewire pipewire-pulse wireplumber

# Verify PipeWire is running
systemctl --user status pipewire | head -5
pipewire --version

# Verify Portal is running
systemctl --user status xdg-desktop-portal | head -5

# Check D-Bus
busctl --user list | grep portal
```

**You should see:** PipeWire and Portal both "active (running)"

### Step 3: Install Rust

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts (just press Enter for defaults)
# Then load Rust into current shell:
source "$HOME/.cargo/env"

# Verify
rustc --version
cargo --version
```

### Step 4: Clone and Build

```bash
# Clone repository
cd ~
git clone https://github.com/lamco-admin/wayland-rdp.git
cd wayland-rdp

# Build (this takes 5-10 minutes first time)
cargo build --release

# You should see:
#   Compiling wrd-server v0.1.0
#   Finished `release` profile in 1m 07s
```

### Step 5: Generate Certificates

```bash
# Create certs directory
cd ~/wayland-rdp
mkdir -p certs

# Generate self-signed certificate
openssl req -x509 -newkey rsa:4096 -nodes \
    -keyout certs/key.pem \
    -out certs/cert.pem \
    -days 365 \
    -subj "/CN=wrd-test/O=Testing/C=US"

# Verify files created
ls -lh certs/
# Should show cert.pem and key.pem
```

### Step 6: Create Configuration

```bash
# Create config file
cd ~/wayland-rdp
cat > config.toml <<'EOF'
[server]
listen_addr = "0.0.0.0:3389"
max_connections = 5
session_timeout = 0
use_portals = true

[security]
cert_path = "certs/cert.pem"
key_path = "certs/key.pem"
require_tls = true

[video]
max_fps = 30
enable_damage_tracking = true
EOF

# Verify config
cat config.toml
```

### Step 7: Configure Firewall (if needed)

```bash
# Check if firewall is active
sudo ufw status

# If active, allow RDP port
sudo ufw allow 3389/tcp

# Verify
sudo ufw status | grep 3389
```

### Step 8: Start the Server!

```bash
cd ~/wayland-rdp
./target/release/wrd-server -c config.toml -vv
```

**You should see:**

```
╔════════════════════════════════════════════════════════════╗
║          WRD-Server Startup Diagnostics                   ║
╚════════════════════════════════════════════════════════════╝
=== System Information ===
  OS: Ubuntu 24.04
  Kernel: 6.8.0-...
  CPUs: 4
  Memory: 8192 MB
=== Environment ===
  Compositor: GNOME
  Portal Backend: GNOME
  PipeWire: compiled 1.x.x
╚════════════════════════════════════════════════════════════╝

INFO Initializing WRD Server
INFO Setting up Portal connection
INFO Creating RemoteDesktop portal session
```

**CRITICAL:** A permission dialog will appear on the Ubuntu desktop!

---

## Step 9: Grant Permission

**On the Ubuntu VM screen, you'll see a dialog:**

Title: **"wrd-server wants to share your screen"** (or similar)

**Actions:**
- **Click "Share" or "Allow"** ✅
- Do NOT click "Deny" ❌

**After granting permission:**

```
INFO Portal session started with 1 streams, PipeWire FD: 5
INFO PipeWire thread started successfully
INFO PipeWire Core connected successfully
INFO Stream 42 is now streaming
INFO Display handler created: 1920x1080, 1 streams

╔════════════════════════════════════════════════════════════╗
║          WRD-Server is Starting                            ║
╚════════════════════════════════════════════════════════════╝
  Listen Address: 0.0.0.0:3389
  TLS: Enabled (rustls 0.23)
  Codec: RemoteFX
  Max Connections: 5
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Server is ready and listening for RDP connections
Waiting for clients to connect...
```

**Server is running!**

---

## Step 10: Connect from Windows

### On Your Windows Machine:

1. **Open Remote Desktop Connection**
   - Press Windows key, type "Remote Desktop"
   - Or run: `mstsc.exe`

2. **Enter:**
   ```
   Computer: 192.168.10.205:3389
   ```

3. **Click "Connect"**

4. **Certificate Warning:**
   - You'll see: "The identity of the remote computer cannot be verified"
   - **This is normal** (self-signed certificate)
   - Click: **"Yes"** to continue

5. **You should see the Ubuntu desktop in RDP!** 🎉

---

## Testing Checklist

Once connected:

- [ ] Can see Ubuntu desktop
- [ ] Mouse moves smoothly
- [ ] Keyboard works (open terminal, type)
- [ ] Can click applications
- [ ] Video is smooth (open YouTube video)
- [ ] Let it run for 5 minutes
- [ ] Monitor server logs for errors
- [ ] Note performance (subjective)

---

## Troubleshooting

### If Server Fails to Start

Check logs for specific error - the user-friendly error handler will tell you exactly what's wrong and how to fix it.

### If Permission Dialog Doesn't Appear

```bash
# Restart Portal
systemctl --user restart xdg-desktop-portal

# Run server again
./target/release/wrd-server -c config.toml -vv
```

### If Can't Connect from Windows

```bash
# On VM, check server is listening
ss -tlnp | grep 3389
# Should show: LISTEN on 0.0.0.0:3389

# Check firewall
sudo ufw status
# Should show: 3389/tcp ALLOW

# Ping VM from Windows
ping 192.168.10.205
# Should respond
```

### If Black Screen

```bash
# Check PipeWire streams
pw-cli ls Node | grep wrd

# Check server logs for frames
# Should see "Processing buffer" or "Got frame" messages
```

---

## Alternative: Copy Files Manually

If git clone doesn't work or you want to use local code:

### From Your Development Machine

```bash
# Create tarball
cd /home/greg/wayland/wrd-server-specs
tar czf wrd-server.tar.gz src/ Cargo.toml Cargo.lock scripts/ config.toml.example

# Copy to VM (use USB, network share, or scp when SSH works)
# Then on VM:
tar xzf wrd-server.tar.gz
cd wrd-server-specs
cargo build --release
```

---

## Quick Reference

**Setup script location:** `/tmp/wrd-setup-ubuntu.sh` (on dev machine)
**Config example:** `/home/greg/wayland/wrd-server-specs/config.toml.example`
**Full guide:** `/home/greg/wayland/wrd-server-specs/QUICK-START-192.168.10.205.md`

---

## Expected Outcome

### Success Looks Like:

✅ Server starts with diagnostics
✅ Permission dialog appears and you grant it
✅ Server shows "ready and listening"
✅ Windows RDP client connects
✅ Ubuntu desktop appears in RDP window
✅ Input and mouse work
✅ Can use applications normally

### Then:

📊 Collect performance data
🐛 Note any issues
📝 Report findings
🔧 We fix any problems
🚀 Optimize and polish

---

**The VM is ready, the code is ready, the infrastructure is ready.**

**Just need to run the setup on 192.168.10.205 and test!**

**Good luck with the first test! 🎉**

