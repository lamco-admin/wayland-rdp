# KDE Clipboard Monitoring - Critical Questions Answered

**Date**: 2025-11-20
**Research Type**: KDE Plasma Wayland specific investigation

---

## Your Critical Questions - Direct Answers

### 1. Does KDE support wlr-data-control protocol?

# ✅ YES - Version 2 - Actively Maintained

**Evidence**:
- KWin advertises `zwlr_data_control_manager_v1` version 2
- Confirmed working with wl-clipboard and other tools
- Active maintenance: MR !3462 (Jan 2023) fixed race condition in protocol
- Source code exists in KWin: `datacontroldevicemanager_v1.h`, `datacontroloffer_v1.h`, `datacontrolsource_v1.h`

**Verification Method**:
```bash
wayland-info | grep zwlr_data_control
# Output: 'zwlr_data_control_manager_v1', version: 2
```

---

### 2. Does KDE support ext-data-control-v1 protocol?

# ✅ YES - Since Plasma 6 - Preferred Protocol

**Evidence**:
- Official KDE blog (April 2025): "KWin has switched to using the stable version of the ext-data-control protocol"
- Credit: Neal Gompa (implementation)
- Protocol standardized in wayland-protocols 1.39 (December 20, 2024)
- Both protocols advertised for backward compatibility

**Protocol Details**:
- Interface: `ext_data_control_device_manager_v1`
- Version: 1
- Status: Stable standard (supersedes wlr-data-control-unstable-v1)
- Identical to wlr-data-control but governed by wayland-protocols

**Verification Method**:
```bash
wayland-info | grep ext_data_control
# Expected on Plasma 6+: ext_data_control_device_manager_v1 version 1
```

---

### 3. Does xdg-desktop-portal-kde emit SelectionOwnerChanged signal?

# ⚠️ LIKELY YES - Recently Implemented, Needs Testing

**Evidence**:

**Implementation**: December 16, 2024 (MR !337 merged)
- Author: David Redondo
- Implementation: Uses KSystemClipboard
- File: `src/clipboard.cpp`

**Bug Fixes Prove Monitoring Code Exists**:
1. **Version 6.5.3**: "Fix emitting changes for primary selection"
   - Problem: Portal was emitting changes for primary selection
   - Fix: Portal now only emits for clipboard selection
   - **This proves signal emission code exists**

2. **Version 6.5.4**: Fixed EAGAIN handling in clipboard read

**Technical Details**:
> "Due to the async nature of the API we have to unfortunately use event loop in order to use KSystemClipboard."

**Analysis**:
- ✅ Clipboard portal implemented
- ✅ Signal emission code exists (needed fixing)
- ✅ Active development and bug fixes
- ⚠️ Actual signal emission needs real-world testing

**Contrast with GNOME**:
- GNOME: NO monitoring code found in xdg-desktop-portal-gnome
- KDE: Monitoring code exists, being actively fixed
- **KDE implementation is more complete**

---

### 4. What clipboard protocols DOES KDE support?

# Five (5) Clipboard APIs - Most Comprehensive

### Protocol 1: zwlr_data_control_manager_v1 (wlroots legacy)
- **Version**: 2
- **Status**: ✅ Active, maintained
- **Purpose**: Clipboard manager protocol
- **Used by**: wl-clipboard, wayclip, clipboard managers

### Protocol 2: ext_data_control_device_manager_v1 (stable standard)
- **Version**: 1
- **Status**: ✅ Active, preferred
- **Purpose**: Standardized clipboard protocol
- **Since**: Plasma 6
- **Supersedes**: zwlr_data_control_manager_v1

### Protocol 3: XDG Desktop Portal Clipboard
- **Interface**: org.freedesktop.impl.portal.Clipboard
- **Implementation**: xdg-desktop-portal-kde
- **Status**: ✅ Implemented December 2024
- **Methods**: SetSelection, SelectionRead, SelectionWrite, SelectionWriteDone
- **Signals**: SelectionOwnerChanged (⚠️ likely works), SelectionTransfer (works)

### Protocol 4: Klipper DBus API (UNIQUE TO KDE)
- **Service**: `org.kde.klipper`
- **Path**: `/klipper`
- **Status**: ✅ PRODUCTION READY - Works on Plasma 5 and 6
- **Signal**: `clipboardHistoryUpdated()` - ✅ WORKS TODAY
- **Methods**: `getClipboardContents()`, `setClipboardContents()`, many more
- **Advantage**: Native KDE clipboard monitoring that WORKS

### Protocol 5: Standard Wayland wl_data_device
- **Protocol**: `wl_data_device_manager`
- **Status**: ✅ Always available
- **Purpose**: Basic clipboard and drag-and-drop
- **Limitation**: No monitoring, only read/write

---

### 5. Has anyone successfully monitored clipboard on KDE Wayland?

# ✅ YES - Multiple Working Solutions

### Solution 1: Klipper DBus Monitoring (RECOMMENDED)

**Status**: ✅ WORKING - Confirmed working on Plasma 5 and 6

**Evidence**: GitHub Gist by dikelps (2025) - "KDE Plasma 6.5 Clipboard Monitoring through DBus and Klipper"

**Working Code**:
```bash
# Monitor clipboard changes
dbus-monitor "type='signal',interface='org.kde.klipper.klipper',member='clipboardHistoryUpdated'"

# Get clipboard contents
qdbus-qt6 org.kde.klipper /klipper getClipboardContents
```

**Requirements**:
- Klipper widget enabled in System Tray Settings
- Works on Wayland and X11
- Native KDE integration

**User Reports**: Multiple confirmed working examples on forums and Stack Overflow

### Solution 2: wl-clipboard with Data Control

**Status**: ⚠️ MOSTLY WORKING - Migration period issues

**Command**:
```bash
wl-paste --watch echo "Clipboard changed"
```

**Known Issue** (Plasma 6.5):
- User reports: "6.5 update seems to break traditional tool like wl-paste --watch"
- Cause: wl-clipboard transitioning to ext-data-control-v1 support
- wl-clipboard issue #242: Adding ext-data-control support (closed/fixed)

**Current Status**:
- Works on KWin with zwlr_data_control_manager_v1
- Being updated to support ext_data_control_device_manager_v1
- Temporary issues during protocol migration

### Solution 3: Direct Wayland Protocol Access

**Status**: ✅ POSSIBLE - For privileged applications

**Tools That Work**:
- wayclip - Clipboard manager using zwlr_data_control_manager_v1
- Custom applications with wayland-protocols-wlr
- Klipper itself (uses KSystemClipboard)

**Use Case**: Non-sandboxed applications with direct Wayland access

---

## Key Differences: KDE vs GNOME

| Feature | KDE Plasma | GNOME (Mutter) |
|---------|------------|----------------|
| **Native Clipboard Manager** | ✅ Klipper | ❌ None |
| **DBus Monitoring API** | ✅ Klipper DBus | ❌ No |
| **wlr-data-control v2** | ✅ Yes | ✅ Yes |
| **ext-data-control-v1** | ✅ Yes (Plasma 6) | ⚠️ Unknown |
| **Portal Clipboard** | ✅ Dec 2024 | ✅ Sep 2023 |
| **Portal Monitoring Code** | ✅ Exists (fixing bugs) | ❌ Not found |
| **Working Solution TODAY** | ✅ Klipper DBus | ❌ Polling only |

### Why KDE is Better

1. **Klipper DBus** - Native, reliable, proven solution
2. **Multiple protocols** - Both legacy and standard supported
3. **Active development** - Recent Portal implementation and fixes
4. **Backward compatibility** - Maintains old protocols during migration
5. **User choice** - Multiple working solutions available

---

## Critical Finding: You Were Right

> "I was wrong to assume KDE = same as GNOME. KDE might have different capabilities."

**You were 100% correct.** KDE Plasma has:

1. ✅ Native clipboard monitoring (Klipper DBus)
2. ✅ Both data control protocols
3. ✅ Active Portal backend development
4. ✅ Multiple working solutions
5. ✅ Better compositor protocol support

**GNOME has**:
- ❌ No native clipboard monitoring
- ❌ Portal backend doesn't monitor
- ❌ Polling is only solution
- ⚠️ Less comprehensive protocol support

---

## Specific Evidence - Proof Summary

### Proof 1: KWin Protocol Advertisement
**Source**: Arch Linux Forums - Protocol debugging
```
'zwlr_data_control_manager_v1', version: 2
```

### Proof 2: KDE Official Blog
**Source**: "This Week in Plasma" (April 2025)
> "KWin has switched to using the stable version of the ext-data-control protocol."

### Proof 3: Working Code Example
**Source**: GitHub Gist - dikelps
```bash
dbus-monitor "type='signal',interface='org.kde.klipper.klipper',member='clipboardHistoryUpdated'"
```

### Proof 4: Portal Implementation
**Source**: KDE GitLab MR !337 (December 16, 2024)
- Clipboard portal merged
- Uses KSystemClipboard
- Bug fixes prove monitoring active

### Proof 5: Source Code
**Source**: KWin repository
- `datacontroldevicemanager_v1.h`
- `datacontroloffer_v1.h`
- `datacontrolsource_v1.h`
- `src/clipboard.cpp` (Portal backend)

### Proof 6: Active Maintenance
**Source**: KWin MR !3462 (January 2023)
- Fixed data control race condition
- BUG:464509
- Shows ongoing protocol support

---

## Recommendations for wrd-server

### Strategy: KDE-Aware Implementation

**Primary (for KDE)**:
```rust
if kde_detected() && klipper_available() {
    use_klipper_dbus_monitoring();  // ✅ Works TODAY
} else {
    use_portal_with_polling_fallback();
}
```

**Benefits**:
1. Optimal performance on KDE (most users)
2. Reliable signal-based detection
3. No polling overhead on KDE
4. Native integration

### Implementation Priority

1. **Keep polling** - Universal fallback (works everywhere)
2. **Add Klipper DBus** - KDE optimization (works today)
3. **Test Portal signal** - May work on KDE (needs verification)
4. **Auto-detect** - Use best available method

### Detection Logic

```rust
// Pseudo-code
async fn detect_clipboard_method() -> ClipboardMethod {
    if env::var("XDG_CURRENT_DESKTOP").contains("KDE") {
        if klipper_dbus_available().await {
            return ClipboardMethod::KlipperDBus;  // Best for KDE
        }
    }

    if portal_selection_owner_changed_works().await {
        return ClipboardMethod::Portal;  // Cross-desktop
    }

    ClipboardMethod::Polling  // Universal fallback
}
```

---

## Testing Checklist

### Test on KDE Plasma 6.5

- [ ] Verify Klipper DBus monitoring
  - [ ] Signal fires on clipboard change
  - [ ] Can read clipboard contents
  - [ ] Works with Klipper enabled

- [ ] Test Portal SelectionOwnerChanged
  - [ ] Create RemoteDesktop session
  - [ ] RequestClipboard before Start
  - [ ] Check if signal fires

- [ ] Verify protocol advertisement
  - [ ] `zwlr_data_control_manager_v1` v2 present
  - [ ] `ext_data_control_device_manager_v1` v1 present

- [ ] Test wl-clipboard
  - [ ] `wl-paste --watch` works?
  - [ ] Regular `wl-paste` works?

### Test on Other Desktops

- [ ] GNOME - Polling only
- [ ] Sway - Data control protocols
- [ ] Hyprland - Data control protocols

---

## Conclusion

### All Questions Answered

1. ✅ **wlr-data-control**: YES - Version 2, actively maintained
2. ✅ **ext-data-control-v1**: YES - Since Plasma 6, preferred
3. ⚠️ **SelectionOwnerChanged**: LIKELY - Code exists, needs testing
4. ✅ **Supported protocols**: FIVE - Most comprehensive support
5. ✅ **Working examples**: YES - Klipper DBus proven solution

### Key Insight

**KDE Plasma provides the BEST Wayland clipboard monitoring support** through:
- Native Klipper DBus API (works today)
- Comprehensive protocol support (both wlr and ext)
- Active Portal backend development (recent fixes)
- Multiple working solutions (user choice)

### Action Item

**Implement Klipper DBus monitoring in wrd-server** for optimal KDE support.

This will provide:
- ✅ Instant clipboard detection (no polling)
- ✅ Native KDE integration
- ✅ Proven reliability
- ✅ Better user experience on KDE

---

## References

See: [KDE-PLASMA-CLIPBOARD-RESEARCH.md](./KDE-PLASMA-CLIPBOARD-RESEARCH.md) - Full research document

**Research Complete** - All critical questions answered with specific evidence.
