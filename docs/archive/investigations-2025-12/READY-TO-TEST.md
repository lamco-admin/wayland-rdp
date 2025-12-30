# Ready to Test - Enhanced Diagnostics Deployed ✅

**Date**: 2025-12-27
**Status**: All diagnostic code deployed and script configured

---

## What's Deployed

### Binary
- **Location**: `greg@192.168.10.205:~/lamco-rdp-server`
- **MD5**: `4145bfac348192ec44b0ba57c9cbadef`
- **Features**: Multi-position color sampling at 5 screen positions

### Run Script
- **Location**: `greg@192.168.10.205:~/run-server.sh`
- **Settings**:
  - Log level: `-vvv` (maximum verbosity)
  - Log file: `colorful-test-YYYYMMDD-HHMMSS.log`
  - Console output: Also logged to `console-output.log`

---

## How to Test (Step by Step)

### 1. Prepare Colorful Screen Content

**SSH to test server:**
```bash
ssh greg@192.168.10.205
```

**Open colorful windows** (desktop background is gray - we need colors!):
- **Web browser** with a colorful webpage (Google, news site with images)
- **Image viewer** with a bright photo (sunset, flowers, anything vivid)
- **Color picker** application or system settings
- **Terminal** with syntax highlighting or colored prompts

**Arrange windows** so they're visible on screen (not minimized)

### 2. Run the Server

Simply execute:
```bash
cd ~
./run-server.sh
```

The script will:
- Display a reminder to open colorful windows
- Check prerequisites (D-Bus, PipeWire, certs)
- Show binary MD5 checksum
- Start server with `-vvv` (TRACE level logging)
- Save logs to `colorful-test-YYYYMMDD-HHMMSS.log`

### 3. Connect from Windows RDP Client

Connect to: `192.168.10.205:3389`

Let the connection run for a few seconds so it captures some frames.

### 4. Stop and Retrieve Logs

**Press Ctrl+C** to stop the server

**Retrieve the log file:**
```bash
# From your local machine
scp greg@192.168.10.205:~/colorful-test-*.log ./
```

---

## What to Look For in Logs

### Section 1: Color Conversion Samples
```
🎨 COLOR CONVERSION SAMPLES (Matrix::BT709):
  center (640,400): BGRA=(XXX,XXX,XXX) → YUV=(YYY,UUU,VVV)
  ...
```

**Check**:
- Are U and V values varying across positions? (not all 128)
- Do U/V values make sense for the colors?
  - Red: V>128, U<128
  - Blue: U>128, V<128
  - Green: U<128, V<128
  - Gray: U≈V≈128

### Section 2: Main View Analysis
```
═══ MAIN VIEW MULTI-POSITION ANALYSIS ═══
📍 Center @ (640, 400)
  Y444: [YYY,YYY] [YYY,YYY]
  U444: [UUU,UUU] [UUU,UUU]
  V444: [VVV,VVV] [VVV,VVV]
  Main U420: UUU, Main V420: VVV
```

**Check**:
- Does Main U420/V420 match the average of U444/V444 2×2 block?
- This verifies 4:2:0 subsampling is working correctly

### Section 3: Auxiliary View Analysis
```
═══ AUXILIARY VIEW MULTI-POSITION ANALYSIS ═══
📍 Center @ (640, 400)
  Aux Y (row 400): [AAA,AAA,AAA,AAA]
  Source U444[row 401]: [UUU,UUU,UUU,UUU]
  Aux U420: UUU, Aux V420: VVV
  Source U444[1,0]: UUU, V444: VVV
```

**Check**:
- Does Aux Y match source U444 odd rows (for rows 0-7 of macroblock)?
- Does Aux Y match source V444 odd rows (for rows 8-15 of macroblock)?
- Does Aux U/V match source U444/V444 at odd columns, even rows?

### Temporal Stability Check
```
✅ TEMPORAL STABLE: Auxiliary IDENTICAL (hash: 0x...)
```
or
```
⚠️  TEMPORAL CHANGE: Auxiliary DIFFERENT (prev: 0x..., curr: 0x...)
```

**Check**: Static screen should show TEMPORAL STABLE (indicates no P-frame corruption)

---

## If Logs Show All U=V=128

This means the screen content is gray (no colors captured).

**Solution**:
1. Make sure colorful windows are **visible** (not minimized)
2. Try moving mouse over colorful areas to trigger damage detection
3. Open even more vibrant content (bright red/blue/green)
4. Rerun the test

---

## Next Steps After Log Analysis

Once we have logs with actual color data (U/V ≠ 128), we can:

1. **Identify color patterns** - Which colors are wrong and how?
2. **Check for U/V swap** - Do reds look blue-ish?
3. **Verify auxiliary packing** - Is it capturing the right residual chroma?
4. **Compare with AVC420** - Same input, different chroma handling
5. **Test VUI parameters** - Try different color ranges/matrices

---

## Files for Reference

- `TESTING-SESSION-2025-12-27.md` - Detailed testing guide
- `START-HERE-NOW.md` - Current status and next steps
- `QUICK-STATUS.md` - Quick reference from previous session

---

**Ready to go!** Just run `./run-server.sh` on the test server with colorful windows open. 🎨
