# Multimonitor Testing Setup Guide for VMs
**Date:** 2025-12-25
**Target:** Ubuntu 24.04 + GNOME 46 (KVM)
**Goal:** Add virtual displays for multimonitor testing

---

## QUICK START (Recommended Method)

### Option 1: KVM Virtual GPU with Multiple Heads (Easiest)

**What:** Configure KVM VM to expose multiple virtual displays

**On Host Machine (where VM runs):**

```bash
# 1. Shut down the VM
virsh shutdown ubuntu-wayland-test  # Or whatever your VM name is

# 2. Edit VM configuration
virsh edit ubuntu-wayland-test

# 3. Find the <video> section and modify it:
```

**Change from single head:**
```xml
<video>
  <model type='virtio' heads='1' primary='yes'>
    <acceleration accel3d='yes'/>
  </model>
</video>
```

**To dual head:**
```xml
<video>
  <model type='virtio' heads='2' primary='yes'>
    <acceleration accel3d='yes'/>
  </model>
</video>
```

**Or for 4 virtual monitors:**
```xml
<video>
  <model type='virtio' heads='4' primary='yes'>
    <acceleration accel3d='yes'/>
  </model>
</video>
```

**4. Start VM:**
```bash
virsh start ubuntu-wayland-test
```

**5. In the VM (via virt-manager console or VNC):**

GNOME will auto-detect the virtual displays. Configure in Settings:
```bash
# Open display settings
gnome-control-center display

# You should see multiple displays
# Arrange them (side-by-side, stacked, etc.)
# Apply configuration
```

**Verification:**
```bash
# Check how many monitors GNOME sees
gnome-randr | grep "Display"  # Should show multiple displays

# Or check via D-Bus
gdbus introspect --session --dest org.gnome.Mutter.DisplayConfig \
  --object-path /org/gnome/Mutter/DisplayConfig
```

---

### Option 2: QXL Graphics with Multiple Heads (Alternative)

**If virtio-gpu doesn't work, try QXL:**

```xml
<video>
  <model type='qxl' ram='65536' vram='65536' vgamem='16384' heads='2'>
    <acceleration accel3d='no'/>
  </model>
</video>
```

**Note:** QXL is older but more widely supported

---

### Option 3: GNOME Virtual Displays (Pure Software)

**What:** Create virtual displays within GNOME (no hypervisor changes)

**Requires:** GNOME 43+ with Wayland

**Method A: GNOME Settings (GUI)**

```bash
# In GNOME Settings → Displays
# GNOME on Wayland sometimes shows "Virtual Display" option
# Check if available in your setup

gnome-control-center display
```

**Method B: Mutter Headless Outputs (Advanced)**

```bash
# Create virtual output via D-Bus (GNOME 46+)
# This is experimental and may not be exposed

# Check if available:
busctl --user introspect org.gnome.Mutter.DisplayConfig \
  /org/gnome/Mutter/DisplayConfig | grep -i virtual
```

---

### Option 4: Software Virtual Displays (Fallback)

**If hypervisor method doesn't work:**

**Using dummy-display-connector (Wayland virtual output):**

```bash
# Install dummy display driver
sudo apt install -y xserver-xorg-video-dummy

# This creates virtual displays that GNOME can use
# May require session restart
```

**Or use weston nested compositor:**

```bash
# Run a nested Wayland compositor with multiple outputs
sudo apt install weston

# Start weston with 2 outputs
weston --width=1920 --height=1080 &
# Weston provides virtual displays to test with
```

---

## STEP-BY-STEP: RECOMMENDED APPROACH

### Step 1: Configure KVM for 2 Virtual Displays

**On your KVM host:**

```bash
# Find your VM name
virsh list --all

# Edit the VM
virsh edit [VM-NAME]

# Find <video> section, change heads='1' to heads='2'

# Save and exit (in vi: :wq)

# Restart VM
virsh shutdown [VM-NAME]
virsh start [VM-NAME]
```

### Step 2: Configure Displays in GNOME

**SSH into VM:**
```bash
ssh greg@192.168.10.205

# Check if GNOME sees multiple displays
gnome-randr
# Should show 2 displays

# Or use GUI (via VNC/virt-manager console):
gnome-control-center display
```

**Arrange displays:**
- Side-by-side: Monitor1 (0,0), Monitor2 (1920,0)
- Stacked: Monitor1 (0,0), Monitor2 (0,1080)
- Different sizes: Mix resolutions

**Apply configuration:**
- Click "Apply" in display settings
- GNOME will remember the layout

### Step 3: Verify Portal Sees Multiple Streams

**Test Portal detection:**

```bash
# Install portal testing tools
sudo apt install -y xdg-desktop-portal-gnome

# Check portal status
systemctl --user status xdg-desktop-portal

# The portal should provide 2 PipeWire streams when you start RDP server
```

### Step 4: Run RDP Server and Check Logs

```bash
# Run server
./run-server.sh

# In logs, look for:
# - "Total streams from Portal: 2"
# - "Portal provided stream: node_id=X, size=(1920, 1080), position=(0, 0)"
# - "Portal provided stream: node_id=Y, size=(1920, 1080), position=(1920, 0)"
```

---

## MULTIMONITOR TEST SCENARIOS

### Test 1: Monitor Detection

**What to Check:**
```
Expected in logs:
✅ "Portal provided stream: node_id=54, size=(1920, 1080), position=(0, 0)"
✅ "Portal provided stream: node_id=55, size=(1920, 1080), position=(1920, 0)"
✅ "Total streams from Portal: 2"
✅ "📊 Full multiplexer queues created" (for each stream)
```

**Test:**
1. Start server
2. Check logs immediately
3. Verify 2 streams detected
4. Note positions and sizes

### Test 2: Layout Verification

**What to Check:**
```
Monitor 1: 0,0 → 1920×1080
Monitor 2: 1920,0 → 1920×1080
Combined desktop: 3840×1080

Or stacked:
Monitor 1: 0,0 → 1920×1080
Monitor 2: 0,1080 → 1920×1080
Combined desktop: 1920×2160
```

**Test:**
1. Windows RDP client connects
2. Check if shows 2 monitors
3. Verify layout matches GNOME configuration
4. Check for any offset errors

### Test 3: Video Display

**What to Check:**
```
Expected:
✅ Both monitors show video
✅ Content correct on each monitor
✅ No black screens
✅ Window spans work (drag window across monitors)
```

**Test:**
1. Open window on monitor 1
2. Move to monitor 2
3. Span window across both
4. Verify all cases work

### Test 4: Input Routing

**What to Check:**
```
Click on Monitor 1 → Input goes to correct stream
Click on Monitor 2 → Input goes to correct stream
Mouse moves across boundary → Smooth transition
```

**Test:**
1. Click on monitor 1 (left side)
2. Click on monitor 2 (right side)
3. Move mouse from 1 → 2 across boundary
4. Check logs for input routing messages

---

## MULTI-RESOLUTION TESTING

Once multimonitor is working, test various resolutions:

### Resolution Test Matrix

**Test Configuration:**

**1. Dual 1080p (Most Common)**
```
Monitor 1: 1920×1080 @ (0,0)
Monitor 2: 1920×1080 @ (1920,0)
Combined: 3840×1080
Expected Level: 4.0 for each
```

**2. Mixed Resolutions**
```
Monitor 1: 2560×1440 @ (0,0)
Monitor 2: 1920×1080 @ (2560,0)
Combined: 4480×1440
Expected Levels: 4.1 (monitor 1), 4.0 (monitor 2)
```

**3. Stacked 1080p**
```
Monitor 1: 1920×1080 @ (0,0)
Monitor 2: 1920×1080 @ (0,1080)
Combined: 1920×2160
Expected Level: 4.0 for each
```

**For Each Configuration:**
1. Set resolution in GNOME settings
2. Start RDP server
3. Check logs for level selection
4. Connect with Windows client
5. Verify display correct
6. Test input on both monitors

---

## TROUBLESHOOTING

### Issue: GNOME Only Shows 1 Display

**Check:**
```bash
# See what Wayland outputs exist
sudo apt install wayland-utils
wayland-info | grep output

# Or check via GNOME:
gnome-randr
```

**Fix:**
- Verify KVM heads='2' is set
- Restart VM completely (not just logout)
- Check virt-manager shows 2 displays in VM settings
- May need to manually add display in GNOME settings

### Issue: Portal Only Provides 1 Stream

**Check:**
```bash
# Portal logs
journalctl --user -u xdg-desktop-portal -f

# When starting RDP server, should see:
# "Creating session for 2 outputs"
```

**Fix:**
- Ensure both displays active in GNOME (not mirrored)
- Check extended desktop mode (not mirror mode)
- Verify portal version: `xdg-desktop-portal --version`

### Issue: Windows Client Doesn't Show 2 Monitors

**Check:**
- Windows client must connect in "Use all monitors" mode
- Or manually span monitors
- Check RDP client settings

**Windows Client Setup:**
```
mstsc.exe → Display tab → "Use all my monitors for the remote session"
```

---

## EXPECTED LOG OUTPUT (Success)

**When starting server with 2 monitors:**

```
INFO lamco_portal: RemoteDesktop started with 3 devices and 2 streams
INFO lamco_portal: 📺 Portal provided stream: node_id=54, size=(1920, 1080), position=(0, 0)
INFO lamco_portal: 📺 Portal provided stream: node_id=55, size=(1920, 1080), position=(1920, 0)
INFO lamco_portal: 📊 Total streams from Portal: 2
INFO lamco_rdp_server::server: Portal session started with 2 streams
INFO lamco_rdp_server::server: Initial desktop size: 3840x1080
```

**When EGFX initializes:**

```
DEBUG Created H.264 encoder: bitrate=5000kbps, max_fps=30, level=Level 4.0
INFO ✅ H.264 encoder initialized for 1920×1088 (aligned)
INFO 📐 Aligning surface: 1920×1080 → 1920×1088 (16-pixel boundary)
INFO ✅ EGFX surface 0 created (1920×1088 aligned)

# Should see similar logs for second surface
```

---

## QUICK SETUP COMMANDS

**For KVM/QEMU (virt-manager):**

```bash
# 1. Shutdown VM
virsh shutdown ubuntu-wayland-test

# 2. Edit VM XML
virsh edit ubuntu-wayland-test

# 3. Change: heads='1' → heads='2' in <video> section

# 4. Start VM
virsh start ubuntu-wayland-test

# 5. Connect via virt-manager console (not SSH!)
# GUI needed to configure displays

# 6. In GNOME Settings → Displays:
#    - See 2 virtual displays
#    - Arrange side-by-side
#    - Apply

# 7. Run RDP server
cd ~
./run-server.sh

# 8. Check for "2 streams" in logs
```

**For VirtualBox:**

```bash
# VirtualBox Manager → VM Settings → Display
# Monitor Count: 2
# Video Memory: 128 MB (increase for multiple monitors)
# Scale Factor: 100%
# Graphics Controller: VMSVGA or VBoxVGA

# Start VM, configure displays in GNOME
```

**For VMware:**

```bash
# VMware → VM Settings → Display
# Number of monitors: 2
# Accelerate 3D graphics: Yes
# Graphics memory: 512 MB

# Start VM, GNOME should see 2 displays
```

---

## TESTING WORKFLOW

### Phase 1: Single Monitor Baseline (5 min)

```bash
# Current setup (1 monitor)
1. Connect with RDP client
2. Verify works perfectly
3. Note bandwidth, latency
4. Disconnect
```

### Phase 2: Add Second Monitor (15-20 min)

```bash
# Configure VM for 2 heads
1. Shutdown VM
2. Edit VM: heads='2'
3. Start VM
4. Configure displays in GNOME (via VNC/console)
5. Verify GNOME shows 2 monitors
6. Apply layout
```

### Phase 3: RDP Server Multimonitor Test (30-40 min)

```bash
# Test multimonitor RDP
1. Start server: ./run-server.sh
2. Check logs:
   - "2 streams" message
   - 2 node_id entries
   - Combined desktop size
3. Connect Windows client (use all monitors)
4. Verify:
   - 2 monitors visible
   - Correct layout
   - Can use both monitors
   - Mouse works on both
   - Keyboard works on both
5. Test scenarios:
   - Open window on monitor 1
   - Drag to monitor 2
   - Span window across both
   - Maximize on each monitor
6. Check logs for any errors
```

### Phase 4: Multi-Resolution Testing (60-90 min)

**Test these combinations:**

```bash
# Configuration 1: Dual 1080p
Monitor 1: 1920×1080
Monitor 2: 1920×1080
→ Test level selection (should be 4.0 for each)

# Configuration 2: Mixed resolutions
Monitor 1: 2560×1440
Monitor 2: 1920×1080
→ Test different levels per monitor

# Configuration 3: 4K single (if VM can handle it)
Monitor 1: 3840×2160
→ Test Level 5.1 selection
```

**For each:**
1. Configure in GNOME
2. Restart RDP server
3. Check level selection in logs
4. Connect and verify
5. Measure performance

---

## WHAT TO COLLECT

### Log Data to Capture

**Monitor Detection:**
```bash
grep "Portal provided stream\|Total streams" ~/kde-test-*.log
```

**Level Selection:**
```bash
grep "Created H.264 encoder\|level=" ~/kde-test-*.log
```

**Surface Creation:**
```bash
grep "Aligning surface\|EGFX surface.*created" ~/kde-test-*.log
```

**Input Routing:**
```bash
grep "routing.*input\|Input multiplexer" ~/kde-test-*.log
```

**Errors:**
```bash
grep "ERROR\|WARN" ~/kde-test-*.log | grep -v "clipboard timeout"
```

### Screenshots to Take

1. GNOME display settings showing 2 monitors
2. Windows RDP client showing 2 monitors
3. Window spanning both monitors
4. Any artifacts or errors

---

## COMMON ISSUES & SOLUTIONS

### Issue 1: VM Only Shows 1 Display in GNOME

**Cause:** KVM configuration not applied or GNOME in mirror mode

**Solution:**
```bash
# Check if VM actually has 2 video heads
virsh dumpxml ubuntu-wayland-test | grep -A5 "<video>"
# Should show heads='2'

# Check GNOME display mode
gnome-control-center display
# Ensure "Join Displays" is OFF (extended, not mirrored)
```

### Issue 2: Portal Only Provides 1 Stream

**Cause:** Displays configured as mirrored instead of extended

**Solution:**
```bash
# In GNOME Settings → Displays
# Ensure "Join Displays" toggle is OFF
# Arrange monitors side-by-side or stacked
# Apply configuration
# Restart RDP server
```

### Issue 3: Windows Client Shows 1 Large Display Instead of 2

**Cause:** Client configured for single monitor mode

**Solution:**
```
In mstsc.exe:
Display tab → "Use all my monitors for the remote session"
Or
Display tab → Select "Monitors to use" → Choose all
```

### Issue 4: Input Goes to Wrong Monitor

**Cause:** Coordinate transformation bug in multimon code

**Solution:**
```bash
# Check logs for input routing:
grep "routing.*mouse\|Input multiplexer" logs.txt

# Coordinate debug info should show:
# Click at (2500, 500) → Monitor 2, local (580, 500)
```

**Report:** This would be a bug to fix in `src/multimon/`

---

## VALIDATION CHECKLIST

### Multimonitor Working Correctly ✅

- [ ] GNOME shows 2 displays in settings
- [ ] Portal provides 2 PipeWire streams
- [ ] Server logs show "2 streams"
- [ ] Windows client shows 2 monitors
- [ ] Layout matches GNOME configuration
- [ ] Can open windows on both monitors
- [ ] Mouse moves between monitors smoothly
- [ ] Keyboard input works on both
- [ ] Window spanning works
- [ ] No coordinate offset errors
- [ ] No visual artifacts specific to multimon

### Multi-Resolution Working ✅

- [ ] 1920×1080: Level 4.0 selected
- [ ] 2560×1440: Level 4.1 selected
- [ ] 3840×2160: Level 5.1 selected (if tested)
- [ ] Different levels per monitor (if mixed)
- [ ] All resolutions display correctly
- [ ] No Windows Event ID 1404 errors
- [ ] Performance acceptable for each resolution

---

## AFTER TESTING

### If Everything Works:

**Excellent!** Multimonitor support validated. Next steps:
1. Document configuration (display layout examples)
2. Update compatibility matrix
3. Move to premium feature development (AVC444)

### If Issues Found:

**We debug together:**
1. You collect logs (as above)
2. I analyze coordinate transformations, stream routing
3. We fix bugs in `src/multimon/` code
4. Iterate until working

---

## ESTIMATED TIME

**Setup:** 15-20 minutes (VM configuration)
**Testing:** 30-40 minutes (basic multimonitor validation)
**Multi-resolution:** 60-90 minutes (testing various combinations)
**Total:** ~2-2.5 hours of your time

**If bugs found:** Additional time for iteration (unpredictable)

---

## READY TO START?

**Simplest path:**

1. **Right now:** Shutdown VM, edit virsh XML, set heads='2'
2. **Start VM:** Boot and configure displays in GNOME
3. **Run server:** `./run-server.sh`
4. **Check logs:** Look for "2 streams"
5. **Connect:** Windows client with "use all monitors"
6. **Report:** Send me the logs and findings

Let me know when you're ready to begin, or if you hit any issues with the VM configuration!
