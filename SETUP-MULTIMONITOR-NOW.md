# Setup Multimonitor Testing - Step by Step

**VM**: ubuntu-wayland-test (192.168.10.205)
**GNOME**: 46.0 ✅
**Method**: KVM virtual GPU with 2 heads

---

## STEP 1: Configure VM for Dual Displays (ON KVM HOST)

You need to run this on the **KVM host machine** (the machine running the VM, not the VM itself):

```bash
# Find VM name
virsh list --all

# If VM name is ubuntu-wayland-test:
virsh shutdown ubuntu-wayland-test

# Edit VM configuration
virsh edit ubuntu-wayland-test
```

**In the editor that opens**, find the `<video>` section and change:

**FROM**:
```xml
<video>
  <model type='virtio' heads='1' primary='yes'>
    <acceleration accel3d='yes'/>
  </model>
</video>
```

**TO**:
```xml
<video>
  <model type='virtio' heads='2' primary='yes'>
    <acceleration accel3d='yes'/>
  </model>
</video>
```

Save and exit (in vi: press ESC, type `:wq`, press ENTER)

**Start VM**:
```bash
virsh start ubuntu-wayland-test
```

---

## STEP 2: Configure Displays in GNOME (VIA GUI)

**You'll need GUI access to the VM**. Use one of:
- virt-manager console (opens GUI window)
- VNC connection to VM
- Physical monitor if VM is on local machine

**Can't do this via SSH** - GNOME display settings require GUI.

**In the VM GUI**:

1. Open Settings:
   ```bash
   gnome-control-center display
   ```

2. You should now see **2 displays** (Virtual-1, Virtual-2)

3. Arrange them:
   - **Side-by-side**: Drag monitor 2 to the right of monitor 1
   - Recommended: 1920×1080 each → Combined 3840×1080

4. Click **Apply**

5. Verify: Both monitors should show desktop

---

## STEP 3: Verify Setup (FROM SSH)

Once displays are configured, SSH back in and verify:

```bash
ssh greg@192.168.10.205

# Check environment
echo $WAYLAND_DISPLAY  # Should show wayland-0 or similar
echo $XDG_SESSION_TYPE  # Should show 'wayland'

# Verify we can connect to session bus
echo $DBUS_SESSION_BUS_ADDRESS  # Should show unix:path=...
```

---

## STEP 4: Test RDP Server

```bash
cd ~
./run-server.sh
```

**Look for in logs**:
```
📺 Portal provided stream: node_id=XX, size=(1920, 1080), position=(0, 0)
📺 Portal provided stream: node_id=YY, size=(1920, 1080), position=(1920, 0)
📊 Total streams from Portal: 2
Initial desktop size: 3840x1080
```

If you see "2 streams" → Success! Multimonitor is detected.

---

## ALTERNATIVE: If You Can't Edit VM

**If you don't have access to virsh**, let me know and I'll guide you through:

**Option A**: Use virt-manager GUI
- Open virt-manager
- Right-click VM → Open
- View → Details → Video
- Change to 2 displays
- Apply, restart VM

**Option B**: Pure software virtual displays (less reliable)
- May not work on all GNOME versions
- Requires experimental features

---

## READY TO PROCEED?

**After you complete Steps 1-2** (VM config + GNOME display setup):

1. Run `./run-server.sh`
2. Send me the log
3. I'll analyze and verify multimonitor is working

**Or if you get stuck**:
- Let me know which step
- I'll provide more specific guidance

**Estimated time**: 15-20 minutes if you have virsh/virt-manager access

Ready when you are!
