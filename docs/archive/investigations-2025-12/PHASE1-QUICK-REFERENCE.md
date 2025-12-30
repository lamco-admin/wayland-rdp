# Phase 1: Quick Reference Card

**Binary MD5**: `c3c8e95d885a34fe310993d50d59f085`
**Deployed**: ✅ Ready on 192.168.10.205
**Status**: Aux omission DISABLED (safe default)
**Run**: `ssh greg@192.168.10.205` then `~/run-server.sh`

---

## CURRENT STATE (Deployed Binary)

**Configuration**: Aux omission implemented but **DISABLED**

```rust
// src/egfx/avc444_encoder.rs line 315:
enable_aux_omission: false,   // ← CURRENTLY DISABLED
```

**What this means**:
- Behavior identical to previous (both streams always sent)
- Bandwidth: ~4.3 MB/s
- Quality: Perfect
- Purpose: Verify implementation doesn't break anything

**Test this first!** (10 minutes)

---

## TO ENABLE AUX OMISSION

### Quick Enable (Code Change)

**Edit**: `src/egfx/avc444_encoder.rs` line 315

**Change**:
```rust
enable_aux_omission: false,
```

**To**:
```rust
enable_aux_omission: true,
```

**Rebuild**:
```bash
cargo build --release --features h264
./test-kde.sh deploy  # Or manual deployment
```

**Expected**: Logs will show "Aux: OMITTED" messages

---

## TO ENABLE P-FRAMES (Full Phase 1)

**After aux omission works, also change**:

**Edit**: `src/egfx/avc444_encoder.rs` line 307

**Change**:
```rust
force_all_keyframes: false,
```

**To**:
```rust
force_all_keyframes: false,  # Already false, but remove all-I workaround below
```

**And comment out lines 370-371**:
```rust
// self.main_encoder.force_intra_frame();  // Remove this
```

**Expected**: Main uses P-frames, bandwidth drops to 0.7-1.5 MB/s

**CRITICAL**: Check for lavender corruption!

---

## WHAT TO LOOK FOR IN LOGS

### With Omission Disabled (Current)

```
[AVC444 Frame #0000] Main: IDR (74KB), Aux: IDR (73KB)
[AVC444 Frame #0001] Main: IDR (74KB), Aux: IDR (73KB)
```

No "OMITTED" messages - all frames send both streams

### With Omission Enabled (After Line 315 Change)

```
[AVC444 Frame #0000] Main: IDR (74KB), Aux: IDR (73KB) [BOTH SENT]
[AVC444 Frame #0001] Main: IDR (74KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
[AVC444 Frame #0002] Main: IDR (74KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
...
[AVC444 Frame #0030] Main: IDR (75KB), Aux: IDR (72KB) [BOTH SENT]  ← Forced refresh
```

Plus on first enable:
```
🎬 Phase 1 AUX OMISSION ENABLED: max_interval=30frames, force_idr_on_return=true
```

### With P-Frames Enabled (Full Phase 1)

```
[AVC444 Frame #0008] Main: P (21KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
[AVC444 Frame #0009] Main: P (19KB), Aux: OMITTED (LC=1) [BANDWIDTH SAVE]
```

**Main shows "P" instead of "IDR"** - this is the compression!

---

## BANDWIDTH MEASUREMENT

### From Logs

```bash
# After test, copy log locally
scp greg@192.168.10.205:~/colorful-test-*.log ./test-phase1.log

# Calculate bandwidth
rg "\[AVC444 Frame" test-phase1.log > frames.txt
python3 <<EOF
import re
with open('frames.txt') as f:
    data = f.read()
    # Count frames
    total = len(re.findall(r'\[AVC444 Frame', data))
    omitted = len(re.findall(r'OMITTED', data))
    sent = len(re.findall(r'BOTH SENT', data))

    print(f"Total frames: {total}")
    print(f"Aux omitted: {omitted} ({omitted/total*100:.1f}%)")
    print(f"Aux sent: {sent} ({sent/total*100:.1f}%)")

    # Calculate average size
    sizes = [int(m.group(1)) for m in re.finditer(r'Main:.*?\((\d+)B\)', data)]
    if sizes:
        avg = sum(sizes) / len(sizes)
        print(f"Average main size: {avg/1024:.1f} KB")
        print(f"Bandwidth @ 30fps: {avg*30/(1024*1024):.2f} MB/s")
EOF
```

---

## TESTING CHECKLIST

### Test 1: Disabled (Current Binary)

- [ ] Deploy current binary
- [ ] Run `~/run-server.sh`
- [ ] Connect via RDP
- [ ] Verify perfect quality
- [ ] Check bandwidth ~4.3 MB/s
- [ ] No "OMITTED" in logs

**Duration**: 10 minutes

### Test 2: Enabled (All-I Mode)

- [ ] Edit line 315 → `true`
- [ ] Rebuild and deploy
- [ ] Run server
- [ ] Logs show "🎬 Phase 1 AUX OMISSION ENABLED"
- [ ] Logs show "OMITTED" messages
- [ ] Quality still perfect
- [ ] No corruption

**Duration**: 15 minutes

### Test 3: Full Phase 1 (P-Frames)

- [ ] Also remove all-I workaround (lines 370-371)
- [ ] Rebuild and deploy
- [ ] **CRITICAL**: Watch for lavender corruption
- [ ] If clean: Measure bandwidth
- [ ] Should be <2 MB/s

**Duration**: 30 minutes + extended test

---

## QUICK COMMANDS

```bash
# Deploy current (omission disabled)
# Already done - just run server

# Enable omission and redeploy
sed -i 's/enable_aux_omission: false/enable_aux_omission: true/' src/egfx/avc444_encoder.rs
cargo build --release --features h264
ssh greg@192.168.10.205 "rm -f ~/lamco-rdp-server"
scp target/release/lamco-rdp-server greg@192.168.10.205:~/
ssh greg@192.168.10.205 "chmod +x ~/lamco-rdp-server && md5sum ~/lamco-rdp-server"

# Then SSH and run
ssh greg@192.168.10.205
~/run-server.sh
```

---

## SUCCESS CRITERIA

**Phase 1A** (disabled): ✅ No regression
**Phase 1B** (enabled, all-I): ✅ Omission works, no corruption
**Phase 1C** (enabled, P-frames): ✅ <2 MB/s + no corruption = **SUCCESS!**

---

## DOCUMENTS TO READ

1. **PHASE1-DEPLOYMENT-GUIDE.md** - Detailed testing instructions
2. **ULTIMATE-AVC444-CAPABILITY-PLAN.md** - Complete roadmap (Phase 2-4)
3. **COMPREHENSIVE-RESEARCH-FINDINGS-2025-12-29.md** - Research background

---

**Deployed**: ✅ Yes (`c3c8e95d885a34fe310993d50d59f085`)
**Ready**: ✅ Run `~/run-server.sh` on test VM
**Next**: Test, enable omission, test P-frames, measure bandwidth

**Phase 1 is DEPLOYED and ready to test!** 🚀
