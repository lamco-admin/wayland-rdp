# Incremental Test Sequence - Systematic Approach

**Goal**: Test Phase 1 properly, one change at a time
**Problem**: Configuration not wired, all-I blocks omission
**Solution**: Fix config wiring, then test incrementally

---

## CURRENT STATE ANALYSIS

### What's Wrong

1. **Config.toml values not passed to encoder**
   - Code default: `enable_aux_omission: true` (line 315)
   - Config.toml: `avc444_enable_aux_omission: false`
   - **But config values never reach encoder!**
   - Encoder uses code defaults only

2. **All-I mode blocks omission**
   - Lines 370-371 force every frame to IDR
   - Logic: "Always send aux with keyframes"
   - **Result**: Aux can never be omitted until P-frames enabled

3. **Incomplete deployment**
   - Binary deployed ✅
   - Config.toml NOT deployed ❌

---

## THE PROBLEM

**Aux omission can't be properly tested until P-frames are enabled!**

**Why**: With all-I mode:
```
Every frame: Main = IDR (keyframe)
Logic: if (main_is_keyframe) return true;  // Always send aux
Result: No omission ever
```

**This means**: There's no useful "Test 2" (omission + all-I)

**We can only meaningfully test**:
- Test 1: Omission disabled, all-I ✅ (done)
- Test 3: Omission enabled, P-frames 🎯 (the real test)

---

## REVISED INCREMENTAL PLAN

### Option A: Skip to Test 3 (Recommended)

**Rationale**: Test 2 can't show omission anyway (all-I blocks it)

**Steps**:
1. Wire config properly (so we can control via config.toml)
2. Remove all-I workaround (lines 370-371)
3. Deploy everything (binary + config.toml)
4. Test once: **P-frames + aux omission together**
5. Check for corruption
6. Measure bandwidth

**Risk**: Testing two changes at once (P-frames + omission)
**Mitigation**: Both are well-understood, and we can disable omission in config if needed

### Option B: Test in Parts (More Conservative)

**Steps**:
1. Wire config properly
2. Deploy with omission DISABLED in config.toml
3. Enable P-frames only (no omission)
4. Test for corruption
5. If clean, enable omission in config.toml
6. Redeploy and test

**Benefit**: Tests P-frames alone first
**Downside**: Extra deploy cycle

---

## MY RECOMMENDATION

**Use Option B** (test P-frames alone first):

**Why**:
1. We know all-I works (proven)
2. We don't know if P-frames work yet (corruption risk)
3. Test P-frames FIRST without omission
4. If P-frames clean, THEN add omission

**Sequence**:
```
Current:       All-I, no omission     → 4.4 MB/s ✅ (proven)
Test 3A:       P-frames, no omission  → ~1.8 MB/s, check corruption
Test 3B:       P-frames + omission    → ~0.7-1.5 MB/s, final goal
```

This tests ONE variable at a time properly.

---

## IMPLEMENTATION PLAN

### Task 1: Wire Config (15 minutes)

**Need**: Pass `config.egfx` values to encoder

**Approach**: Display handler needs access to config

**Options**:
A. Add config field to WrdDisplayHandler
B. Pass config to display_handler processing loop
C. Read config from a shared Arc in display_handler

**Simplest**: Pass needed config values when creating encoder

Let me find the exact location and fix it.

### Task 2: Deploy Properly (5 minutes)

**Deploy** (following workflow):
1. Delete old binary on server
2. Copy new binary
3. **Copy config.toml to server** (currently missing!)
4. Verify both deployed
5. Check MD5s match

### Task 3: Test P-Frames Alone (Test 3A) - 15 minutes

**Config**: `avc444_enable_aux_omission = false`
**Code**: P-frames enabled (remove lines 370-371)
**Test**: Watch for corruption!
**Expected**: ~1.8-2.5 MB/s, no corruption hopefully

### Task 4: Enable Omission (Test 3B) - 10 minutes

**Config**: `avc444_enable_aux_omission = true`
**Redeploy** config.toml only (binary unchanged)
**Test**: Measure bandwidth
**Expected**: ~0.7-1.5 MB/s

---

**Status**: Creating proper incremental plan
**Next**: Wire config, then deploy systematically
