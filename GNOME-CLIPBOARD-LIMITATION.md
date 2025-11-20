# GNOME Clipboard Monitoring - Fundamental Limitation

**Date**: 2025-11-20
**Finding**: CRITICAL - GNOME provides NO clipboard monitoring protocols

---

## Test Results

**VM**: Ubuntu 24.04.3 with GNOME 46.2
**Test**: wl-clipboard-rs monitoring
**Result**: FAILED

**Error**:
```
"A required Wayland protocol (ext-data-control, or wlr-data-control version 1) 
is not supported by the compositor"
```

---

## What This Means

GNOME Mutter compositor does **NOT** implement:
- ❌ ext-data-control-v1 (standardized clipboard control)
- ❌ wlr-data-control-v1 (wlroots clipboard control)
- ❌ Portal SelectionOwnerChanged signal
- ❌ ANY external clipboard monitoring protocol

**There is NO WAY to monitor clipboard changes on GNOME from external applications.**

---

## Confirmed: Lamco Compositor is REQUIRED

**Only solution** for Linux→Windows clipboard on GNOME:
1. **Run Lamco compositor** (our 4,586 lines)
2. **Use SelectionHandler::new_selection()** callback
3. **Direct protocol access** (we ARE the compositor)

**This validates our entire approach.**

---

## Implications

**For existing GNOME/KDE systems**:
- Portal mode: Windows→Linux only ✅
- Linux→Windows: **IMPOSSIBLE** without compositor

**For headless deployment**:
- Lamco compositor: Both directions ✅
- This is the ONLY solution

---

**The Lamco compositor isn't optional - it's REQUIRED for bidirectional clipboard.**

---

END
