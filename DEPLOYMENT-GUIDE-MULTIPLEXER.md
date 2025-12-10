# Deployment Guide: Phase 1 Multiplexer Build

## Quick Deploy (Recommended)

### Step 1: Copy Binary to VM
```bash
# From dev machine
cd /home/greg/wayland/wrd-server-specs
scp target/release/wrd-server greg@192.168.10.3:~/wayland/wrd-server-specs/target/release/wrd-server
```

### Step 2: Copy Test Script to VM
```bash
scp run-test-multiplexer.sh greg@192.168.10.3:~/wayland/wrd-server-specs/
```

### Step 3: Run on VM (via console, not SSH)
```bash
# On the VM's console (KDE desktop)
cd ~/wayland/wrd-server-specs
./run-test-multiplexer.sh
```

The script will:
- Kill any existing wrd-server instances
- Run the new build
- Create timestamped log file
- Display test instructions

---

## Alternative: Manual Testing

If you prefer manual control:

```bash
# On VM console
cd ~/wayland/wrd-server-specs
pkill -f wrd-server
./target/release/wrd-server -c config.toml 2>&1 | tee test-multiplexer.log
```

---

## What to Look For in Logs

### 1. Graphics Queue Initialization
```
INFO Graphics queue created (Phase 1 multiplexer): bounded to 4 frames with drop policy
INFO Graphics drain task started (coalescing enabled)
INFO 🎬 Graphics drain task started (Phase 1 multiplexer)
```

### 2. Graphics Statistics (every 100 frames)
```
INFO 📊 Graphics coalescing: 123 frames coalesced total
DEBUG 📊 Graphics drain stats: received=500, coalesced=123, sent=377
```

**Interpretation:**
- `received`: Frames from display handler
- `coalesced`: Frames discarded (newer available)
- `sent`: Frames actually sent to IronRDP
- **Coalescing = 0**: No congestion (good!)
- **Coalescing > 0**: Queue was full, frames coalesced (working as designed)

### 3. Frame Drops (if graphics queue full)
```
DEBUG Graphics queue full - frame dropped (QoS policy)
```

**Normal:** None under typical load
**Expected under heavy load:** Occasional drops during rapid window movement

### 4. Video Format Diagnostics (first 5 frames)
```
INFO 📐 Stream X format negotiated via param_changed
     Configured format: BGRx
INFO 📐 Buffer analysis frame 0:
     Size: 4096000 bytes, Width: 1280, Height: 800
     Calculated stride: 5120 bytes/row (16-byte aligned)
     Actual stride: 5120 bytes/row
     Expected buffer size: 4096000 bytes
     Buffer type: 3 (1=MemPtr, 2=MemFd, 3=DmaBuf)
     Pixel format: BGRx
     First 32 bytes (hex): [byte pattern]
```

**Check for:**
- Stride mismatch warnings
- Unexpected pixel format
- Unusual byte patterns in hex dump

---

## Testing Checklist

### Basic Functionality ✅
- [ ] Video displays (not black screen)
- [ ] Mouse cursor moves smoothly
- [ ] Keyboard input works
- [ ] Linux→Windows text copy/paste works
- [ ] Windows→Linux text copy/paste works (single paste, not 45!)

### Graphics Queue Behavior (NEW) 🎬
- [ ] No backpressure warnings (old issue eliminated)
- [ ] Frame coalescing stats appear if heavy load
- [ ] Input remains responsive during window movement
- [ ] Clipboard works during graphics activity

### Video Quality Investigation 🔍
- [ ] Horizontal lines still present? (known issue)
- [ ] Stride calculation correct in logs?
- [ ] Pixel format shown as expected?
- [ ] Hex dump shows normal byte pattern?

### Performance Under Load 🚀
Test heavy graphics (move windows rapidly):
- [ ] Input doesn't lag
- [ ] Clipboard operations complete
- [ ] Logs show frame coalescing (expected)
- [ ] No crash or freeze

---

## Log Analysis

### Good Signs ✅
```
Graphics drain stats: received=1000, coalesced=0, sent=1000
```
- No coalescing = no congestion

```
Frame rate regulation: dropped 400 frames, sent 600
```
- This is the 30 FPS regulator working (60 FPS → 30 FPS)

### Normal Under Heavy Load ⚠️
```
Graphics coalescing: 50 frames coalesced total
```
- Queue filled briefly, coalescing working

### Issues to Investigate 🔴
```
⚠️ Stride mismatch detected:
   Calculated: 5120 bytes/row
   Actual: 5632 bytes/row
```
- Possible cause of horizontal lines

```
Failed to send display update: channel closed
```
- Critical error, report immediately

---

## Comparison to Previous Build

| Feature | Previous | With Multiplexer |
|---------|----------|------------------|
| Backpressure warnings | ~143/session | 0 expected |
| Frame delivery | Blocking | Non-blocking |
| Under graphics load | May affect input | Input isolated |
| Statistics | Basic | Comprehensive |
| Diagnostics | Limited | Enhanced |

---

## Rollback Plan

If issues occur, rollback to stable:

```bash
# On VM
cd ~/wayland/wrd-server-specs
./target/release/wrd-server-stable -c config.toml
```

The `wrd-server-stable` binary from the previous session should still work.

---

## Next Steps After Testing

1. **Collect logs** from test session
2. **Review statistics** for graphics coalescing behavior
3. **Check diagnostics** for horizontal lines investigation
4. **Report findings** in session handover

If successful:
- Tag this build as stable
- Proceed with horizontal lines fix
- Plan full multiplexer (Phases 2-4)

If issues found:
- Document in new issue tracker
- Revert if critical
- Debug and iterate
