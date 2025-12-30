# Phase 2: THE CRITICAL TEST - Single Encoder P-Frames

**Date**: 2025-12-29 02:10 UTC
**Binary MD5**: `17da98ec8c23fe05d38f377cbd4aee05`
**Status**: DEPLOYED - Ready for testing
**This is IT**: Will single encoder architecture fix P-frame corruption?

---

## WHAT CHANGED IN PHASE 2

### Phase 1 → Phase 2 Differences

**Phase 1** (just tested, worked perfectly):
- ✅ Single encoder structure
- ✅ All-I frames (safe mode)
- ✅ Validated architecture

**Phase 2** (this test):
- ✅ Single encoder structure (same)
- 🚀 **P-FRAMES ENABLED** (the critical change!)
- 🔬 NUM_REF = 2 (DPB holds 2 reference frames)
- 📊 NAL instrumentation (logs what references are used)

---

## THE THEORY

### Why This Should Work

**Encoding Sequence**:
```
Frame 0: Main IDR → DPB[0]
Frame 0: Aux IDR → DPB[1]
Frame 1: Main P → DPB[2] (evicts oldest, probably DPB[0])
         Motion search in DPB finds Main frame 0 (best match)
         Uses Main(0) as reference ✓
Frame 1: Aux P → DPB[3]
         Motion search in DPB finds Aux frame 0 (best match)
         Uses Aux(0) as reference ✓
```

**Natural Reference Selection**:
- OpenH264's motion search looks for best matching blocks
- Main frames match Main frames (luma patterns)
- Aux frames match Aux frames (chroma patterns)
- **Should automatically select correct references!**

---

## WHAT TO LOOK FOR

### Primary: Corruption Status

**SUCCESS** ✅:
- NO lavender/brown corruption
- Text readable during scrolling
- Windows move smoothly
- Colors correct everywhere
- **Single encoder + multi-ref DPB SOLVED IT!**

**PARTIAL** ⚠️:
- LESS corruption than before
- But still some artifacts
- Check logs to see reference behavior

**FAILURE** ❌:
- SAME extensive corruption
- Single encoder didn't help
- Need to check NAL logs for why

### Secondary: Performance

- Bandwidth lower than all-I? (expect ~1.4 MB/s vs 4.3 MB/s)
- Responsiveness good?
- Any stuttering?

---

## NAL LOG ANALYSIS

### After test, I'll check logs for:

```bash
# Copy log
scp greg@192.168.10.205:~/colorful-test-TIMESTAMP.log ./phase2-test.log

# Check NAL structure
rg "Frame #.*NAL" phase2-test.log | head -100

# Look for reference patterns
rg "REFERENCE|NON-REF" phase2-test.log | head -50

# Check frame types
rg "\[AVC444 Frame" phase2-test.log | head -30
```

### Expected NAL Pattern (if working correctly)

```
[Frame #0 MAIN NAL#0] type= 7 (SPS) ref_idc=3 (REFERENCE(3))
[Frame #0 MAIN NAL#1] type= 8 (PPS) ref_idc=3 (REFERENCE(3))
[Frame #0 MAIN NAL#2] type= 5 (IDR) ref_idc=3 (REFERENCE(3))

[Frame #0 AUX NAL#0] type= 5 (IDR) ref_idc=3 (REFERENCE(3))

[Frame #1 MAIN NAL#0] type= 1 (P-slice) ref_idc=2 (REFERENCE(2))
[Frame #1 AUX NAL#0] type= 1 (P-slice) ref_idc=? (???)
```

**Critical Questions**:
1. Is Aux marked as REFERENCE or NON-REF?
2. Are both Main and Aux using P-slices after frame 0?
3. Do frame sizes look reasonable?

---

## THREE POSSIBLE OUTCOMES

### Outcome A: SUCCESS - No Corruption ✅

**Means**:
- Single encoder architecture WAS the solution
- Natural reference selection works
- Multi-ref DPB (NUM_REF=2) is sufficient
- **PROBLEM SOLVED!**

**Next Steps**:
- Document the solution
- Commit Phase 2
- Optimize if needed
- Consider aux non-reference for extra robustness

---

### Outcome B: PARTIAL - Reduced Corruption ⚠️

**Means**:
- Single encoder helps but isn't complete solution
- Reference behavior might be mostly correct
- Need fine-tuning

**Next Steps**:
- Analyze NAL logs to see reference patterns
- Try NUM_REF = 4 (more DPB slots)
- Consider making Aux non-reference
- Investigate remaining issues

---

### Outcome C: FAILURE - Same Corruption ❌

**Means**:
- Single encoder alone isn't enough
- Need additional measures

**Next Steps**:
- Analyze NAL logs carefully
- Check if Main refs Aux (problem)
- Check if Aux marked as reference (might need to disable)
- Implement Phase 3 (aux non-reference or other strategies)

---

## WHAT THE LOGS WILL REVEAL

### Scenario 1: Aux is NON-REF

```
[Frame #1 AUX NAL#0] type= 1 (P-slice) ref_idc=0 (NON-REF)
```

**This is IDEAL**:
- Aux provides chroma detail but doesn't pollute reference chain
- Main can't accidentally reference Aux
- Should eliminate corruption

---

### Scenario 2: Aux is REFERENCE

```
[Frame #1 AUX NAL#0] type= 1 (P-slice) ref_idc=2 (REFERENCE(2))
```

**This could be OK or problematic**:
- If motion search still prefers correct matches → OK
- If Main sometimes refs Aux → Problem
- Need to see corruption status to know

---

### Scenario 3: Something Unexpected

Logs show unusual pattern → deeper investigation needed

---

## PHASE 2 COMPLETION CRITERIA

**PASS**:
- [ ] No lavender corruption
- [ ] NAL logs show sane reference behavior
- [ ] Bandwidth <2 MB/s (P-frames compressing)
- [ ] Quality perfect

**INVESTIGATE**:
- [ ] Reduced but not eliminated corruption
- [ ] Analyze NAL logs for patterns
- [ ] Decide on Phase 3 approach

**FAIL**:
- [ ] Same corruption as before
- [ ] NAL logs show problematic references
- [ ] Need Phase 3 (aux non-reference strategy)

---

## READY FOR TEST

**Binary deployed**: `greg@192.168.10.205:~/lamco-rdp-server`
**MD5**: `17da98ec8c23fe05d38f377cbd4aee05`

**Changes from Phase 1**:
- ❌ Removed: force_intra_frame() calls (P-frames now active!)
- ✅ Added: NUM_REF = 2 configuration
- ✅ Added: NAL structure logging

**Test it thoroughly**:
- Scroll terminal text (watch for lavender)
- Move windows (watch for corruption)
- Right-click menus
- General interaction

**This is the moment of truth!**
