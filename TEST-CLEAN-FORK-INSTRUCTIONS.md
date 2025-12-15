# Test Clean Fork Build
## Verify Clipboard Functionality
**Binary:** wrd-server-clean-fork
**Date:** 2025-12-10

---

## DEPLOYED BINARIES

**VM 192.168.10.3 (Primary - KDE Plasma):**
```
~/wayland/wrd-server-specs/target/release/wrd-server-clean-fork
```

**VM 192.168.10.205 (Secondary - GNOME):**
```
~/wayland/wrd-server-specs/target/release/wrd-server-clean-fork
```

---

## WHAT THIS BINARY IS

**IronRDP Source:**
- Devolutions/IronRDP master (official upstream)
- + 1 commit: Server clipboard initiation patch
- NO allan2 intermediary fork
- NO debug logging commits

**Key Difference:**
- Old binary: 23 commits divergence from Devolutions
- New binary: 1 commit divergence from Devolutions

**What to Verify:**
- Clipboard still works (our patch is present)
- No regressions (Devolutions master is stable)
- Same performance (should be identical or better)

---

## TEST PROCEDURE

### VM 192.168.10.3 (KDE)

```bash
# SSH to VM
ssh greg@192.168.10.3

# Switch to console (Ctrl+Alt+F2 or use GUI)
cd ~/wayland/wrd-server-specs

# Run clean fork binary
./target/release/wrd-server-clean-fork -c config.toml 2>&1 | tee test-clean-fork.log
```

**Test Checklist:**
- [ ] Video displays (not black screen)
- [ ] Mouse moves smoothly
- [ ] Keyboard types responsively
- [ ] **Linux → Windows clipboard** (copy text in Linux, paste in Windows)
- [ ] **Windows → Linux clipboard** (copy text in Windows, paste in Linux)
- [ ] Single paste (not 10x duplication)
- [ ] No crashes

---

### VM 192.168.10.205 (GNOME) - If Available

```bash
ssh greg@192.168.10.205
cd ~/wayland/wrd-server-specs
./target/release/wrd-server-clean-fork -c config.toml 2>&1 | tee test-clean-fork-gnome.log
```

**Same checklist as KDE**

---

## WHAT TO CHECK IN LOGS

**Verify clipboard patch active:**
```bash
grep "SERVER initiate_copy" test-clean-fork.log
```

**Should see:**
```
SERVER initiate_copy: sending FormatList (state=Ready, X formats)
```

**This confirms our patch is present and working.**

---

## SUCCESS CRITERIA

**If clipboard works both directions:**
- ✅ Clean fork is functional
- ✅ Can adopt permanently
- ✅ Delete old 23-commit branch
- ✅ Simplified maintenance going forward

**If clipboard fails or regresses:**
- ❌ Investigate what's missing
- ⚠️ May need something from allan2 branch after all
- 🔍 Compare logs with old binary

---

## COMPARISON TEST (Optional)

**Run old binary for comparison:**
```bash
# Build old version
cd /home/greg/wayland/wrd-server-specs
git stash  # Save Cargo.toml changes
# (Revert to old branch in Cargo.toml)
cargo build --release
scp target/release/wrd-server greg@192.168.10.3:.../wrd-server-old-fork

# Test both side-by-side
# Look for any behavioral differences
```

**But likely not needed** - build succeeded, patch is present.

---

**READY FOR TESTING**

Binary deployed to 192.168.10.3 as `wrd-server-clean-fork`.
Run from VM console and test clipboard functionality.
