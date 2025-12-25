# FreeRDP Windows Client Setup for EGFX Debugging

**Purpose:** Get detailed client-side error messages and protocol trace
**Date:** 2025-12-24

---

## Why FreeRDP Client?

**Advantages over mstsc:**
- Open source - we can read decoder code
- Extensive logging (`/log-level:TRACE`)
- Protocol-level debugging
- EGFX-specific filters
- Actual error messages instead of error codes
- Can be built in debug mode with assertions

---

## Quick Setup (Pre-built Binaries)

### Step 1: Download

Visit: https://github.com/FreeRDP/FreeRDP/releases/latest

Download: `FreeRDP-<version>-win64.zip`

Example (as of Dec 2024):
- `FreeRDP-3.10.2-win64.zip` or similar

### Step 2: Extract

```cmd
REM Create directory
mkdir C:\FreeRDP
cd C:\FreeRDP

REM Extract zip (or use Windows Explorer)
tar -xf Downloads\FreeRDP-3.10.2-win64.zip
```

### Step 3: Test Basic Connection

```cmd
cd C:\FreeRDP\bin
wfreerdp.exe /v:192.168.10.205 /u:test /p:test /cert:ignore
```

If connection works, you should see the Linux desktop.

---

## Debug Logging Setup

### Method 1: Command-Line Logging (Easiest)

```cmd
cd C:\FreeRDP\bin

REM Full TRACE logging to file
wfreerdp.exe /v:192.168.10.205 /u:test /p:test /cert:ignore ^
  /log-level:TRACE ^
  /log-filters:com.freerdp.channels.rdpgfx:TRACE,com.freerdp.codec.h264:TRACE ^
  > C:\rdp-egfx-trace.log 2>&1
```

**Log Filters Available:**
- `com.freerdp.channels.rdpgfx` - EGFX channel (PDUs, state machine)
- `com.freerdp.channels.drdynvc` - DVC layer
- `com.freerdp.codec.h264` - H.264 decoder
- `com.freerdp.core.capabilities` - Capability negotiation
- `com.freerdp.core.connection` - Connection state
- `*` - All modules (very verbose!)

### Method 2: Environment Variables

```cmd
set WLOG_LEVEL=TRACE
set WLOG_APPENDER=FILE
set WLOG_FILEAPPENDER_OUTPUT_FILE_PATH=C:\
set WLOG_FILEAPPENDER_OUTPUT_FILE_NAME=freerdp-debug

wfreerdp.exe /v:192.168.10.205 /u:test /p:test /cert:ignore
```

Logs written to: `C:\freerdp-debug.log`

### Method 3: Focused EGFX/H.264 Debugging

```cmd
wfreerdp.exe /v:192.168.10.205 /u:test /p:test /cert:ignore ^
  /log-level:DEBUG ^
  /log-filters:com.freerdp.channels.rdpgfx:TRACE ^
  /log-filters:com.freerdp.codec:TRACE ^
  2> C:\egfx-debug.log
```

---

## What to Look For in Logs

### 1. Capability Negotiation

```
[INFO][com.freerdp.channels.rdpgfx] - rdpgfx_recv_caps_confirm_pdu
[DEBUG][com.freerdp.channels.rdpgfx] - CAPS version=0x000A0006, flags=0x00000000
```

**Check:** Does client accept our V10_6 with flags=0x0?

### 2. Surface Creation

```
[TRACE][com.freerdp.channels.rdpgfx] - rdpgfx_recv_reset_graphics_pdu width=1280 height=800
[TRACE][com.freerdp.channels.rdpgfx] - rdpgfx_recv_create_surface_pdu surface_id=0 width=1280 height=800
[TRACE][com.freerdp.channels.rdpgfx] - rdpgfx_recv_map_surface_to_output_pdu surface_id=0
```

**Check:** Do these PDUs process successfully?

### 3. H.264 Frame Processing

```
[TRACE][com.freerdp.channels.rdpgfx] - rdpgfx_recv_wire_to_surface_1_pdu
[DEBUG][com.freerdp.codec.h264] - Initializing H.264 decoder
[DEBUG][com.freerdp.codec.h264] - SPS: profile=66 level=32 width=1280 height=800
[ERROR][com.freerdp.codec.h264] - H.264 decoder error: <ACTUAL ERROR MESSAGE>
```

**Check:** What exact error does the decoder report?

### 4. Error Conditions

```
[ERROR][com.freerdp.channels.rdpgfx] - <ERROR MESSAGE>
[WARN][com.freerdp.channels.rdpgfx] - <WARNING MESSAGE>
```

**Critical:** The ERROR/WARN messages will tell us EXACTLY what's wrong!

---

## Advanced: Build from Source (Debug Mode)

**When to use:** If pre-built binaries don't give enough detail.

### Prerequisites

```cmd
choco install cmake git visualstudio2022buildtools visualstudio2022-workload-vctools
```

Or install Visual Studio 2022 Community with "Desktop development with C++"

### Clone and Build

```cmd
git clone https://github.com/FreeRDP/FreeRDP.git C:\FreeRDP-src
cd C:\FreeRDP-src
mkdir build
cd build

cmake .. -G "Visual Studio 17 2022" -A x64 ^
  -DCMAKE_BUILD_TYPE=Debug ^
  -DWITH_DEBUG_ALL=ON ^
  -DWITH_CLIENT=ON ^
  -DWITH_SERVER=OFF ^
  -DWITH_CHANNELS=ON ^
  -DWITH_CODEC_H264=ON

cmake --build . --config Debug
```

**Binary location:** `C:\FreeRDP-src\build\client\Windows\Debug\wfreerdp.exe`

**Benefits:**
- Debug symbols
- Assertions enabled
- More detailed error messages
- Can attach debugger if needed

---

## Testing Protocol

### Test 1: Basic EGFX Connection

```cmd
wfreerdp.exe /v:192.168.10.205 /u:test /p:test /cert:ignore ^
  /log-level:DEBUG ^
  /log-filters:com.freerdp.channels.rdpgfx:TRACE ^
  > C:\test1-egfx.log 2>&1
```

**Look for:**
- Capability exchange completion
- Surface creation success/failure
- First error message

### Test 2: Full Protocol Trace

```cmd
wfreerdp.exe /v:192.168.10.205 /u:test /p:test /cert:ignore ^
  /log-level:TRACE ^
  /log-filters:*:TRACE ^
  > C:\test2-full-trace.log 2>&1
```

**Look for:**
- Complete PDU sequence
- All warnings and errors
- Exact failure point

### Test 3: H.264 Decoder Focus

```cmd
wfreerdp.exe /v:192.168.10.205 /u:test /p:test /cert:ignore ^
  /log-level:TRACE ^
  /log-filters:com.freerdp.codec.h264:TRACE,com.freerdp.codec:DEBUG ^
  > C:\test3-h264-decoder.log 2>&1
```

**Look for:**
- Decoder initialization
- SPS/PPS processing
- Level/profile validation
- IDR frame decode attempt

---

## Analyzing Logs

### Search Patterns

```powershell
# Find errors
Select-String -Path C:\rdp-egfx-trace.log -Pattern "error|ERROR|fail|FAIL|invalid|INVALID"

# Find EGFX events
Select-String -Path C:\rdp-egfx-trace.log -Pattern "rdpgfx|RDPGFX|egfx|EGFX"

# Find capability exchange
Select-String -Path C:\rdp-egfx-trace.log -Pattern "caps|CAPS|Capabilit"

# Find H.264 events
Select-String -Path C:\rdp-egfx-trace.log -Pattern "h264|H264|avc|AVC|decoder|sps|pps|idr"

# Find surface operations
Select-String -Path C:\rdp-egfx-trace.log -Pattern "surface|Surface|SURFACE|reset.*graphics"

# Show context around errors
Select-String -Path C:\rdp-egfx-trace.log -Pattern "error" -Context 10
```

### Key Information to Extract

1. **Does client accept CapabilitiesConfirm?**
   - Look for: "rdpgfx_recv_caps_confirm" success/failure

2. **Does client accept surface PDUs?**
   - Look for: "rdpgfx_recv_reset_graphics" / "create_surface" / "map_surface"

3. **Does H.264 decoder initialize?**
   - Look for: "H.264 decoder init" or "h264_context_new"

4. **What error does decoder report?**
   - Look for: Exact error message with code/reason

5. **Is it the SPS parameters?**
   - Look for: "SPS" messages showing what decoder received

---

## Expected Outcomes

### Scenario A: Level Constraint Error

```
[ERROR][com.freerdp.codec.h264] - SPS specifies Level 3.2 but stream requires Level 4.0
[ERROR][com.freerdp.codec.h264] - Macroblock rate 120,000 exceeds Level 3.2 limit 108,000
```

**Solution:** Configure OpenH264 to use Level 4.0

### Scenario B: SPS Parameter Error

```
[ERROR][com.freerdp.codec.h264] - Invalid SPS parameters
[ERROR][com.freerdp.codec.h264] - Unsupported profile/level combination
```

**Solution:** Adjust OpenH264 encoder configuration

### Scenario C: Surface Parameter Error

```
[ERROR][com.freerdp.channels.rdpgfx] - Invalid surface dimensions
[ERROR][com.freerdp.channels.rdpgfx] - CreateSurface failed: <reason>
```

**Solution:** Fix surface creation parameters

### Scenario D: Capability Mismatch

```
[ERROR][com.freerdp.channels.rdpgfx] - CapabilitiesConfirm version mismatch
[ERROR][com.freerdp.channels.rdpgfx] - Invalid capability flags
```

**Solution:** Fix capability negotiation

---

## Alternative: xfreerdp (Linux Client)

If you have a Linux desktop available for testing:

```bash
# Install FreeRDP
sudo apt install freerdp2-x11

# Run with logging
WLOG_LEVEL=TRACE xfreerdp /v:192.168.10.205 /u:test /p:test /cert:ignore \
  /log-level:TRACE \
  /log-filters:com.freerdp.channels.rdpgfx:TRACE \
  2>&1 | tee rdp-trace.log
```

---

## Next Actions

1. **Install FreeRDP client** on Windows machine
2. **Run with TRACE logging** against our server
3. **Capture logs** (especially first error message)
4. **Share logs** for analysis
5. **Fix identified issue** based on actual error
6. **Test with mstsc** to confirm fix works for Windows too

This is the definitive diagnostic step - FreeRDP will tell us exactly what's wrong instead of just crashing with error 0x1108.
