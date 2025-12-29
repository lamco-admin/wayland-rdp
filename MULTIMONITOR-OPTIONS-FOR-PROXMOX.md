# Multimonitor Testing Options for Proxmox VM

**Your Setup**: Proxmox hypervisor with Ubuntu 24.04 GNOME 46 VM
**Issue**: Modifying Proxmox display settings previously broke console access
**Goal**: Test multimonitor WITHOUT breaking console

---

## WHAT I WAS TRYING (And Why It Didn't Work)

**Method**: Pure software GNOME virtual displays
**Idea**: Create virtual monitors inside GNOME without touching Proxmox
**Reality**: GNOME 46 doesn't have this feature enabled in your build

**Why this would have been ideal**:
- No Proxmox changes needed
- No console access risk
- Pure software solution

**Why it failed**:
- Requires GNOME experimental features or extensions
- Not available by default
- Would need installation/configuration

---

## BETTER OPTIONS FOR PROXMOX

### Option 1: Weston Nested Compositor (SAFEST - RECOMMENDED)

**What**: Run a second Wayland compositor inside your VM that creates virtual displays
**Risk**: ZERO - doesn't touch Proxmox or main GNOME session
**Complexity**: Medium

**How it works**:
```
Proxmox VM (1 display from Proxmox)
  └─> GNOME Session (sees 1 display)
      └─> Weston nested compositor (creates 2 virtual outputs)
          └─> Our RDP server connects to Weston
              └─> Portal provides 2 PipeWire streams!
```

**Commands**:
```bash
# Install weston
sudo apt install weston

# Run weston with 2 outputs
weston --width=1920 --height=1080 &

# Weston creates virtual displays
# Our RDP server can connect to Weston's wayland socket
# Portal will see Weston's outputs as separate monitors
```

**Pros**:
- ✅ No Proxmox changes
- ✅ No risk to console
- ✅ Creates real Wayland outputs
- ✅ Portal sees them as separate streams

**Cons**:
- Need to run RDP server inside weston session
- Slightly more complex setup
- Weston is nested (minor performance hit)

---

### Option 2: GNOME Shell Extension (MEDIUM RISK)

**What**: Install extension that adds virtual monitors to GNOME
**Risk**: Low - just software, but could crash GNOME session
**Complexity**: Low if extension exists

**Available extensions**:
- "Virtual Monitor" extension (if exists for GNOME 46)
- Custom extension we could write

**Commands**:
```bash
# Install extension manager
sudo apt install gnome-shell-extension-manager

# Search for "virtual monitor" or "dummy display"
# Install via GUI
```

**Pros**:
- No Proxmox changes
- Works within existing GNOME

**Cons**:
- May not exist for GNOME 46
- Could destabilize GNOME
- Need to find/install extension

---

### Option 3: Test with Physical Second Monitor (IF AVAILABLE)

**What**: If the Proxmox host has 2 physical monitors, passthrough to VM
**Risk**: Depends on Proxmox passthrough config
**Complexity**: Variable

**Only viable if**:
- Proxmox host has 2 monitors
- You can passthrough second display to VM
- You're comfortable with Proxmox GPU passthrough

---

### Option 4: Accept Proxmox Display Change (RISKY)

**What**: Edit Proxmox VM config to add second virtual display
**Risk**: May break console access (you said this happened before)
**Recovery**: Would need to revert Proxmox config

**I don't recommend this** based on your previous experience.

---

### Option 5: Mock Testing (CODE INSPECTION)

**What**: Verify multimonitor code correctness without actual testing
**Risk**: None - just code review
**Limitation**: Can't catch runtime bugs

**What we'd do**:
- Review `src/multimon/` code
- Trace through Portal multi-stream handling
- Verify input routing logic
- Check coordinate transformations
- Validate against spec

**Pros**:
- Zero risk
- Can do right now

**Cons**:
- Not real testing
- May miss runtime issues

---

## MY RECOMMENDATION: Weston (Option 1)

**Why**:
- Safe (no Proxmox changes)
- Actually creates real Wayland outputs
- Portal will provide multiple streams
- True multimonitor test
- Easy to undo (just kill weston)

**Setup time**: 15-20 minutes
**Test time**: 30-40 minutes
**Risk**: Minimal

**Would you like me to guide you through the Weston setup?**

It's basically:
1. Install weston in the VM
2. Run weston with 2 virtual outputs
3. Set environment to use weston's wayland socket
4. Run our RDP server
5. Portal sees 2 outputs from weston
6. Test!

---

## ALTERNATIVE: Skip Multimonitor for Now

If all options seem too complex or risky, we could:
- Move to damage tracking (different feature, high value)
- Come back to multimonitor when you have better test setup
- Or test on different hardware with real dual monitors

**What would you like to do?**
