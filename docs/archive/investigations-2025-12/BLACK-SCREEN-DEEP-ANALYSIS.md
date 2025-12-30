# Black Screen - Deep Analysis

**Date:** 2025-12-21
**Issue:** RDP connects but no video frames captured from PipeWire
**Environment:** KDE Plasma (Wayland) @ 192.168.10.205
**Status:** INVESTIGATING ROOT CAUSE

---

## TIMELINE OF INVESTIGATION

### Test 1: Initial Connection
- ✅ RDP connects
- ❌ Black screen
- **Finding:** PipeWire stream never transitions to "Streaming" state

### Test 2: Format Parameters Fix
- **Change:** Implemented SPA Pod building for format/size/framerate
- **Result:** Parameters built successfully but stream still stuck
- **Finding:** Format params not the root cause (old working code also used empty params)

### Test 3: After Discovering .register()
- **Finding:** .register() already present in code (line 895)
- **Status:** Still investigating

---

## CURRENT STATE

### What Works ✅

1. **Portal Session Created:**
   ```
   INFO lamco_portal: Portal session created successfully
   INFO lamco_portal: PipeWire FD: 16
   INFO lamco_portal: Streams: 1
   ```

2. **PipeWire Thread Running:**
   ```
   INFO lamco_pipewire::pw_thread: PipeWire thread started successfully
   INFO lamco_pipewire::pw_thread: PipeWire main loop thread started
   INFO lamco_pipewire::pw_thread: PipeWire Core connected successfully
   ```

3. **Stream Created and Connected:**
   ```
   INFO lamco_pipewire::pw_thread: Building format parameters: 1280x800 @ 60fps
   INFO lamco_pipewire::pw_thread: Format parameters built successfully
   DEBUG lamco_pipewire::pw_thread: Stream 51 state changed: Unconnected -> Connecting
   DEBUG lamco_pipewire::pw_thread: Stream 51 connected to node 51
   INFO lamco_pipewire::pw_thread: Stream 51 activated
   ```

4. **Main Loop Iterating:**
   - `loop_ref.iterate(Duration::from_millis(0))` called every 5ms
   - No errors in loop execution

5. **Display Pipeline Running:**
   ```
   INFO lamco_rdp_server::server::display_handler: Starting display update pipeline task
   ```

### What Doesn't Work ❌

1. **Stream Never Reaches "Streaming" State:**
   - Stuck in "Connecting" forever
   - Never logs "Stream 51 is now streaming"

2. **No Callbacks Fired:**
   - `param_changed` never called (should show "📐 Stream 51 format negotiated")
   - `process()` never called (should show "🎬 process() callback fired")

3. **Zero Frames:**
   ```
   Display pipeline heartbeat: 4000 iterations, sent 0, dropped 0
   ```

---

## ROOT CAUSE ANALYSIS

### Hypothesis 1: Listener Not Registered ❌ DISPROVEN

**Initial theory:** Missing `.register()` call

**Evidence:** Code has `.register()` at line 895 (verified from git history)

**Verdict:** Not the issue

### Hypothesis 2: Format Parameters Invalid ❌ DISPROVEN

**Initial theory:** Empty params prevent negotiation

**Evidence:** Old working code (commit 170ed30) also used empty params and worked

**Verdict:** Not the root cause

### Hypothesis 3: Stream Connection Issue ⚠️ INVESTIGATING

**Theory:** Something about how we connect to Portal's node is wrong

**Evidence:**
- Portal gives us node 51
- We create stream for node 51
- Stream connects but doesn't start
- Node 51 not visible in `pw-cli list-objects` (expected - it's on private FD)

**Questions:**
1. Is node 51 actually a valid video source node?
2. Does KDE Portal provide different node types than GNOME?
3. Is there a step missing after connection?

### Hypothesis 4: Main Loop Not Pumping Events ❌ DISPROVEN

**Theory:** Main loop not iterating so callbacks can't fire

**Evidence:** Loop is running (confirmed in code line 454)

**Verdict:** Not the issue

### Hypothesis 5: Race Condition or Timing Issue ⚠️ POSSIBLE

**Theory:** Stream activation happens before listener is ready, or something similar

**Evidence:** All initialization completes within milliseconds

**Questions:**
1. Should we add a delay after activation?
2. Is there a flush or sync needed?

### Hypothesis 6: KDE Portal Specific Issue ⚠️ STRONG POSSIBILITY

**Theory:** KDE's xdg-desktop-portal-kde behaves differently than GNOME

**Evidence:**
- This worked on GNOME/Ubuntu before
- Now testing on KDE Plasma
- Different portal backends

**Next Step:** Test on GNOME VM to compare

---

## CODE COMPARISON: Working vs Current

### Old Working Code (commit 403005e)

**build_stream_parameters:**
```rust
fn build_stream_parameters(_config: &StreamConfig) -> Result<Vec<Pod>> {
    let params = Vec::new();
    // ... variable setup but never used ...
    Ok(params)  // Returns empty Vec
}
```

**Stream creation:**
```rust
stream.add_local_listener::<()>()
    .state_changed(...)
    .param_changed(...)
    .process(...)
    .register()?;  // ✅ HAS THIS

stream.connect(..., &mut params)?;
stream.set_active(true)?;
```

### Current Code (published crates)

**build_stream_parameters:**
```rust
fn build_stream_parameters(config: &StreamConfig) -> Result<Vec<Pod>> {
    info!("Building format parameters...");
    // ... Pod building code ...
    Ok(vec![pod])  // Returns format params
}
```

**Stream creation:**
```rust
stream.add_local_listener::<()>()
    .state_changed(...)
    .param_changed(...)
    .process(...)
    .register()?;  // ✅ HAS THIS

stream.connect(..., &mut params)?;
stream.set_active(true)?;
```

**Difference:** Only the format params content (empty vs populated)

**Both should work!**

---

## DEBUGGING STEPS TRIED

1. ✅ Check if format params needed → No (old code used empty)
2. ✅ Check if .register() missing → No (already there)
3. ✅ Check if main loop running → Yes (iterating every 5ms)
4. ✅ Check if listener kept alive → Yes (stored in ManagedStream)
5. ⏳ Check if KDE-specific issue → NEXT TEST

---

## NEXT ACTIONS

### Immediate: Test on GNOME VM

To determine if this is KDE-specific:
1. Deploy to GNOME VM (192.168.10.XXX)
2. Run same test
3. Compare logs

If GNOME works → KDE portal issue
If GNOME fails → Generic issue with published crates

### If KDE-Specific:

Research:
- KDE portal differences
- xdg-desktop-portal-kde quirks
- KDE-specific stream activation requirements

### If Generic Issue:

Deep dive:
- Compare exact binary between working (pre-refactor) and current
- Check if pipewire-rs version changed
- Check if libspa version changed
- Review every line changed in refactor

---

## TECHNICAL DETAILS

### PipeWire Connection Flow

```
Portal.Start() → FD 16
  ↓
PipeWireThreadManager::new(FD 16)
  ↓
Core.connect_fd(FD 16)
  ↓
CreateStream command (node_id=51)
  ↓
Stream::new() → add_local_listener() → .register()
  ↓
Stream.connect(node_id=51, params=[...])
  ↓
Stream.set_active(true)
  ↓
[STUCK HERE - no state transition]
```

### Expected After set_active():

```
State: Connecting → Paused
Callback: param_changed (format negotiation)
State: Paused → Streaming
Callback: process() starts firing
```

### What Actually Happens:

```
State: Connecting → [NOTHING]
No callbacks fire
Stream sits idle forever
```

---

## MYSTERIOUS DETAILS

1. **No PipeWire Errors:** System journal shows no PipeWire errors
2. **Portal Connection OK:** xdg-desktop-portal running, no errors
3. **FD Valid:** FD 16 accepted by Core.connect_fd()
4. **Node Exists:** Portal provides node 51 (can't verify via pw-cli due to private FD)

**It's like PipeWire is just... ignoring the stream.**

---

## COMPARISON WITH OTHER PROJECTS

Need to check:
- How does OBS Studio connect to Portal streams?
- How does Kooha (GNOME screencast app) do it?
- Are there PipeWire-rs examples for Portal consumption?

---

**STATUS:** Stuck. Need to either test GNOME VM or dig deeper into PipeWire internals.
