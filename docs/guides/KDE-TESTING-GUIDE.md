# KDE Clipboard Testing Guide

**Date**: 2025-12-09
**Purpose**: Test Linux→Windows clipboard on KDE to isolate root cause
**Branch**: `feature/gnome-clipboard-extension`

---

## Why KDE Testing is Critical

The handover document identified that `SendInitiateCopy` generates wrong PDUs (CB_LOCK_CLIPDATA + CB_FORMAT_LIST_RESPONSE instead of CB_FORMAT_LIST). This test determines:

**If Linux→Windows works on KDE:**
- ✅ Issue is D-Bus extension timing/integration (GNOME-specific)
- ✅ Portal SelectionOwnerChanged path works correctly
- 🔧 Fix: Improve D-Bus signal handling to match Portal behavior

**If Linux→Windows fails on KDE too:**
- ❌ Issue is fundamental IronRDP `initiate_copy()` state machine bug
- ❌ Affects both Portal and D-Bus paths
- 🔧 Fix: Bypass IronRDP with direct PDU construction

---

## Test Environment Setup

### KDE VM Requirements
- **OS**: Kubuntu 24.04 / KDE neon / Debian 13 with KDE Plasma
- **Session**: Wayland (NOT X11!) - verify with `echo $XDG_SESSION_TYPE`
- **Network**: Bridge to 192.168.10.x (same as GNOME test box)
- **SSH**: Enabled for deployment

### Verify KDE on Wayland
```bash
# Should output "wayland"
echo $XDG_SESSION_TYPE

# Should show Plasma/KWin compositor
echo $XDG_CURRENT_DESKTOP

# Check for wlr-data-control protocol (may not be needed if Portal works)
weston-info | grep wlr_data_control
```

---

## Deployment Workflow

### On Dev Machine (192.168.10.x)
```bash
cd /home/greg/wayland/wrd-server-specs

# Check what we're deploying
git status
git log --oneline -3

# Push to GitHub
git add -A
git commit -m "Add KDE testing enhancements and guide"
git push origin feature/gnome-clipboard-extension
```

### On KDE Test VM
```bash
# SSH to KDE VM
ssh user@192.168.10.xxx

# Navigate to project (or git clone if first time)
cd ~/wayland/wrd-server-specs

# Pull latest code
git pull origin feature/gnome-clipboard-extension

# Build in release mode
cargo build --release

# Run with logging
./target/release/wrd-server -c config.toml 2>&1 | tee kde-test-$(date +%Y%m%d-%H%M%S).log
```

---

## Testing Procedure

### Phase 1: Verify Portal Path Activates

Look for this in startup logs:
```
✅ SelectionOwnerChanged listener started - monitoring Linux clipboard
   🖥️  Using Portal path (KDE/Sway/wlroots mode) - NOT D-Bus extension
```

**If you see D-Bus path instead**, the Portal isn't working on your KDE setup.

### Phase 2: Test Windows → Linux (Sanity Check)

1. Connect from Windows RDP client
2. Copy text in Windows (Ctrl+C)
3. Paste in KDE (Ctrl+V)
4. **Expected**: ✅ Paste works (this already works on GNOME)

**Look for in logs:**
```
🔒 RDP client took clipboard ownership
📥 handle_rdp_format_list: X formats
Portal SetSelection completed
✅ SelectionTransfer completed
```

### Phase 3: Test Linux → Windows (THE CRITICAL TEST)

1. Copy text in KDE (Ctrl+C) - use Kate, Konsole, or any app
2. Check logs immediately for clipboard detection
3. Try to paste in Windows (Ctrl+V)

**Expected log sequence if working:**
```
📋 Local clipboard change #1: X formats: [...]
✅ Sent PortalFormatsAvailable event to clipboard manager
📥 handle_portal_formats called with X MIME types (force=false): [...]
📋 Sending FormatList to RDP client:
   Format 0: ID=13, Name=""
   Format 1: ID=1, Name=""
📤 Sending ServerEvent::Clipboard(SendInitiateCopy) with 2 formats
✅ ServerEvent::Clipboard sent successfully to IronRDP event loop
[ironrdp_cliprdr TRACE] initiate_copy called with state=Ready
[ironrdp_server TRACE] McsMessage::SendDataRequest.*0002  ← CB_FORMAT_LIST!
```

**Expected log sequence if BROKEN (same as GNOME):**
```
📋 Local clipboard change #1: X formats: [...]
✅ Sent PortalFormatsAvailable event to clipboard manager
📥 handle_portal_formats called with X MIME types (force=false): [...]
📤 Sending ServerEvent::Clipboard(SendInitiateCopy) with 2 formats
✅ ServerEvent::Clipboard sent successfully to IronRDP event loop
[ironrdp_cliprdr TRACE] initiate_copy called with state=Initialization  ← WRONG STATE!
[ironrdp_server TRACE] McsMessage::SendDataRequest.*000A  ← CB_LOCK_CLIPDATA (wrong!)
[ironrdp_server TRACE] McsMessage::SendDataRequest.*0003  ← CB_FORMAT_LIST_RESPONSE (wrong!)
```

---

## Log Analysis: What to Look For

### 1. Which Path Activated?

**Portal Path (expected on KDE):**
```
🖥️  Using Portal path (KDE/Sway/wlroots mode)
```

**D-Bus Path (unexpected on KDE):**
```
🔧 Using D-Bus path (GNOME mode)
```

If you see D-Bus on KDE, your Portal isn't working - check KDE version and try alternate compositor.

### 2. Clipboard Change Detection

**Portal detecting changes:**
```
📋 Local clipboard change #X: Y formats: ["text/plain", "text/html", ...]
```

**NOT detecting:**
- No messages appear when you copy in KDE
- Check: `systemctl --user status xdg-desktop-portal-kde`

### 3. Format Announcement

**Correct PDU (msgType=0x0002):**
```
[ironrdp_server] McsMessage::SendDataRequest { channel_id: 1004, user_id: 1003, data: [16, 0, 0, 0, 19, 0, 0, 0, 2, 0, ...] }
                                                                                              ^^ 0x02 = CB_FORMAT_LIST ✅
```

**Wrong PDUs:**
```
[ironrdp_server] McsMessage::SendDataRequest { channel_id: 1004, user_id: 1003, data: [12, 0, 0, 0, 19, 0, 0, 0, 10, 0, ...] }
                                                                                              ^^ 0x0A = CB_LOCK_CLIPDATA ❌

[ironrdp_server] McsMessage::SendDataRequest { channel_id: 1004, user_id: 1003, data: [8, 0, 0, 0, 19, 0, 0, 0, 3, 0, ...] }
                                                                                             ^^ 0x03 = CB_FORMAT_LIST_RESPONSE ❌
```

### 4. IronRDP State

**Good state:**
```
[ironrdp_cliprdr] Cliprdr::initiate_copy called
[ironrdp_cliprdr] Current state: Ready
[ironrdp_cliprdr] Encoding CB_FORMAT_LIST
```

**Bad state:**
```
[ironrdp_cliprdr] Cliprdr::initiate_copy called
[ironrdp_cliprdr] Current state: Initialization  ← PROBLEM!
[ironrdp_cliprdr] incorrect state
```

### 5. Connection Issues

**Multiple backends (bad):**
```
[ironrdp_server] CliprdrServer backend #1 created
[ironrdp_server] ERROR: Connection reset by peer
[ironrdp_server] CliprdrServer backend #2 created
```

This suggests connection instability causes multiple backend instances, leading to state confusion.

---

## PDU Reference

For log analysis, here are the clipboard PDU message types:

| PDU Type | msgType | Expected When | Direction |
|----------|---------|---------------|-----------|
| CB_FORMAT_LIST | 0x0002 | Server announces clipboard | Server → Client |
| CB_FORMAT_LIST_RESPONSE | 0x0003 | Response to format list | Client → Server |
| CB_FORMAT_DATA_REQUEST | 0x0004 | Request clipboard data | Client → Server |
| CB_FORMAT_DATA_RESPONSE | 0x0005 | Provide clipboard data | Server → Client |
| CB_LOCK_CLIPDATA | 0x000A | Lock during transfer | Server → Client |
| CB_UNLOCK_CLIPDATA | 0x000B | Unlock after transfer | Server → Client |

**What we expect:** Server sends `CB_FORMAT_LIST (0x0002)` when Linux copies
**What we get:** Server sends `CB_LOCK_CLIPDATA (0x000A)` + `CB_FORMAT_LIST_RESPONSE (0x0003)`

---

## Test Results Template

Copy this template when reporting results:

```
## KDE Test Results

**Date**: YYYY-MM-DD HH:MM
**VM**: [KDE version, OS]
**Desktop**: [output of echo $XDG_CURRENT_DESKTOP]
**Session Type**: [output of echo $XDG_SESSION_TYPE]

### Startup
- [ ] Portal path activated (saw "Using Portal path" message)
- [ ] D-Bus path activated (saw "Using D-Bus path" message)
- [ ] Neither path activated (ERROR)

### Windows → Linux
- [ ] Copy in Windows → Paste in Linux: SUCCESS
- [ ] Copy in Windows → Paste in Linux: FAILED

### Linux → Windows (CRITICAL)
- [ ] Copy in Linux detected in logs: YES / NO
- [ ] FormatList sent (0x0002 in logs): YES / NO
- [ ] Wrong PDUs sent (0x000A, 0x0003): YES / NO
- [ ] Paste in Windows: SUCCESS / FAILED

### IronRDP State
- [ ] State was "Ready" when initiate_copy called
- [ ] State was "Initialization" when initiate_copy called
- [ ] Multiple backends created (connection errors): YES / NO

### Conclusion
- [ ] Linux→Windows works on KDE (issue is D-Bus-specific)
- [ ] Linux→Windows fails on KDE (issue is IronRDP fundamental)
- [ ] Inconclusive (Portal didn't activate, test again)
```

---

## Next Steps Based on Results

### If Linux→Windows WORKS on KDE ✅

**Root cause**: D-Bus extension timing or integration issue

**Action items**:
1. Study Portal path implementation in detail
2. Make D-Bus path mimic Portal behavior exactly
3. Consider: Is D-Bus sending events too early/late?
4. Check: Does D-Bus need to wait for IronRDP Ready state?
5. Implement: Retry logic or state checking before SendInitiateCopy

**Branch to review**: Current branch already has both paths, just fix D-Bus

### If Linux→Windows FAILS on KDE ❌

**Root cause**: IronRDP `initiate_copy()` state machine fundamentally broken

**Action items**:
1. Confirm multiple backend issue (connection retries creating wrong instances)
2. Implement direct PDU construction bypass (see DONOTUSE/clipboard-debugging/)
3. Or: Fix connection stability to prevent multiple backends
4. Or: Fork IronRDP and patch cliprdr state machine properly
5. File bug report with IronRDP project with evidence

**Alternative**: Use `feature/wlr-clipboard-backend` branch which may have different architecture

---

## Debugging Tips

### Enable Even More Logging

If needed, set environment variable before running:
```bash
RUST_LOG=trace ./target/release/wrd-server -c config.toml 2>&1 | tee kde-test.log
```

### Monitor D-Bus (to confirm it's NOT activating on KDE)

```bash
# In separate terminal
dbus-monitor --session "interface='io.github.lamco.WaylandRdp.Clipboard'"
# Should be silent on KDE (no GNOME extension)
```

### Check Portal Status

```bash
# List all portals
busctl --user list | grep portal

# Check KDE portal specifically
systemctl --user status xdg-desktop-portal-kde
```

### Decode PDU Bytes

Python script to decode msgType from log hex:
```python
# Example log line:
# data: [16, 0, 0, 0, 19, 0, 0, 0, 2, 0, ...]
#                                   ^^
# Position 8-9 = msgType (little-endian u16)

data = [16, 0, 0, 0, 19, 0, 0, 0, 2, 0]
msg_type = data[8] | (data[9] << 8)
print(f"msgType: 0x{msg_type:04x}")

# 0x0002 = CB_FORMAT_LIST ✅
# 0x000A = CB_LOCK_CLIPDATA ❌
# 0x0003 = CB_FORMAT_LIST_RESPONSE ❌
```

---

## Expected Test Duration

- **Setup**: 30-60 minutes (if VM needs creation)
- **First test**: 15 minutes
- **Log analysis**: 15-30 minutes
- **Total**: 1-2 hours for definitive answer

---

## Contact & References

**Session Handover**: `SESSION-HANDOVER-CLIPBOARD-BIDIRECTIONAL-2025-12-09.md`
**Investigation**: `DONOTUSE/clipboard-debugging/CLIPBOARD-LINUX-TO-WINDOWS-INVESTIGATION.md`
**Repository**: https://github.com/lamco-admin/wayland-rdp
**Branch**: `feature/gnome-clipboard-extension`

---

## Files to Review After Testing

If tests complete, these files have the key implementation:
- `src/clipboard/manager.rs` - Portal vs D-Bus path selection
- `src/clipboard/ironrdp_backend.rs` - IronRDP integration
- `src/clipboard/sync.rs` - State management and loop detection

**Good luck! This test will definitively answer whether we fix D-Bus or bypass IronRDP.**
