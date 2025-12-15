# Quick Start - Test VM at 192.168.10.205

**VM Ready:** Ubuntu 24.04 at 192.168.10.205
**Status:** Ready for deployment and testing
**Est. Time:** 15-30 minutes to first RDP connection

---

## Step 1: SSH to VM and Run Setup

```bash
# From your host machine
ssh user@192.168.10.205

# Once on the VM, run automated setup:
bash <(curl -s https://raw.githubusercontent.com/lamco-admin/wayland-rdp/main/scripts/setup-ubuntu.sh)

# OR manually copy the script:
# Copy scripts/setup-ubuntu.sh to the VM and run it
```

**What the script does:**
1. Installs all dependencies (PipeWire, Portal, build tools)
2. Verifies Wayland session
3. Installs Rust
4. Clones repository
5. Builds wrd-server (release mode)
6. Generates TLS certificates
7. Creates config file
8. Configures firewall

**Time:** ~10-15 minutes (mostly building)

---

## Step 2: Start the Server

```bash
cd ~/wayland-rdp
./target/release/wrd-server -c config.toml -vv
```

**Expected Output:**
```
2025-11-18 10:30:45 INFO Starting WRD-Server v0.1.0

╔════════════════════════════════════════════════════════════╗
║          WRD-Server Startup Diagnostics                   ║
╚════════════════════════════════════════════════════════════╝
=== System Information ===
  OS: Ubuntu 24.04
  Kernel: 6.8.0-45-generic
  Hostname: ubuntu
  CPUs: 4
  Memory: 8192 MB
=== Environment ===
  Compositor: GNOME
  Portal Backend: GNOME
  PipeWire: compiled 1.2.3
╚════════════════════════════════════════════════════════════╝

INFO Initializing WRD Server
INFO Setting up Portal connection
INFO Creating RemoteDesktop portal session
```

**At this point:** A permission dialog will appear on the Ubuntu desktop

---

## Step 3: Grant Portal Permission

**CRITICAL:** A dialog will appear asking for screen sharing permission

**Dialog says:** "wrd-server wants to: View your screen / Control input devices"

**Action:** Click **"Allow"** or **"Share"**

**After granting permission:**
```
INFO Portal session started with 1 streams, PipeWire FD: 5
INFO PipeWire thread started successfully
INFO PipeWire Core connected successfully
INFO Stream 42 is now streaming
INFO Display handler created: 1920x1080, 1 streams
INFO TLS 1.3 configuration created successfully
INFO Clipboard manager initialized
INFO WRD Server initialized successfully

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

**Server is now running!**

---

## Step 4: Connect from Windows RDP Client

### On Your Windows Machine

1. **Open Remote Desktop Connection** (mstsc.exe)
   - Press Windows key
   - Type "Remote Desktop Connection"
   - Or run: `mstsc.exe`

2. **Enter Connection Details:**
   ```
   Computer: 192.168.10.205:3389
   ```

3. **Click "Connect"**

4. **Certificate Warning:**
   - You'll see: "The identity of the remote computer cannot be verified"
   - **This is expected** (self-signed certificate)
   - Click: **"Yes"** or **"Connect anyway"**

5. **You should see the Ubuntu desktop!**

---

## Step 5: Test Basic Functionality

### On the RDP Session

**Test Keyboard:**
- Open terminal (Ctrl+Alt+T)
- Type: `echo "Hello from RDP"`
- Should work normally

**Test Mouse:**
- Move mouse around
- Click on applications
- Right-click for context menu
- Scroll with mouse wheel

**Test Applications:**
- Open Firefox
- Browse to a website
- Play a YouTube video (tests video streaming)
- Type in text editor

**Let it run for 5-10 minutes** to check stability

---

## Monitoring (On VM in Another SSH Session)

While the server is running, in another terminal:

```bash
# Watch PipeWire streams
watch -n 1 'pw-cli ls Node | grep wrd'

# Monitor resource usage
htop

# Monitor network
sudo iftop -i eth0

# Check server logs
# (logs are in the terminal where you ran wrd-server)
```

---

## Troubleshooting

### Issue: Permission Dialog Doesn't Appear

**Solution:**
```bash
# Check Portal is running
systemctl --user status xdg-desktop-portal

# Restart Portal
systemctl --user restart xdg-desktop-portal

# Run server again
./target/release/wrd-server -c config.toml -vv
```

### Issue: "Connection Failed" from Windows

**Check server is listening:**
```bash
ss -tlnp | grep 3389
# Should show: LISTEN on 0.0.0.0:3389
```

**Check firewall:**
```bash
sudo ufw status
# Should show: 3389/tcp ALLOW
```

**Test locally first:**
```bash
# On the VM
sudo apt install freerdp2-x11
xfreerdp /v:localhost:3389 /cert:ignore
```

### Issue: Black Screen in RDP

**Check PipeWire:**
```bash
systemctl --user status pipewire
pw-cli ls Node
# Should see wrd-capture streams
```

**Check logs for frames:**
```bash
# Look for "Processing buffer" or "Got frame" in server output
# If not seeing frames, PipeWire capture isn't working
```

### Issue: Input Doesn't Work

**Check Portal permissions:**
```bash
# Permission should include both screen + input
# If only screen sharing was granted, re-run and grant both
```

---

## Expected Performance (First Test)

### Good Results
- **Video:** Smooth desktop, 30+ fps
- **Input:** Responsive, <50ms latency
- **CPU:** 20-40% on Ubuntu VM
- **Memory:** 200-400 MB
- **Network:** 10-30 Mbps

### If Performance is Bad
- Check VM has enough resources (4+ CPUs, 8GB RAM)
- Check GPU 3D acceleration is enabled
- Run `virt-manager` → VM → Video → Ensure VirtIO-GPU with 3D

---

## Quick Test Commands

### On Host (Your Development Machine)

```bash
# Push code to VM
rsync -av /home/greg/wayland/wrd-server-specs/ user@192.168.10.205:~/wayland-rdp/

# SSH and build
ssh user@192.168.10.205 'cd ~/wayland-rdp && cargo build --release'

# Or: Run setup script remotely
ssh user@192.168.10.205 'bash -s' < scripts/setup-ubuntu.sh
```

### On VM

```bash
# Quick rebuild after code changes
cd ~/wayland-rdp
cargo build --release

# Run with different log levels
./target/release/wrd-server -c config.toml -v     # info
./target/release/wrd-server -c config.toml -vv    # debug
./target/release/wrd-server -c config.toml -vvv   # trace

# Run with specific module debugging
RUST_LOG=wrd_server::pipewire=trace ./target/release/wrd-server -c config.toml

# Save logs to file
./target/release/wrd-server -c config.toml -vv 2>&1 | tee test-session.log
```

---

## Success Criteria for First Test

### Minimum Success (We can work with this)
- [x] Server starts without crashing
- [x] Portal permission can be granted
- [x] RDP client connects
- [x] Some video appears (even if slow)
- [x] Some input works (even if laggy)

### Good Success (Expected)
- [x] Smooth video at 30fps
- [x] Responsive input (<100ms)
- [x] Stable for 5+ minutes
- [x] Can use applications normally

### Excellent Success (Ideal)
- [x] Smooth video at 60fps
- [x] Instant input response
- [x] Stable for 1+ hour
- [x] Feels like local desktop

**Any of these is progress!**

---

## Data to Collect

### During First Test

1. **Screenshot the permission dialog**
2. **Screenshot successful connection**
3. **Save server logs:**
   ```bash
   ./target/release/wrd-server -c config.toml -vv 2>&1 | tee first-test.log
   ```
4. **Note any errors** in logs
5. **Measure FPS** (eyeball or RDP client stats)
6. **Note responsiveness** (subjective)
7. **Check resource usage:**
   ```bash
   htop  # Note CPU and memory
   ```

### After Test

- First test log file
- Any error messages
- Performance observations
- What worked
- What didn't work
- Ideas for improvement

---

## Next Session Planning

Based on test results, we'll:

1. **Fix any crashes** discovered
2. **Fix any obvious bugs** (black screen, no input, etc.)
3. **Optimize** if performance is poor
4. **Add integration tests** based on real scenarios
5. **Wire up metrics** for monitoring
6. **Implement file transfer** if basic RDP works

---

## Quick Reference

### Important Files on VM

```
~/wayland-rdp/
├── target/release/wrd-server   # The binary
├── config.toml                  # Configuration
├── certs/
│   ├── cert.pem                 # TLS certificate
│   └── key.pem                  # TLS private key
└── [source code]
```

### Important Commands

```bash
# Start server
./target/release/wrd-server -c config.toml -vv

# Stop server
Ctrl+C

# Check if running
ps aux | grep wrd-server

# Check port
ss -tlnp | grep 3389

# View PipeWire
pw-top
```

---

## Expected Timeline

**Now:** VM is ready at 192.168.10.205
**+15 min:** Run setup script, build completes
**+20 min:** Start server, grant permission
**+25 min:** Connect from Windows
**+30 min:** **FIRST SUCCESSFUL RDP SESSION** 🎉

---

**You're ready to test! The VM is waiting at 192.168.10.205.**

**Run the setup script and let's see what happens!**

