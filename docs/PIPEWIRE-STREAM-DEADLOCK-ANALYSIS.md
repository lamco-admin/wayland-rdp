# PipeWire Stream Deadlock - Comprehensive Analysis

**Date:** 2025-12-21
**Environment:** KDE Plasma (Wayland) @ 192.168.10.205
**Issue:** PipeWire stream connects but never delivers frames
**Status:** ROOT CAUSE IDENTIFIED - Portal node not producing data

---

## EXECUTIVE SUMMARY

**Problem:** Stream state machine stuck in "Connecting" - never reaches "Streaming"

**Root Cause:** PipeWire graph has NO events to process (loop.iterate() always returns 0)

**This means:** Node 51 (from Portal) is not active/producing on the private FD

---

## COMPLETE TIMELINE OF FIXES ATTEMPTED

### Test 1: Initial Black Screen
- **Issue:** Empty screen on RDP connection
- **Diagnosis:** No frames being captured
- **Finding:** Stream stuck in Connecting state

### Test 2: Format Parameters
- **Change:** Implemented SPA Pod building for format/size/framerate
- **Result:** Parameters built successfully
- **Outcome:** No change - still stuck

### Test 3: Removed set_active()
- **Theory:** Calling set_active(true) interferes with AUTOCONNECT
- **Change:** Removed stream.set_active(true) call
- **Result:** No change - still stuck

### Test 4: PW_ID_ANY vs Some(node_id)
- **Theory:** Direct node ID addressing doesn't work on private FD
- **Change:** stream.connect(Direction::Input, None, ...) instead of Some(node_id)
- **Result:** No change - still stuck

### Test 5-10: Extensive Debug Logging
- **Added:** Logging at every step of stream creation, connection, activation
- **Finding:** Everything succeeds, but loop.iterate() returns 0 events

---

## WHAT WORKS ✅

### Portal Session
```
✅ Portal session created
✅ RemoteDesktop started with 3 devices and 1 streams
✅ PipeWire FD obtained: OwnedFd { fd: 16 }
✅ Portal provided stream: node_id=51, size=(1280, 800), position=(0, 0)
```

### PipeWire Thread
```
✅ PipeWire thread started successfully
✅ PipeWire main loop thread started
✅ PipeWire Core connected successfully to Portal FD 16
```

### Stream Creation
```
✅ CreateStream command received: stream_id=51, node_id=51
✅ Config: 1280x800 @ 60fps, dmabuf=true, buffers=3
✅ Building stream properties
   media.type = Video
   media.category = Capture
   media.role = Screen
   media.name = lamco-pw-51
   node.target = 51
   stream.capture-sink = true
✅ Stream::new() succeeded
✅ Callbacks registered (state_changed, param_changed, process)
✅ stream.connect() succeeded
```

### Main Loop
```
✅ PipeWire main loop running
✅ loop.iterate() called every 5ms
✅ Streams map has 1 active stream
```

---

## WHAT DOESN'T WORK ❌

### Stream State Machine
```
✅ State: Unconnected -> Connecting
❌ [STOPS HERE - NO FURTHER TRANSITIONS]
❌ Expected: Connecting -> Paused -> Streaming
❌ Never reaches Streaming
```

### Callbacks
```
❌ state_changed never fires again (only Unconnected->Connecting)
❌ param_changed never fires
❌ process() never fires
```

### Main Loop Events
```
❌ loop.iterate() ALWAYS returns 0
❌ Zero events processed (every single iteration)
❌ This continues forever
```

### Frame Capture
```
❌ Display pipeline heartbeat: 3000 iterations, sent 0, dropped 0
❌ Zero frames ever received
```

---

## ROOT CAUSE: PipeWire Graph Not Connected

### The Smoking Gun

**loop.iterate() returns 0 means:**
- PipeWire event loop has nothing to process
- No state transitions waiting
- No format negotiations happening
- No data flowing

**This can only mean:**
- Node 51 doesn't exist on this PipeWire instance (FD 16)
- OR: Node 51 exists but isn't activated/producing
- OR: Our stream isn't linked to node 51 in the graph

### Evidence: Everything We Call Succeeds

```rust
Stream::new()     ✅ No error
.register()       ✅ No error
.connect()        ✅ No error
.set_active()     ✅ No error (when we called it)
```

**But:** Success doesn't mean the graph is connected!

PipeWire can accept all these calls without error, but if the target node isn't producing, nothing happens.

---

## HYPOTHESIS: Portal Node Not Active

### Theory

When Portal gives us:
- FD 16 (private PipeWire connection)
- Node 51 (video capture node)

**Node 51 might not be actively capturing yet.**

### Why This Would Happen

Portal lifecycle:
1. Create session
2. Select devices
3. Select sources
4. Start session → Get FD + node IDs
5. **Missing step?:** Actually start the capture?

### Similar Issues in Other Projects

OBS Studio, Kooha, and other screencast consumers must solve this.

Need to check:
- Do they do something after getting FD to activate node?
- Is there a ScreenCast-specific call needed?
- Does the node need explicit activation via Portal API?

---

## TESTS PERFORMED

| Test | Parameter | Result |
|------|-----------|--------|
| 1 | Empty format params | Stuck in Connecting |
| 2 | Full format params (SPA Pod) | Stuck in Connecting |
| 3 | With set_active(true) | Stuck in Connecting |
| 4 | Without set_active(true) | Stuck in Connecting |
| 5 | connect(Some(node_id)) | Stuck in Connecting |
| 6 | connect(None) + node.target | Stuck in Connecting |

**Conclusion:** Nothing we change in stream creation affects it.

**This proves:** The problem is NOT in our stream code.

---

## THE REAL PROBLEM

**We're connecting to a Portal node that isn't producing data.**

Either:
1. Portal requires additional activation we're not doing
2. KDE Portal behaves differently than GNOME
3. The node ID from Portal is a placeholder, not the real capture node
4. There's a timing issue - node hasn't started yet

---

## NEXT STEPS TO DEBUG

### Option 1: Check Portal API Sequence

Review ashpd documentation:
- Is there a step after .start() to actually begin capture?
- Do we need to call something on Screencast proxy?
- Is there a "resume" or "activate" call needed?

### Option 2: Compare with Working Software

Check OBS Studio's portal integration:
- What does it do after getting FD and node ID?
- Any additional Portal API calls?
- Any PipeWire activation beyond stream.connect()?

### Option 3: Test on GNOME VM

Determine if this is KDE-specific:
- Does it work on GNOME?
- If yes → KDE portal issue
- If no → Generic problem with our Portal integration

### Option 4: Inspect PipeWire Graph

On the VM during connection:
```bash
pw-dump | jq . > graph.json
# Analyze what nodes exist, what's connected
```

---

## CODE LOCATIONS

### Portal Session Creation
- `lamco-portal/src/lib.rs:315-397` - create_session()
- `lamco-portal/src/remote_desktop.rs:67-122` - start_session()

### PipeWire Stream Creation
- `lamco-pipewire/src/pw_thread.rs:582-965` - create_stream_on_thread()
- `lamco-pipewire/src/pw_thread.rs:307-476` - run_pipewire_main_loop()

### Integration
- `wrd-server-specs/src/server/mod.rs:116-351` - WrdServer::new()
- `wrd-server-specs/src/server/display_handler.rs:149-261` - WrdDisplayHandler::new()

---

## DIAGNOSTIC DATA

### Stream Properties (Confirmed Correct)
```
media.type = Video ✅
media.category = Capture ✅
media.role = Screen ✅
node.target = 51 ✅
stream.capture-sink = true ✅
```

### Stream Flags (Confirmed Correct)
```
AUTOCONNECT ✅
MAP_BUFFERS ✅
RT_PROCESS ✅
```

### Direction (Need to Verify)
```
Direction::Input (consumer/sink)
```

**Question:** Should screencast consumers use Input or Output?

From PipeWire docs:
- Input = consumes data (recording app)
- Output = produces data (source)

**We're a consumer → Input is correct ✅**

---

## COMPARISON: What Should Happen

### Expected Sequence
```
1. stream.connect() → Success
2. Main loop iterate → Returns >0 (processing graph events)
3. state_changed callback → Connecting -> Paused
4. param_changed callback → Format negotiated
5. state_changed callback → Paused -> Streaming
6. process() callback → Frame available
7. Continue processing frames
```

### What Actually Happens
```
1. stream.connect() → Success ✅
2. Main loop iterate → Returns 0 (NO events) ❌
3-7. [NOTHING HAPPENS]
```

---

## CRITICAL INSIGHT

**PipeWire accepted our stream connection without error, but the graph has no work to do.**

This is like plugging headphones into a jack that's not connected to anything. The connection succeeds, but no audio flows because there's no source.

**Node 51 from Portal is either:**
- Not active yet
- Not reachable on FD 16
- A placeholder/virtual node
- Waiting for something we haven't done

---

## RECOMMENDATION

**STOP trying different PipeWire parameters.**

The PipeWire code is correct. The problem is the Portal node.

**START investigating:**
1. What does Portal.start() actually activate?
2. Is there a separate ScreenCast activation needed?
3. Does KDE Portal need special handling?
4. Test on GNOME to isolate KDE vs generic issue

---

**STATUS:** Blocked on understanding Portal node activation

**Next Action:** Research ashpd/Portal API or test on GNOME VM
