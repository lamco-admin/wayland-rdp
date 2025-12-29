# Why Weston Nested Doesn't Work for Multimonitor Testing

**Issue Discovered**: Portal can't see weston's virtual outputs

---

## THE FUNDAMENTAL PROBLEM

**Portal talks to the MAIN compositor (GNOME), not nested compositors**

**What happens**:
```
Your setup:
├─ Proxmox (provides 1 display to VM)
├─ GNOME (main compositor, sees 1 display)
│   └─ Weston window (nested compositor, creates 2 internal outputs)
│
When Portal starts:
├─ Talks to GNOME (the main compositor)
├─ Asks: "What screens can we share?"
├─ GNOME responds: "I have 1 display (1280×800 from Proxmox)"
└─ Portal shows THIS in the permission dialog

Result: Portal gives us 1 stream (GNOME's display), not weston's 2 outputs
```

**Even though**:
- Weston HAS 2 virtual outputs internally
- RDP server uses WAYLAND_DISPLAY=wayland-1
- Portal IGNORES this and talks to GNOME directly

**This is by design** - Portal is compositor-specific, doesn't see nested compositors.

---

## WHAT YOU'RE SEEING IN THE DIALOG

**Screenshot 288** shows:
- Screen selection from GNOME's perspective
- Lists GNOME's outputs (just the 1 Proxmox display)
- Can select "entire screen" or individual windows
- But ALL options are from GNOME, not weston

**No matter what you select**: Portal provides 1 stream (GNOME desktop)

**Weston approach won't work for Portal multimonitor testing.**

---

## ACTUAL OPTIONS FOR MULTIMONITOR

### Option 1: Code Review (Do This Now) ✅

**Review multimonitor code without running it**:
- Verify layout calculations
- Check coordinate transformations
- Validate Portal multi-stream handling
- Review input routing logic

**Benefit**: Can verify code correctness
**Limitation**: Won't catch runtime bugs

### Option 2: Damage Tracking (Do This Next) ✅

**High value, easy to test**:
- Already implemented
- No multimonitor needed
- Huge bandwidth benefits
- Can test right now

### Option 3: Real Multimonitor Test (Final Round)

**Requires ONE of**:

**A. Proxmox Configuration** (you said this broke console before):
```
VM Settings → Hardware → Display
- Change to 2 displays
- Risk: May break console access again
```

**B. Physical Dual Monitor Setup**:
- Test server on machine with 2 real monitors
- Portal will see both
- True multimonitor test

**C. Different VM/Hardware**:
- Test on different system
- Where multimonitor config is safe

**Defer this until final testing phase** when we have proper test environment

---

## HONEST ASSESSMENT

**Weston nested was a dead end** - I apologize for not realizing Portal limitation earlier.

**For Portal to see multiple displays**, they must be:
1. Configured at hypervisor/hardware level
2. Visible to the main compositor (GNOME/KDE)
3. NOT nested/virtual inside another compositor

**Weston creates virtual outputs**, but Portal can't see them.

---

## REVISED PLAN

**Now**:
1. ✅ Review multimonitor code (Option 1) - I'll do this
2. ✅ Test damage tracking - High value, ready now

**Later** (final testing phase):
3. ⏳ Real multimonitor with Proxmox config or physical hardware

**Sound good?**

Let me start the multimonitor code review now!
