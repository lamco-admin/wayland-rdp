# EGFX Surface Setup Failure - Deep Investigation Plan

**Date:** 2025-12-24
**Issue:** Windows client disconnects 20-30ms after receiving surface setup PDUs (ResetGraphics, CreateSurface, MapSurfaceToOutput)
**Status:** Active Investigation - Multiple diagnostic paths

---

## Validation Test Results

### Hypothesis: H.264 Level Constraints

**Test:** Reduced framerate from 30fps to 27fps to meet Level 3.2 constraint

**Result:** ❌ **INVALIDATED**
- Crash timing unchanged (22-26ms after surface PDUs)
- Crash occurs BEFORE any H.264 frames reach client
- H.264 level constraints are irrelevant to current issue

**Conclusion:** The problem is in EGFX initialization/surface setup, not H.264 streaming.

---

## Current Understanding

### Timeline Analysis

**Correct Sequence (from logs):**
```
T+0.000s - CapabilitiesAdvertise received from client
T+0.000s - CapabilitiesConfirm queued
T+0.000s - drain_output() returns CapabilitiesConfirm (1 message)
T+0.029s - EGFX channel ready callback in display_handler
T+0.029s - Create H.264 encoder
T+0.029s - Create surface (calls server.create_surface())
T+0.029s - Map surface to output
T+0.029s - drain_output() returns 3 messages (ResetGraphics, CreateSurface, MapSurfaceToOutput)
T+0.029s - Encode as DVC messages, send via ServerEvent::Egfx
T+0.029s - ✅ "EGFX surface PDUs sent to client"
T+0.051s - ❌ CLIENT DISCONNECTS (error 0x1108)
T+0.060s - Frame 0 encoded and sent (too late - client already gone)
```

### PDUs Being Sent (Verified in Logs)

1. **CapabilitiesConfirm**
   - Version: V10_6 (0x000A0006)
   - Flags: 0x00000000
   - Queued: ✅
   - Drained: ✅
   - Wire send: ❓ (need to verify)

2. **ResetGraphics** (340 bytes)
   - Width: 1280
   - Height: 800
   - MonitorCount: 0
   - Verified correct

3. **CreateSurface** (15 bytes)
   - SurfaceId: 0
   - Width: 1280
   - Height: 800
   - PixelFormat: 0x20 (XRgb)

4. **MapSurfaceToOutput** (20 bytes)
   - SurfaceId: 0
   - Origin: (0, 0)

All structures verified correct per MS-RDPEGFX.

---

## Investigation Areas

### Area 1: CapabilitiesConfirm Wire Transmission

**Question:** Is CapabilitiesConfirm actually reaching the wire?

**Evidence:**
- Queued: ✅ Line 2181 "Queued CapabilitiesConfirm"
- Drained: ✅ Line 2185/2186 "Draining CapabilitiesConfirm" + "drain_output returning 1 messages"
- Encoded: ❓ No log yet
- Wire send: ❓ No log yet

**Two Paths:**

**Path A: DVC Reactive (where CapabilitiesConfirm should go)**
```rust
// ironrdp-egfx/src/server.rs:1234-1256
fn process(&mut self, _channel_id: u32, payload: &[u8]) -> Vec<DvcMessage> {
    match pdu {
        GfxPdu::CapabilitiesAdvertise(pdu) => {
            self.handle_capabilities_advertise(pdu);  // Queues CapConfirm
        }
        ...
    }
    Ok(self.drain_output())  // Returns CapConfirm here
}

// ironrdp-dvc/src/server.rs:173
let msg = c.processor.process(channel_id, &complete)?;
resp.extend(encode_dvc_messages(channel_id, msg, ...)?);
// Returns to SVC layer

// ironrdp-server/src/server.rs:992-1019
let response_pdus = svc.process(&data.user_data)?;  // Gets CapConfirm
let response = server_encode_svc_messages(response_pdus, ...)?;
writer.write_all(&response).await?;  // Should send to wire
```

**Path B: Proactive (where surfaces go)**
```rust
// display_handler.rs:505-510
let msg = EgfxServerMessage::SendMessages { channel_id, messages };
event_tx.send(ServerEvent::Egfx(msg));

// server.rs:638-649
ServerEvent::Egfx(EgfxServerMessage::SendMessages { channel_id, messages }) => {
    let data = server_encode_svc_messages(messages, ...)?;
    writer.write_all(&data).await?;
}
```

**Logging Added (this session):**
```rust
// server.rs:996-1018
debug!("SVC process returned response PDUs (includes EGFX CapabilitiesConfirm if DRDYNVC)");
debug!("Writing SVC response to wire (CapabilitiesConfirm goes through here!)");
debug!("SVC response written to wire successfully");
```

**Action:** Run test and check if these DEBUG logs appear for CapabilitiesConfirm.

---

### Area 2: V10_6 Capability Flags

**Client Advertises:**
```
V10_6 { flags: 0x0 }  // No flags set
```

**Server Prefers:**
```rust
// handler.rs:276-278
CapabilitySet::V10_6 {
    flags: CapabilitiesV104Flags::SMALL_CACHE,  // 0x02
}
```

**Server Confirms:**
```
V10_6 { flags: 0x0 }  // Client's flags (per negotiation spec)
```

**Questions:**
1. Is flags=0x0 valid for V10_6?
2. Should we confirm with OUR flags or CLIENT's flags?
3. Does AVC_DISABLED flag (0x20) being absent mean AVC is enabled?

**MS-RDPEGFX 2.2.3.9 RDPGFX_CAPSET_VERSION106:**
```
Flags (4 bytes): Capability flags
  0x02 = RDPGFX_CAPS_FLAG_SMALL_CACHE
  0x20 = RDPGFX_CAPS_FLAG_AVC_DISABLED
  0x40 = RDPGFX_CAPS_FLAG_AVC_THINCLIENT

If neither AVC_DISABLED nor AVC_THINCLIENT is set, AVC is enabled.
```

**Current Behavior:**
- Client: flags=0x0 → AVC enabled (SMALL_CACHE not set, AVC_DISABLED not set)
- Server confirms: flags=0x0 → AVC enabled

**Hypothesis:** Flags=0x0 might be valid, but we should double-check spec.

---

### Area 3: Required PDU Sequence

**MS-RDPEGFX Section 3 (Protocol Details):**

Let me fetch the exact sequence from specification...

**Expected Sequence (from FreeRDP behavior):**
```
1. Client → CapabilitiesAdvertise
2. Server → CapabilitiesConfirm
3. Server → [optional] CacheImportOffer
4. Client → [optional] CacheImportReply
5. Server → ResetGraphics (required before first frame)
6. Server → CreateSurface
7. Server → MapSurfaceToOutput (or MapSurfaceToWindow/MapSurfaceToScaledOutput)
8. Server → StartFrame + WireToSurface + EndFrame (frame data)
9. Client → FrameAcknowledge
```

**Our Sequence:**
```
1. Client → CapabilitiesAdvertise ✅
2. Server → CapabilitiesConfirm ✅ (queued & drained)
3. Server → ResetGraphics ✅ (340 bytes)
4. Server → CreateSurface ✅ (id=0, 1280×800)
5. Server → MapSurfaceToOutput ✅ (id=0, origin 0,0)
6. ❌ CLIENT DISCONNECTS (22ms later)
7. Server → StartFrame + WireToSurface + EndFrame (never received by client)
```

**Potential Issues:**
- Missing CacheImportOffer?
- Timing: All PDUs sent too quickly?
- Order: ResetGraphics, CreateSurface, MapSurface in same batch?

---

### Area 4: Windows Client Logging

**Previous Attempt:**
We tried ETW tracing but got no RDP-specific events.

**Better Approaches:**

#### Option A: FreeRDP Client with Verbose Logging

**FreeRDP (wfreerdp.exe) Advantages:**
- Open source - we can read the code
- Extensive logging capabilities
- Protocol-level debugging (/log-level:TRACE)
- Can log EGFX PDUs and decoder state
- Cross-platform (works on Windows)

**Setup Instructions:**
```powershell
# Download FreeRDP for Windows
https://github.com/FreeRDP/FreeRDP/releases

# Or build from source for maximum logging
git clone https://github.com/FreeRDP/FreeRDP.git
cd FreeRDP
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Debug -DWITH_DEBUG_ALL=ON
cmake --build . --config Debug

# Run with verbose logging
wfreerdp.exe /v:192.168.10.205 /u:test /p:test /log-level:TRACE /log-filters:*:TRACE 2> rdp-debug.log

# EGFX-specific filters
wfreerdp.exe /v:192.168.10.205 /u:test /p:test /log-level:TRACE /log-filters:com.freerdp.channels.rdpgfx:TRACE

# Capture to file
wfreerdp.exe /v:192.168.10.205 /u:test /p:test +log-level:DEBUG > freerdp-output.txt 2>&1
```

**What We'll See:**
- Every EGFX PDU received
- Capability negotiation details
- Surface creation attempts
- Decoder initialization
- **Exact error messages** from decoder
- Frame processing state

#### Option B: mstsc with Registry Logging

**Enable detailed RDP client logging:**
```cmd
REM Run as Administrator
reg add "HKLM\SOFTWARE\Microsoft\Terminal Server Client" /v LogLevel /t REG_DWORD /d 3 /f

REM Enable EGFX-specific logging
reg add "HKLM\SOFTWARE\Policies\Microsoft\Windows NT\Terminal Services" /v fEnableLog /t REG_DWORD /d 1 /f

REM Set log location
reg add "HKLM\SOFTWARE\Microsoft\Terminal Server Client" /v LogLocation /t REG_SZ /d "C:\RDPLogs" /f
mkdir C:\RDPLogs

REM Connect with mstsc, then check:
C:\RDPLogs\*.log
%TEMP%\mstsc\*.log
%LOCALAPPDATA%\Microsoft\Terminal Server Client\*.etl
```

#### Option C: Wireshark/tcpdump Packet Capture

```bash
# On server (192.168.10.205)
sudo tcpdump -i any -w /tmp/rdp-capture.pcap port 3389

# Or on Windows client
# Use Wireshark with RDP dissector
# Filter: tcp.port == 3389
```

**Benefits:**
- See exact bytes on wire
- Verify CapabilitiesConfirm is actually transmitted
- Check if client sends any response before disconnecting
- Timing analysis

---

### Area 5: MS-RDPEGFX Specification Review

**Sections to Review:**

1. **Section 1.3.1 - Initialization and Sequencing**
   - Required PDU sequence
   - Timing requirements
   - Capability exchange details

2. **Section 2.2.2.2 - RDPGFX_CAPS_CONFIRM_PDU**
   - Structure requirements
   - Valid flag combinations
   - Version-specific requirements

3. **Section 2.2.3.9 - RDPGFX_CAPSET_VERSION106**
   - V10_6-specific requirements
   - Flag meanings and requirements
   - Compatibility notes

4. **Section 2.2.4.1 - RDPGFX_RESET_GRAPHICS_PDU**
   - When it must be sent
   - What can follow it
   - Timing constraints

5. **Section 3.3 - Server-Side Message Processing**
   - State machine
   - Required acknowledgments
   - Error conditions

**Action:** Need to access the PDF spec for detailed review.

---

## Immediate Next Steps

### Step 1: Test with Enhanced Logging (5 min)

Run server with new debug logging:
```bash
~/run-server.sh
```

**Look for:**
```bash
# After test:
grep "SVC process returned.*EGFX\|Writing SVC response to wire.*CapabilitiesConfirm\|SVC response written" log

# Should see:
# "SVC process returned response PDUs (includes EGFX CapabilitiesConfirm if DRDYNVC)"
# "Writing SVC response to wire (CapabilitiesConfirm goes through here!)"
# "SVC response written to wire successfully"
```

**If these appear:** CapabilitiesConfirm is being sent
**If missing:** CapabilitiesConfirm is NOT reaching the wire (critical bug!)

### Step 2: Set Up FreeRDP Client on Windows (15 min)

**Download:**
https://github.com/FreeRDP/FreeRDP/releases/latest
- Get `FreeRDP-<version>-win64.zip`
- Extract to `C:\FreeRDP\`

**Test Connection:**
```cmd
cd C:\FreeRDP\bin
wfreerdp.exe /v:192.168.10.205 /u:test /p:test /cert:ignore
```

**Enable TRACE Logging:**
```cmd
wfreerdp.exe /v:192.168.10.205 /u:test /p:test /cert:ignore ^
  /log-level:TRACE ^
  /log-filters:com.freerdp.channels.rdpgfx:TRACE ^
  > C:\rdp-trace.log 2>&1
```

**Analyze Output:**
```powershell
# Search for errors
Select-String -Path C:\rdp-trace.log -Pattern "error|ERROR|fail|FAIL"

# Search for EGFX events
Select-String -Path C:\rdp-trace.log -Pattern "EGFX|rdpgfx|Capabilities|Surface|AVC"

# Check decoder initialization
Select-String -Path C:\rdp-trace.log -Pattern "decoder|h264|avc420"
```

### Step 3: Check V10_6 Flags Requirement (10 min)

**Questions to Answer:**
1. Can V10_6 have flags=0x0, or must SMALL_CACHE (0x02) be set?
2. Should we confirm client's flags or our preferred flags?
3. Is there a flag combination that breaks compatibility?

**Code Review:**
```rust
// Check negotiate_capabilities() in ironrdp-egfx/src/server.rs:459-475
// Currently returns CLIENT's capability set
// Should it merge flags or use server's preferred flags?
```

**FreeRDP Comparison:**
```c
// Check how FreeRDP negotiates and confirms capabilities
// Does it use client flags or server flags in confirm?
```

### Step 4: Packet Capture Analysis (10 min)

**Capture on Server:**
```bash
ssh greg@192.168.10.205
sudo tcpdump -i any -s 65535 -w /tmp/rdp-egfx.pcap 'port 3389'
# Connect with Windows client
# Ctrl+C when it crashes
scp /tmp/rdp-egfx.pcap local-machine:/tmp/
```

**Analyze with Wireshark:**
- Open `/tmp/rdp-egfx.pcap`
- Filter: `tcp.port == 3389`
- Follow TCP stream
- Look for:
  - DRDYNVC Data PDUs
  - EGFX PDU structures
  - CapabilitiesConfirm presence
  - Client's last message before disconnect

### Step 5: Review MS-RDPEGFX Specification (20 min)

**Critical Sections:**

**3.3.5.1.1 Server Initialization:**
> "Upon receiving the RDPGFX_CAPS_ADVERTISE_PDU message from the client,
> the server MUST send an RDPGFX_CAPS_CONFIRM_PDU message."

**3.3.5.1.2 Capability Negotiation:**
> "The server MUST select one of the capability sets advertised by the client."
> "The server MUST NOT modify the flags of the selected capability set."

**Key Rule:** We must use CLIENT's flags, not our preferred flags! ✅ (We're doing this)

**3.3.5.2 Graphics Output:**
> "Before sending any graphics commands, the server MUST send an
> RDPGFX_RESET_GRAPHICS_PDU message."

**Questions from Spec:**
1. Must CapabilitiesConfirm be sent immediately or can it be async?
2. Can surface PDUs be sent in same batch as CapabilitiesConfirm?
3. Are there timing requirements between PDUs?

---

## FreeRDP Client - Detailed Setup

### Windows Installation

**Pre-built Binaries:**
1. Visit: https://github.com/FreeRDP/FreeRDP/releases
2. Download: `FreeRDP-3.x.x-win64.zip` (latest)
3. Extract to: `C:\FreeRDP\`
4. Add to PATH: `C:\FreeRDP\bin`

**Or Build from Source (for debug symbols):**
```powershell
# Install dependencies
choco install cmake git visualstudio2022buildtools

# Clone and build
git clone https://github.com/FreeRDP/FreeRDP.git C:\FreeRDP-src
cd C:\FreeRDP-src
mkdir build
cd build

cmake .. -G "Visual Studio 17 2022" -A x64 ^
  -DCMAKE_BUILD_TYPE=Debug ^
  -DWITH_DEBUG_ALL=ON ^
  -DWITH_CLIENT=ON ^
  -DWITH_SERVER=OFF

cmake --build . --config Debug

# Binary at: build\client\Windows\Debug\wfreerdp.exe
```

### Logging Configuration

**Environment Variables:**
```cmd
set WLOG_LEVEL=TRACE
set WLOG_APPENDER=FILE
set WLOG_FILEAPPENDER_OUTPUT_FILE_PATH=C:\freerdp-debug.log
set WLOG_FILEAPPENDER_OUTPUT_FILE_NAME=wfreerdp
```

**Command-Line Logging:**
```cmd
wfreerdp.exe /v:192.168.10.205:3389 ^
  /u:test /p:test ^
  /cert:ignore ^
  /log-level:TRACE ^
  /log-filters:"com.freerdp.core:TRACE,com.freerdp.channels.rdpgfx:TRACE,com.freerdp.codec.h264:TRACE" ^
  > C:\rdp-full-trace.log 2>&1
```

**Log Filters Available:**
- `com.freerdp.channels.rdpgfx` - EGFX channel
- `com.freerdp.channels.drdynvc` - DVC layer
- `com.freerdp.codec.h264` - H.264 decoder
- `com.freerdp.core.capabilities` - Capability exchange
- `*` - All modules (verbose!)

### What We'll Learn

**From FreeRDP Client Logs:**
1. Exact PDUs received (with hex dumps)
2. Capability negotiation details
3. Surface creation success/failure
4. H.264 decoder initialization
5. **Specific error messages** with context
6. Frame decode attempts

**Example Output:**
```
[TRACE][com.freerdp.channels.rdpgfx] - rdpgfx_recv_caps_confirm_pdu
[DEBUG][com.freerdp.channels.rdpgfx] - CAPS version=0x000A0006, flags=0x00000000
[TRACE][com.freerdp.channels.rdpgfx] - rdpgfx_recv_reset_graphics_pdu width=1280 height=800
[ERROR][com.freerdp.channels.rdpgfx] - Invalid surface parameters: <ACTUAL ERROR>
```

---

## Parallel Investigation Tracks

### Track 1: Server-Side Deep Logging
- ✅ Added SVC/DVC wire send logging
- ⏳ Test and verify CapabilitiesConfirm transmission
- ⏳ Add hex dump of encoded CapabilitiesConfirm
- ⏳ Log exact timing of all EGFX PDU sends

### Track 2: Client-Side Visibility
- ⏳ Set up FreeRDP client on Windows
- ⏳ Run with TRACE logging
- ⏳ Capture exact error message and context
- ⏳ Compare PDUs received vs PDUs we think we sent

### Track 3: Specification Compliance
- ⏳ Review MS-RDPEGFX sections 2.2.2.2, 2.2.3.9, 3.3.5
- ⏳ Verify V10_6 flags=0x0 is valid
- ⏳ Check for missing required PDUs
- ⏳ Validate PDU sequence and timing

### Track 4: Wire-Level Analysis
- ⏳ Packet capture with tcpdump/Wireshark
- ⏳ Verify bytes on wire match expectations
- ⏳ Check if client sends any response before disconnecting
- ⏳ Timing analysis of TCP packets

---

## Success Criteria

We've solved the issue when:
1. ✅ Windows client accepts CapabilitiesConfirm
2. ✅ Windows client accepts surface PDUs (ResetGraphics, CreateSurface, MapSurfaceToOutput)
3. ✅ Windows client processes H.264 frames
4. ✅ Windows client sends FRAME_ACKNOWLEDGE PDUs
5. ✅ Connection stable for extended period
6. ✅ Video displays correctly on Windows client

---

## Timeline Estimate

- **Enhanced Logging Test:** 5 minutes
- **FreeRDP Client Setup:** 15 minutes
- **FreeRDP Debug Session:** 10 minutes
- **Specification Review:** 20 minutes
- **Analysis & Fix:** 30-120 minutes (depends on findings)

**Total:** 1.5-3 hours to definitive answer

---

## Next Actions (In Order)

1. **Run test** with enhanced SVC/wire logging
2. **Copy log** and verify CapabilitiesConfirm wire transmission
3. **Set up FreeRDP** client on Windows
4. **Run FreeRDP** with TRACE logging against our server
5. **Analyze FreeRDP** output for exact error
6. **Review spec** for any missed requirements
7. **Fix identified issue**
8. **Build roadmap** once basic streaming works

Let's start with the test now.
