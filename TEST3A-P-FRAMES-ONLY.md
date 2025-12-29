# Test 3A: P-Frames Only (No Aux Omission) - CRITICAL TEST

**Binary MD5**: `8fc01dc2b8b7a2f7a2d40713b08ab05b`
**Config MD5**: `6c569ec1d5f2165cdaeee0b23b067879`
**Deployed**: ✅ Complete environment (binary + config.toml)
**Status**: P-FRAMES ENABLED, Aux omission DISABLED
**Run**: `ssh greg@192.168.10.205` then `~/run-server.sh`

---

## WHAT'S BEING TESTED

**Single Variable Change**: P-frames enabled (all-I workaround removed)

**Configuration**:
- P-frames: ✅ ENABLED (lines 370-371 commented)
- Aux omission: ❌ DISABLED (config: avc444_enable_aux_omission = false)
- Both streams: ALWAYS sent (no omission yet)

**Purpose**: Verify if P-frames work WITHOUT corruption

---

## CRITICAL CHECK: CORRUPTION

**This is THE test** for whether P-frame corruption issue is resolved.

**Watch for**:
- ❌ Lavender tint
- ❌ Brown artifacts
- ❌ Color corruption
- ❌ Visual artifacts during scrolling

**Previous results**: P-frames caused lavender corruption
**Current hypothesis**: Issue may or may not persist

**If corruption appears**: P-frame issue still exists, need deeper investigation
**If NO corruption**: ✅ Can proceed to Test 3B (enable omission)

---

## EXPECTED RESULTS

### Frame Pattern

**Should see**:
```
[AVC444 Frame #0] Main: IDR (~74KB), Aux: IDR (~73KB) [BOTH SENT]
[AVC444 Frame #1] Main: IDR (~74KB), Aux: IDR (~73KB) [BOTH SENT]
...
[AVC444 Frame #7] Main: IDR (~74KB), Aux: IDR (~73KB) [BOTH SENT]
[AVC444 Frame #8] Main: P (~20KB), Aux: IDR (~73KB) [BOTH SENT]  ← Main switches to P!
[AVC444 Frame #9] Main: P (~18KB), Aux: IDR (~73KB) [BOTH SENT]
...
```

**Key indicators**:
- Main should show "P" after ~8 frames (OpenH264 warmup)
- Aux will still show "IDR" (scene change detection, expected)
- All frames show "[BOTH SENT]" (omission disabled)

### Bandwidth Expectation

**Calculation**:
```
Frames 0-7: Main IDR (74KB) + Aux IDR (73KB) = 147KB/frame
Frames 8+:  Main P (20KB) + Aux IDR (73KB) = 93KB/frame

Average: (147*8 + 93*22) / 30 = 107KB/frame
Bandwidth: 107KB * 30fps = 3.14 MB/s
```

**Expected**: ~2.5-3.5 MB/s (significant reduction from 4.4 MB/s!)

**This is STILL above 2 MB/s target** because aux is always sent.
Test 3B will add omission to get below 2 MB/s.

---

## WHAT TO LOOK FOR

### In Logs

**Startup**:
```
Created AVC444 encoder: BT709 matrix, 5000kbps, level=Some(L4_0)
Config loaded: avc444_enable_aux_omission = false
```

**No "AUX OMISSION ENABLED" message** (disabled)

**Frame logging**:
```
[AVC444 Frame #8] Main: P (21345B), Aux: IDR (73456B) [BOTH SENT]
```

**Key**: Main shows "P" not "IDR" after warmup

### Performance

**Lag should reduce**:
- Test 2: 14% frame drops (4.4 MB/s too high)
- Test 3A: Should be <10% drops (~3 MB/s more manageable)

---

## SUCCESS CRITERIA

### Must Have

- ✅ Main produces P-frames (logs show "P" after frame ~8)
- ✅ **NO CORRUPTION** (critical!)
- ✅ Quality remains good
- ✅ Bandwidth ~2.5-3.5 MB/s (lower than 4.4)
- ✅ Less laggy (fewer frame drops)

### If Corruption Occurs

**Then**:
- ❌ P-frame issue NOT resolved by Phase 1 implementation
- Need deeper investigation
- May require:
  - Hardware encoders (VA-API/NVENC)
  - Different encoder (x264)
  - Alternative approach

### If NO Corruption

**Then**:
- ✅ **MAJOR BREAKTHROUGH** - P-frames work!
- Proceed to Test 3B (enable omission)
- Expected final result: <2 MB/s

---

## DEPLOYMENT COMPLETE

**Following proper workflow**:
1. ✅ Deleted old binary
2. ✅ Deleted old config
3. ✅ Copied new binary (8fc01dc2b8b7a2f7a2d40713b08ab05b)
4. ✅ Copied new config (6c569ec1d5f2165cdaeee0b23b067879)
5. ✅ Set executable
6. ✅ Verified MD5s match

**Environment**: Complete and ready

---

## WHAT TO REPORT

After testing, report:

1. **Corruption status**: Any lavender/artifacts? YES/NO
2. **Frame types**: Do logs show Main P-frames?
3. **Bandwidth feel**: Still laggy or improved?
4. **Quality**: Perfect/Good/Issues?

**This determines if we can proceed to Test 3B!**

---

**Status**: ✅ DEPLOYED - Test 3A Ready
**Critical**: Watch for P-frame corruption!
**Next**: If clean → Test 3B (enable omission)

**Run server and test now!** 🎯
