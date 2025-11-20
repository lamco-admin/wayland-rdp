# Compositor vs Native: The Real Decision

## NEW DISCOVERY: KDE ≠ GNOME!

**KDE Plasma Wayland** has MUCH better clipboard support:
- ✅ Klipper DBus API (works TODAY)
- ✅ wlr-data-control protocol
- ✅ ext-data-control-v1 protocol
- ✅ Portal backend with monitoring (recent)

**GNOME** has essentially nothing:
- ❌ No native clipboard manager
- ❌ No data-control protocols
- ❌ Portal backend doesn't monitor

---

## REVISED STRATEGY

### For Each Desktop Environment:

**KDE Plasma 6+**:
- Use Klipper DBus for instant clipboard monitoring ✅
- Fallback to ext-data-control-v1
- Portal mode works great

**Sway/River/Hyprland (wlroots)**:
- Use wlr-data-control protocol ✅
- Portal mode works great

**GNOME**:
- Portal mode: Windows→Linux only
- OR: Use Lamco compositor for full clipboard
- User chooses based on needs

---

## IMPLEMENTATION PLAN

### Smart Backend Selection

```rust
pub enum ClipboardBackend {
    KlipperDBus,        // KDE - instant, best
    ExtDataControl,     // KDE/modern compositors
    WlrDataControl,     // Sway/wlroots
    Compositor,         // Lamco - always works
    Portal,             // Fallback (limited)
}

fn detect_best_backend() -> ClipboardBackend {
    if klipper_available() {
        ClipboardBackend::KlipperDBus  // KDE
    } else if ext_data_control_available() {
        ClipboardBackend::ExtDataControl  // Modern
    } else if wlr_data_control_available() {
        ClipboardBackend::WlrDataControl  // wlroots
    } else if user_wants_compositor() {
        ClipboardBackend::Compositor  // Lamco
    } else {
        ClipboardBackend::Portal  // Limited fallback
    }
}
```

---

## WHAT TO IMPLEMENT

### Priority 1: Klipper DBus Backend (1 day)
- Works on KDE TODAY
- No polling needed
- Instant clipboard detection
- Simple DBus API

### Priority 2: ext-data-control Backend (2 days)
- Works on KDE, newer compositors
- Direct Wayland protocol
- Event-driven

### Priority 3: Keep Lamco Compositor (Done!)
- For GNOME users who need bidirectional
- For headless deployment
- For cloud VDI

---

## RECOMMENDATION

**Don't force one solution. Build adaptive system:**

```
┌─────────────────────────────────────────┐
│ WRD-Server Clipboard Auto-Detection     │
├─────────────────────────────────────────┤
│                                          │
│ KDE Detected:                            │
│   → Klipper DBus (instant) ✅           │
│   → Bidirectional clipboard              │
│   → Portal mode, native desktop          │
│                                          │
│ Sway/wlroots Detected:                   │
│   → wlr-data-control (event-driven) ✅  │
│   → Bidirectional clipboard              │
│   → Portal mode, native desktop          │
│                                          │
│ GNOME Detected:                          │
│   → Portal mode (Windows→Linux) ✅      │
│   → User choice: Accept limit OR         │
│   → Run compositor mode for full ✅      │
│                                          │
│ Headless:                                │
│   → Compositor mode (always) ✅          │
│   → Full bidirectional clipboard         │
└─────────────────────────────────────────┘
```

---

## THE TRUTH

**"Support ALL Wayland desktops with FULL clipboard":**

**Correct answer**: 
- KDE: ✅ YES (Klipper or protocols)
- Sway: ✅ YES (wlr-data-control)
- GNOME: ⚠️ YES (but requires compositor mode)

**Each desktop gets best available solution.**

---

## NEXT STEPS

**Week 1**: Implement Klipper DBus backend (KDE users get full clipboard)
**Week 2**: Implement ext-data-control backend (modern compositors)
**Week 3**: Polish compositor mode (GNOME users' option)

**Result**: WRD-Server works optimally on EVERY Wayland desktop!

---

**This is the right architecture: Adaptive, not one-size-fits-all.**
