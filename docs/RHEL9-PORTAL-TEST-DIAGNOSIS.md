# RHEL 9 Portal-Only Test - Exhaustive Diagnosis

**Date:** 2026-01-05
**Test:** Portal + Token strategy on RHEL 9 GNOME 40.10
**Result:** ✅ Working but with issues
**Log:** 99,669 lines

---

## Summary of Issues

**User Experience:**
1. ❌ Had to approve permissions TWICE (should be once)
2. ❌ Video quality "awful" for reading text
3. ✅ Right click worked
4. ⚠️ Connection established (something is working)

**Technical Findings:**
1. 🔴 **TWO Portal sessions created** (hybrid mode bug - still present)
2. 🔴 **EGFX initialization delay** (7+ seconds, 300 dropped frames)
3. 🟡 **RemoteFX used initially** (poor bitmap compression before H.264)
4. ✅ **Input working** (Portal keyboard/mouse injection successful)
5. ✅ **H.264/AVC444 working** (after EGFX ready)
6. ⚠️ **Connection error** logged but session continued

---

## Issue #1: Two Permission Dialogs 🔴 CRITICAL

### Evidence from Log

**First Portal Session (23:00:31):**
```
23:00:31.280: Portal Manager initialized successfully
23:00:31.280: Creating combined portal session (ScreenCast + RemoteDesktop)
23:00:31.288: Portal Manager initialized successfully (DUPLICATE!)
23:00:31.288: Creating combined portal session (ScreenCast + RemoteDesktop) (DUPLICATE!)
```

**Second Portal Session (23:00:36, 5 seconds later - first dialog approved):**
```
23:00:36.542: Portal session handle created with 1 streams
23:00:36.543: Portal session created successfully with input injection and clipboard support
23:00:36.544: Portal Manager initialized successfully (THIRD!)
23:00:36.544: Creating combined portal session (AGAIN!)
```

**Third Portal Session (23:00:50, 14 seconds later - second dialog approved):**
```
23:00:50.357: Portal session handle created with 1 streams
23:00:50.357: Separate Portal session created for input+clipboard (non-persistent)
```

### Root Cause

**portal_token.rs is creating TWO managers:**
1. First attempt with persistence → Rejected → Retries without persistence ✓
2. That succeeds and creates portal_handle ✓

**But then server/mod.rs STILL has hybrid mode code:**
- Checks `session_handle.portal_clipboard()` (returns Some because Portal has session)
- Portal strategy path creates ANOTHER PortalManager (line 309-314)
- Then tries to use it but... this path shouldn't run for Portal strategy!

**The bug:** server/mod.rs lines 305-330 run for BOTH Portal and Mutter strategies. Should only run for Mutter.

### Why Two Dialogs

**Dialog 1 (T+5s approved):**
- Portal + Token strategy creates first session
- Tries with persistence
- Rejected
- Retries without persistence
- Success → portal_handle created

**Dialog 2 (T+19s approved):**
- server/mod.rs checks `session_handle.portal_clipboard()`
- Returns Some (Portal has clipboard components)
- Runs "Using Portal clipboard from strategy" path
- Creates ANOTHER Portal session (line 310-314 creates new PortalManager)
- This triggers second permission dialog
- User approves
- Now TWO active Portal sessions (wasteful, confusing)

**Fix Required:**
Portal strategy shouldn't go through hybrid mode at all. The check at line 305 is wrong.

---

## Issue #2: Poor Video Quality 🔴 CRITICAL

### What User Saw

From screenshot: Text is blurry/unreadable

### Evidence from Log

**Initial Encoding (First 7+ seconds):**
```
Codec: RemoteFX
client connected, waiting for EGFX channel (dropped 60 frames at T+0.5s)
waiting for EGFX channel (dropped 120 frames at T+2s)
waiting for EGFX channel (dropped 180 frames at T+4s)
waiting for EGFX channel (dropped 210 frames at T+5s)
waiting for EGFX channel (dropped 240 frames at T+6s)
waiting for EGFX channel (dropped 300 frames at T+7s)
EGFX channel open, initializing (dropped 330 frames at T+7.8s)
```

**Then EGFX Ready (T+8s):**
```
🎬 EGFX channel ready - initializing H.264 encoder
EGFX: Channel ready with V10_6
EGFX: AVC420 + AVC444v2 encoding enabled
✅ AVC444 encoder initialized for 1280×800 (4:4:4 chroma)
```

### Timeline of User's Visual Experience

**T+0 to T+8s: RemoteFX (POOR QUALITY)**
- Using bitmap compression
- Low quality
- What user described as "awful"
- 330 frames of poor quality

**T+8s onwards: AVC444 H.264 (GOOD QUALITY)**
- Hardware H.264 encoding
- 4:4:4 chroma (perfect for text)
- 5000 kbps bitrate
- VUI color space signaling

**User's perception:** "Quality was awful" because first 8 seconds WAS awful (RemoteFX)

### Why EGFX Delay?

**EGFX took 7+ seconds to become ready:**

**Possible causes:**
1. **Client negotiation slow** (8 capability sets exchanged)
2. **DVC channel setup delay** (DRDYNVC → EGFX sub-channel)
3. **Server-side initialization** (creating encoder, buffers)
4. **Network/protocol overhead**

**Configuration:**
```
h264_bitrate: 5000 kbps
qp_min: 10, qp_max: 40, qp_default: 23
avc444_enabled: true
```

**This is a known issue** - not specific to RHEL 9/Portal, happened on Mutter test too

---

## Issue #3: Input Working Correctly ✅

### Evidence

**Keyboard:**
```
Keyboard event injected successfully (10+ times logged)
```

**Mouse:**
```
Pointer motion injected successfully (7+ times logged)
Pointer button injected successfully (right click worked)
```

**No Errors:**
- Zero "Failed to inject" errors (unlike Mutter test)
- Portal input API working perfectly
- 100% success rate

**Input via Portal working as expected.**

---

## Issue #4: EGFX Encoding After Initialization 🟡

### What's Working

**Codec:** AVC444 (4:4:4 chroma subsampling)
- Best possible quality for text/UI
- Full color resolution
- VUI color space signaling

**Frame Processing:**
```
Frame #257 acknowledged
Latency: 8.77ms (excellent)
Frames encoded: 2,771 total
Frame rate: ~30 fps
```

**Encoding Quality:**
```
Y444 values sampled correctly
U444, V444 chroma at full resolution
Main U420/V420 subsampling for main stream
```

### Configuration Analysis

**Bitrate: 5000 kbps for 1280x800**
```
Pixels: 1,024,000
Uncompressed @ 30fps: ~92 Mbps
Compressed @ 5000 kbps: ~183:1 compression ratio
```

**For comparison:**
- YouTube 1080p: 8,000 kbps
- This is 1280x800 @ 5,000 kbps
- **Ratio:** ~4.8 kbps per 1000 pixels (YouTube: ~3.8)
- Actually HIGHER than YouTube

**Bitrate should be adequate** for this resolution.

### Possible Quality Issues

**If quality still poor after H.264 starts:**

1. **QP settings:**
   ```
   qp_min: 10 (high quality)
   qp_max: 40 (can get blocky)
   qp_default: 23 (middle)
   ```
   - QP 23 is reasonable
   - But QP max 40 allows quality to degrade significantly

2. **Aux omission:**
   ```
   avc444_enable_aux_omission: true
   avc444_max_aux_interval: 30 frames
   ```
   - Chroma stream only sent every 30 frames or on change
   - Can cause color artifacts
   - Text rendering may suffer

3. **Scene change threshold:**
   ```
   scene_change_threshold: 0.7
   ```
   - High threshold (0.7) means fewer I-frames
   - More P-frames = more compression artifacts
   - Text may look blocky

---

## Issue #5: Hybrid Mode Still Running (Code Bug) 🔴

### Evidence

```
Portal session created successfully (FIRST)
Using Portal clipboard from strategy (shared session)
Creating separate Portal session for input+clipboard (SECOND!)
```

**This is server/mod.rs lines 305-330** running when it shouldn't.

### Why It's Running

**Code flow:**
```rust
// Line 306: Portal strategy returns Some from portal_clipboard()
if let Some(clipboard) = session_handle.portal_clipboard() {
    // This runs for Portal strategy
    // Creates ANOTHER PortalManager
    // Shows "Using Portal clipboard from strategy"
    // But still creates portal_manager on line 310
}
```

**The check is backwards:**
- Should check: "Is this MUTTER strategy?" (no clipboard)
- Currently checks: "Does session have clipboard?" (always true for Portal)

**Fix:**
```rust
if session_handle.session_type() == SessionType::MutterDirect {
    // Only for Mutter: create Portal hybrid
} else {
    // Portal strategy: use session_handle directly, no extra sessions
}
```

---

## Issue #6: Connection Error (Non-Fatal) ⚠️

### Error

```
ERROR: Connection error, error: failed to accept client during finalize
```

**But:** Connection still worked, video displayed, input functional

**Possible causes:**
1. Client disconnected/reconnected during handshake
2. TLS finalization issue (non-fatal)
3. Race condition in IronRDP server

**Impact:** None (connection recovered)

---

## Screenshot Analysis

### What's Visible

**RDP Window:**
- Terminal with colored text (server logs visible)
- Text is somewhat readable but blurry/compressed
- Background: Red Hat Enterprise Linux desktop
- Console output showing server starting

**Quality Assessment:**
- Not completely broken (can see shapes/colors)
- Text legibility poor (as user reported)
- Likely viewing during RemoteFX phase or low bitrate H.264

### Comparing to Screenshot

**Good signs:**
- Connection established ✓
- Video being transmitted ✓
- Colors approximately correct ✓
- Layout correct ✓

**Bad signs:**
- Text blurry/hard to read
- Compression artifacts visible
- Not crystal clear like local display

---

## Full Diagnostic Timeline

### T+0s: Server Start
```
lamco-rdp-server v0.1.0
GNOME 40.10 detected
Portal v4, RestoreTokens supported
Deployment: Native Package
Credential Storage: GNOME Keyring
```

### T+0.1s: Service Registry
```
Services: 6 guaranteed, 3 best-effort, 0 degraded, 7 unavailable
❌ Direct Compositor API [Unavailable] (Mutter disabled ✓)
✅ Session Persistence [Guaranteed]
✅ Selected: Portal + Token strategy (CORRECT!)
```

### T+0.1s-T+5s: First Permission Dialog
```
Permission dialog will appear
Creating Portal session
[User approves first dialog]
```

### T+5s: First Session Created (but something wrong?)
```
Portal session created
But: immediately creates another PortalManager?
```

### T+5s-T+19s: Second Permission Dialog
```
Using Portal clipboard from strategy
Creating ANOTHER PortalManager
Creating ANOTHER session
[User has to approve AGAIN]
```

### T+19s: Second Session Created
```
Portal session handle created (second time)
Separate Portal session created for input+clipboard
```

### T+19s: Server Initialization Continues
```
Session started with 1 streams, PipeWire FD: 16
Initial desktop size: 1280x800
Input handler created (using Portal)
Graphics drain task started
Multiplexer started
Building IronRDP server
  Codec: RemoteFX (initial)
Server is ready and listening
Listening for connections on 0.0.0.0:3389
```

### T+20s: Client Connects
```
client connected, waiting for EGFX channel (dropped 60 frames)
```

### T+20s-T+27s: EGFX Negotiation (7 second delay)
```
waiting for EGFX channel... (300 frames dropped)
RemoteFX bitmaps sent (poor quality)
```

### T+27s: EGFX Ready
```
🎬 EGFX channel ready - initializing H.264 encoder
Client advertised 8 capability sets (V8, V8.1, V10, V10.2, V10.3, V10.4, V10.5, V10.6)
EGFX: Channel ready with V10_6
EGFX: AVC420 + AVC444v2 encoding enabled
✅ AVC444 encoder initialized for 1280×800 (4:4:4 chroma)
```

### T+27s onwards: H.264 Streaming
```
Frames encoded: 2,771
Frame acknowledgments: Working
Latency: ~8ms (good)
Input: Keyboard and mouse working via Portal
```

---

## Root Cause Analysis

### 1. Two Permission Dialogs

**Cause:** server/mod.rs lines 305-330 hybrid mode code

**Current logic:**
```rust
if let Some(clipboard) = session_handle.portal_clipboard() {
    // Portal strategy has clipboard → runs this path
    // Creates ANOTHER PortalManager
    // Triggers second dialog
}
```

**Should be:**
```rust
if session_handle.session_type() == SessionType::Portal {
    // Portal: use session_handle directly
    // NO extra managers, NO extra sessions
} else {
    // Mutter: create Portal for input
}
```

**Impact:** User confusion, wasted resources, delays startup

### 2. Poor Video Quality (First 7+ Seconds)

**Cause:** EGFX channel initialization delay

**What happened:**
1. Client connects (T+20s)
2. Server advertises RemoteFX (always available, immediate)
3. Server advertises EGFX (requires DVC negotiation)
4. Client connects to server
5. DVC channel negotiates (slow - 7+ seconds)
6. During negotiation: Uses RemoteFX (poor bitmap compression)
7. After negotiation: Switches to H.264 (good quality)

**RemoteFX Quality:**
- Bitmap-based compression
- Not H.264
- Poor text rendering
- **This is what user saw and described as "awful"**

**After H.264 kicks in:** Quality should improve significantly

**Possible fixes:**
1. Pre-negotiate EGFX faster (investigate DVC channel delay)
2. Increase RemoteFX quality during transition
3. Don't advertise RemoteFX (force client to wait for EGFX)
4. Reduce EGFX initialization time

### 3. H.264 Quality (If Still Poor After Switching)

**Bitrate: 5000 kbps** (adequate for 1280x800)

**Possible issues:**
1. **QP too high** (max 40 allows blocky compression)
   - Recommendation: Lower qp_max to 30

2. **Aux omission** (chroma only every 30 frames)
   - For text, chroma is critical
   - Recommendation: Disable aux_omission or reduce interval to 5-10

3. **Scene change threshold** (0.7 is high)
   - Fewer I-frames = more artifacts
   - Recommendation: Lower to 0.3-0.5 for more I-frames

---

## Input Analysis ✅ WORKING

**Keyboard:**
```
Keyboard event injected successfully (20+ logged)
```

**Mouse:**
```
Pointer motion injected successfully (10+ logged)
```

**Right Click:**
```
Pointer button injected successfully
```

**Success Rate:** 100% (vs 0% on Mutter)

**Latency:** Not explicitly logged but input appeared responsive

**Verdict:** Portal input is fully functional on RHEL 9 GNOME 40

---

## PipeWire/Video Capture ✅ WORKING

**Stream:**
- Node ID: 48 (then 51 after reconnect?)
- Resolution: 1280x800
- Format: BGRx (4 bytes/pixel)
- Buffer type: MemFd
- FPS: 60fps negotiated, 30fps actual

**Frame Delivery:**
```
🎬 process() callback fired for stream 48 (continuous)
🎬 Got buffer from stream 48
🎬 Buffer: type=2, size=4096000 bytes
MemFd buffer: copying 4096000 bytes
Frame sent to async runtime
Received frame from PipeWire
```

**Frames processed:** 2,771+ (continuous streaming)

**No frame drops during streaming** (after EGFX ready)

**Verdict:** Video capture pipeline working correctly

---

## EGFX/H.264 Encoding ✅ WORKING

**Codec Negotiation:**
```
Client capabilities: V8, V8.1, V10, V10.2, V10.3, V10.4, V10.5, V10.6
Selected: V10_6
AVC420: Enabled
AVC444: Enabled
```

**Encoder:**
```
Type: AVC444 SINGLE
Bitrate: 5000 kbps
QP: 10-40 (default 23)
Level: 4.0
Color: BT.709 full range
VUI: Enabled
```

**Encoding:**
```
Main stream: YUV420 (downsampled chroma)
Aux stream: Chroma difference (4:4:4 reconstruction)
Aux omission: Enabled (send aux every 30 frames or on change)
```

**Frame Acknowledgments:**
```
Frame 257 acknowledged, latency: 8.77ms
Frame acknowledgments working (2,771+ frames)
```

**Verdict:** H.264 pipeline working, but quality may be affected by QP/omission settings

---

## Recommendations

### Immediate Fixes (This Session)

**1. Fix Two Dialogs (CRITICAL)**

Change server/mod.rs line 305:
```rust
// OLD (wrong):
if let Some(clipboard) = session_handle.portal_clipboard() {

// NEW (correct):
if session_handle.session_type() == SessionType::MutterDirect {
    // Only for Mutter: create Portal hybrid
} else {
    // Portal strategy: skip all of this
    let portal_clipboard_manager = session_handle.portal_clipboard().map(|c| c.manager);
    let portal_input_handle = session_handle;
}
```

**2. Improve Initial Quality (HIGH PRIORITY)**

Option A: Don't advertise RemoteFX (force EGFX)
```rust
// Remove RemoteFX from codec list
let codecs = server_codecs_capabilities(&[])  // Empty = no bitmap codecs
```

Option B: Increase RemoteFX quality during transition
```rust
// Adjust RemoteFX compression (if keeping it)
```

Option C: Pre-initialize EGFX
```rust
// Start EGFX negotiation earlier
// Don't wait for client connection
```

**3. Optimize H.264 Quality**

Update config defaults:
```toml
[egfx]
qp_max = 30  # Was 40 (lower = better quality ceiling)
avc444_enable_aux_omission = false  # Always send chroma (text-critical)
# OR
avc444_max_aux_interval = 5  # Send chroma more frequently
scene_change_threshold = 0.3  # More I-frames (was 0.7)
```

### Testing Needed

**1. Test with fixes**
- Fix two dialogs bug
- Retest on RHEL 9
- Verify only ONE dialog

**2. Test quality improvements**
- Disable aux omission
- Lower qp_max to 30
- Increase bitrate to 8000 kbps
- Retest text readability

**3. Test on other platforms**
- KDE Plasma (verify Portal universal)
- Ubuntu 24.04 (regression test)
- Document quality on each

---

## Success Criteria Met

**✅ Portal Strategy Selected** (Mutter correctly disabled)
**✅ Connection Established** (RDP handshake successful)
**✅ Video Working** (PipeWire → H.264 → Client)
**✅ Input Working** (Keyboard and mouse via Portal)
**❌ Quality Issues** (RemoteFX initial + possible H.264 tuning needed)
**❌ Two Dialogs** (Hybrid mode bug)

---

## Next Actions

**Immediate (Do Now):**
1. Fix hybrid mode check (session_type instead of portal_clipboard check)
2. Test with one dialog
3. Adjust quality settings (qp_max, aux_omission)
4. Retest

**After Fixes:**
1. Commit changes
2. Test on another platform (KDE or Ubuntu 24.04)
3. Document findings
4. Proceed to publication

---

**STATUS: Portal strategy WORKS on RHEL 9, but needs quality tuning and hybrid mode bug fix**
