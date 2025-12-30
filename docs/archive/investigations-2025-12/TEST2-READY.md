# Test 2: Aux Omission ENABLED - Ready to Test

**Binary MD5**: `ec2afd99423d17bd988e8526b39ce19e`
**Deployed**: ✅ greg@192.168.10.205:~/lamco-rdp-server
**Status**: Aux omission **ENABLED**, All-I mode still active
**Run**: `ssh greg@192.168.10.205` then `~/run-server.sh`

---

## WHAT CHANGED FROM TEST 1

**Code Change**:
```rust
// Line 315 in src/egfx/avc444_encoder.rs:
enable_aux_omission: true,    // Changed from false
```

**What This Means**:
- Aux omission logic is now ACTIVE
- Will skip encoding aux when chroma unchanged
- Client receives LC=1 (luma only, reuse aux)

---

## WHAT TO EXPECT

### On Startup

Look for this NEW message:
```
🎬 Phase 1 AUX OMISSION ENABLED: max_interval=30frames, force_idr_on_return=true
```

**If you see this**: Aux omission is confirmed active

### During Streaming

**Frame logging will show**:

**When aux is sent** (frame 0, then every ~30 frames):
```
[AVC444 Frame #0000] Main: IDR (74KB), Aux: IDR (73KB) [BOTH SENT]
[AVC444 Frame #0030] Main: IDR (75KB), Aux: IDR (72KB) [BOTH SENT]
```

**When aux is omitted** (most frames):
```
[AVC444 Frame #0001] Main: IDR (74KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
[AVC444 Frame #0002] Main: IDR (74KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
...
[AVC444 Frame #0029] Main: IDR (74KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
```

**Expected pattern**: 29 omitted, 1 sent, repeat

### Bandwidth Expectation

**Still ~4.4 MB/s** (all-I mode active)

**Why no reduction yet?**
- Main is still IDR every frame (74KB each)
- Aux omitted most frames (saves aux bandwidth)
- But Main IDR is large, dominates bandwidth
- **Bandwidth won't drop significantly until P-frames enabled** (Test 3)

**Purpose of Test 2**: Verify omission LOGIC works correctly, not bandwidth reduction

---

## SUCCESS CRITERIA FOR TEST 2

### Must Have

- ✅ Startup shows "🎬 Phase 1 AUX OMISSION ENABLED"
- ✅ Logs show mix of `[BOTH SENT]` and `[OMITTED]`
- ✅ Aux sent approximately every 30 frames
- ✅ **No corruption** (critical!)
- ✅ Quality still perfect

### Measurements

- ⏳ Count omissions vs sent
- ⏳ Verify ~30 frame interval
- ⏳ Confirm forced refresh working
- ⏳ Check bandwidth (should be ~4-4.5 MB/s still)

---

## WHAT TO CHECK

### Visual Quality

**Test scenarios**:
1. Static screen - should be perfect
2. Scrolling text - should be readable
3. Window movement - should be smooth
4. Right-click menus - should be clear
5. **Watch for**: Any lavender tint or color artifacts

**Expected**: ✅ Perfect quality (omission shouldn't affect quality with all-I)

### Log Messages

```bash
# After running server and testing, copy log:
scp greg@192.168.10.205:~/colorful-test-*.log ./test2.log

# Check for omission:
rg "OMITTED" test2.log | wc -l
rg "BOTH SENT" test2.log | wc -l

# Should see:
# - Most frames omitted (~90%)
# - Some frames sent (~10%)
```

---

## IF ISSUES OCCUR

### No Omission Messages

**Symptom**: All frames still show `[BOTH SENT]`, no `[OMITTED]`

**Diagnosis**:
- Check if startup shows "AUX OMISSION ENABLED"
- Verify correct binary deployed (MD5: ec2afd99423d17bd988e8526b39ce19e)
- Check aux hash is changing every frame (triggering send)

### Corruption Appears

**Symptom**: Lavender artifacts with aux omission

**Action**:
- **STOP IMMEDIATELY**
- Report findings
- Revert to Test 1 (line 315 → false)

**This would be unexpected** - aux omission shouldn't cause corruption with all-I

### Quality Degradation

**Symptom**: Colors look washed out or wrong

**Diagnosis**:
- Check aux refresh interval (should send every 30 frames max)
- May need to reduce interval
- Should NOT happen with 30 frame interval

---

## AFTER TEST 2

### If Successful (Expected)

**Then proceed to Test 3**:
- Remove all-I workaround (lines 370-371)
- Enable P-frames
- **This is when we'll see bandwidth drop to <2 MB/s**

### If Issues

**Then**:
- Analyze logs exhaustively
- Diagnose root cause
- Adjust configuration or implementation

---

## QUICK REFERENCE

**Binary**: `ec2afd99423d17bd988e8526b39ce19e`
**Change**: Aux omission ENABLED
**Expected**: Omission working, quality perfect, bandwidth ~4.4 MB/s
**Duration**: 15-20 minute test
**Next**: Test 3 (P-frames) if successful

---

**Status**: ✅ DEPLOYED AND READY
**Run**: `ssh greg@192.168.10.205` then `~/run-server.sh`
**Watch**: For "🎬 Phase 1 AUX OMISSION ENABLED" message and `[OMITTED]` in frame logs

**Ready to test Test 2!** 🚀
