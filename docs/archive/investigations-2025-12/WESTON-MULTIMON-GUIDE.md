# Weston Multimonitor Testing Guide

**Setup**: Weston nested compositor in GNOME
**Safety**: No Proxmox changes, fully reversible
**Result**: 2 virtual displays for testing

---

## QUICK START

### On Test VM (192.168.10.205)

**Step 1: Run setup** (requires sudo password):
```bash
ssh greg@192.168.10.205
./setup-weston-multimon.sh
```

This will:
- Install weston
- Create config for 2 virtual outputs (1920×1080 each)
- Create helper scripts

**Step 2: Start weston** (Terminal 1):
```bash
./run-weston.sh
```

You'll see a Weston window open (3840×1080 - dual monitors side-by-side)

**Step 3: Run RDP server** (Terminal 2 or tmux):
```bash
./run-server-weston.sh
```

**Step 4: Check logs for**:
```
📺 Portal provided stream: node_id=X, size=(1920, 1080), position=(0, 0)
📺 Portal provided stream: node_id=Y, size=(1920, 1080), position=(1920, 0)
📊 Total streams from Portal: 2
```

**Step 5: Connect with RDP client**
- Should see 2 monitors!

---

## HOW IT WORKS

### Architecture

```
Proxmox (1 display)
  ↓
GNOME Session (sees 1 physical display)
  ↓
Weston Nested Compositor (creates 2 VIRTUAL outputs)
  ├─> Virtual Output 1: 1920×1080 @ (0,0)
  └─> Virtual Output 2: 1920×1080 @ (1920,0)
  ↓
Portal ScreenCast (sees 2 Weston outputs as separate streams)
  ├─> PipeWire stream 1
  └─> PipeWire stream 2
  ↓
RDP Server (receives 2 streams, multimonitor!)
```

### What Weston Does

**Weston is a reference Wayland compositor**:
- Can run "nested" inside another compositor (GNOME)
- Creates its own Wayland outputs (virtual displays)
- Portal sees these as real separate monitors
- Perfect for testing!

**Safety**:
- Runs in a window inside GNOME
- Close window = everything gone
- No system changes
- No Proxmox modifications

---

## EXPECTED BEHAVIOR

### When You Start Weston

**Window appears**:
- Size: 3840×1080 (2 monitors side-by-side)
- Content: Weston desktop with panel
- You can click inside, open terminals, etc.

**Wayland socket created**:
- Path: `$XDG_RUNTIME_DIR/wayland-1`
- This is Weston's socket (separate from GNOME's wayland-0)

### When You Run RDP Server

**In weston context**, RDP server will:
1. Connect to weston's wayland-1 socket
2. Request Portal session
3. Portal sees 2 weston outputs
4. Provides 2 PipeWire streams
5. **Multimonitor activated!**

**Logs should show**:
```
Portal provided stream: node_id=54, size=(1920, 1080), position=(0, 0)
Portal provided stream: node_id=55, size=(1920, 1080), position=(1920, 0)
Total streams from Portal: 2
Initial desktop size: 3840x1080
```

---

## TESTING CHECKLIST

### Verify Weston Setup

- [ ] Weston window appears (3840×1080)
- [ ] Can click inside weston window
- [ ] Weston shows panel/desktop
- [ ] `ls $XDG_RUNTIME_DIR/wayland-*` shows wayland-1

### Verify RDP Server

- [ ] Server starts without errors
- [ ] Logs show "2 streams from Portal"
- [ ] Both streams have correct positions
- [ ] Combined desktop size: 3840×1080
- [ ] EGFX initializes for both surfaces

### Verify RDP Client

- [ ] Windows client connects
- [ ] Shows 2 monitors in display settings
- [ ] Can see content on both monitors
- [ ] Can move mouse between monitors
- [ ] Keyboard input works on both
- [ ] Can drag windows between monitors

---

## TROUBLESHOOTING

### Issue: Weston Won't Start

**Error**: "failed to create display"

**Fix**:
```bash
# Check if WAYLAND_DISPLAY is set
echo $WAYLAND_DISPLAY  # Should be wayland-0

# Try with explicit display
WAYLAND_DISPLAY=wayland-0 weston --width=3840 --height=1080
```

### Issue: Portal Only Sees 1 Stream

**Possible causes**:
- Weston not creating 2 outputs (check weston.ini)
- RDP server connecting to wrong display (should be wayland-1)
- Portal permission not granted

**Debug**:
```bash
# Check which displays weston created
WAYLAND_DISPLAY=wayland-1 weston-info | grep -A5 "output"

# Check RDP server environment
echo $WAYLAND_DISPLAY  # run-server-weston.sh sets this to wayland-1
```

### Issue: Permission Denied for Portal

**Fix**: Grant permission when dialog appears
- Portal will ask for screen sharing permission
- Grant for both weston outputs

---

## WHAT TO REPORT BACK

After running, send me:

1. **Setup output**: Did weston install successfully?
2. **Weston status**: Does weston window appear?
3. **Server logs**: Does it show "2 streams"?
4. **RDP client**: Do you see 2 monitors?

**Or if issues**:
- Which step failed?
- Error messages
- Logs for analysis

---

## ALTERNATIVE APPROACH

**If weston gives trouble**, we can also try:

**Wayland backend instead of DRM**:
```bash
# Simpler - weston creates 2 windows, each is an output
weston --backend=wayland-backend.so --width=1920 --height=1080 &
weston --backend=wayland-backend.so --width=1920 --height=1080 &
```

But start with the main approach first!

---

**Ready to go! Run `./setup-weston-multimon.sh` on the VM and let me know how it goes!**
