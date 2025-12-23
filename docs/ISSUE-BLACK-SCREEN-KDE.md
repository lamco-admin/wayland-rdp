# Black Screen Issue - KDE VM Test

**Date:** 2025-12-21
**Test Environment:** KDE Plasma (Wayland) @ 192.168.10.205
**Status:** Connection successful, but no video frames

---

## SYMPTOMS

- ✅ RDP connection successful
- ✅ TLS handshake complete
- ✅ Client connected
- ❌ **Black screen** - no video displayed

---

## DIAGNOSIS FROM LOGS

### What's Working

**Portal Session:** ✅
```
INFO lamco_portal: Portal session started successfully
INFO lamco_portal: PipeWire FD: OwnedFd { fd: 16 }
INFO lamco_portal: Streams: 1
```

**PipeWire Stream Created:** ✅
```
INFO lamco_pipewire::pw_thread: Creating PipeWire thread manager for FD 16
INFO lamco_pipewire::pw_thread: PipeWire Core connected successfully
DEBUG lamco_pipewire::pw_thread: Creating stream 51 for node 51
DEBUG lamco_pipewire::pw_thread: Stream 51 state changed: Unconnected -> Connecting
DEBUG lamco_pipewire::pw_thread: Stream 51 connected to node 51
INFO lamco_pipewire::pw_thread: Stream 51 activated - buffers will now be delivered to process() callback
INFO lamco_pipewire::pw_thread: Stream 51 created successfully
```

**Display Pipeline Started:** ✅
```
INFO lamco_rdp_server::server::display_handler: 🎬 Starting display update pipeline task
```

### What's NOT Working

**NO FRAMES CAPTURED:** ❌

```
Display pipeline heartbeat: 1000 iterations, sent 0, dropped 0
Display pipeline heartbeat: 2000 iterations, sent 0, dropped 0
Display pipeline heartbeat: 3000 iterations, sent 0, dropped 0
Display pipeline heartbeat: 4000 iterations, sent 0, dropped 0
Display pipeline heartbeat: 5000 iterations, sent 0, dropped 0
Display pipeline heartbeat: 6000 iterations, sent 0, dropped 0
```

**Analysis:**
- Pipeline is running (6000+ iterations)
- `sent 0` = No frames processed
- `dropped 0` = No frames received at all

**Root Cause:**
PipeWire stream is created and "activated" but **process() callback is never being called**.

This means PipeWire is not delivering buffers from the compositor.

---

## POSSIBLE CAUSES

### 1. Stream Not Actually Streaming ⚠️ **MOST LIKELY**

**Hypothesis:** Stream state is "Paused" not "Streaming"

PipeWire streams can be in states:
- Unconnected
- Connecting
- Paused (connected but not active)
- Streaming (active, sending buffers)

**Log shows:** "Stream 51 connected" but never shows "Streaming"

**Fix:** Need to explicitly set stream to streaming state or trigger format negotiation

### 2. Format Negotiation Incomplete

**Hypothesis:** Stream created but format not agreed upon

**Missing in logs:**
- No "format_changed" callback
- No "param_changed" callback
- No buffer format information

**Fix:** Check if format parameters are being set correctly

### 3. KDE Portal Not Actively Capturing

**Hypothesis:** Permission granted but compositor not capturing

**Evidence:** Portal session created successfully but no actual capture

**Fix:** May need specific KDE portal configuration

---

## COMPARISON WITH WORKING SESSIONS

From previous working sessions (old docs), we saw:

```
# Working logs would have:
DEBUG lamco_pipewire: process() called - buffer available
DEBUG lamco_pipewire: Frame captured: 1920x1080, format: BGRx
INFO lamco_pipewire: Frame 1 sent to channel
```

**We have NONE of these logs** = process() callback never invoked

---

## INVESTIGATION NEEDED

### Check PipeWire Stream State

Need to add debug logging in `lamco-pipewire` to show:
1. Current stream state (Paused vs Streaming)
2. Format negotiation callbacks (param_changed, format_changed)
3. Why process() is not being called

### Possible Code Issues

**In `lamco-pipewire` crate:**

**Stream creation** (`pw_thread.rs` or similar):
```rust
// We probably have:
stream.connect(...)

// But might need:
pw_stream_set_active(stream, true)  // Explicitly activate
```

**Or format parameters might not be set correctly:**
```rust
// Need to ensure SPA_PARAM_Buffers is set
// Need to ensure format is properly negotiated
```

---

## NEXT STEPS

### 1. Add Debug Logging to lamco-pipewire

Need to add trace logging for:
- Stream state transitions (especially Paused → Streaming)
- param_changed callback
- format_changed callback
- process() callback entry

### 2. Check Stream Active State

Add explicit stream activation:
```rust
// After connecting stream
pw_stream_set_active(stream, true)
```

### 3. Check Format Negotiation

Ensure format params are properly set before connecting.

### 4. Test with GNOME VM

Test on GNOME VM to see if this is KDE-specific or general issue.

---

## CODE LOCATIONS TO CHECK

**lamco-pipewire crate:**
- `crates/lamco-pipewire/src/pw_thread.rs` - Stream creation
- `crates/lamco-pipewire/src/stream.rs` - Stream management (if exists)

**Look for:**
- `pw_stream_connect()` call
- Missing `pw_stream_set_active(true)` call
- Format parameter setup (SPA_PARAM_Format, SPA_PARAM_Buffers)
- State change callbacks

---

## TEMPORARY WORKAROUND

None - this is a fundamental issue with stream activation.

---

## PRIORITY

🔴 **CRITICAL** - Blocking all video functionality

Without frames, the server is useless. This must be fixed before any other testing.

---

**CONCLUSION**

The integration is correct, RDP protocol works, but **PipeWire stream is not delivering frames** because it's not in streaming state or format negotiation is incomplete.

Need to investigate `lamco-pipewire` stream activation code.
