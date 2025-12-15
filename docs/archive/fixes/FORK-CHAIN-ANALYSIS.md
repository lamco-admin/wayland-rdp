# Fork Chain Analysis - Do We Need allan2?
## Understanding Devolutions → allan2 → glamberson
**Date:** 2025-12-10

---

## FORK HIERARCHY

```
Devolutions/IronRDP (official upstream)
    ↓
    └─ allan2/IronRDP fork
           ↓
           └─ glamberson/IronRDP (our fork)
                  ↓
                  └─ Branch: update-sspi-with-clipboard-fix
```

**Question:** Can we bypass allan2 and fork Devolutions directly?

---

## COMMIT ANALYSIS

### Devolutions/IronRDP master (Latest)

**Latest commit:** `0903c9ae` (Dec 9, 2025)
**SSPI Status:**
- Commit `5bd31912` - "build(deps): bump picky and sspi (#1028)"
- **Devolutions HAS updated SSPI** (merged Dec 2024)

**Key Recent Commits:**
- Dec 9: Dependency updates
- Dec 5: FFI error fix
- Dec 4: Clipboard window class fix (#1047)
- Dec 3: Clippy lint improvements
- **Nov 19: SSPI bump (#1028)** ⭐

---

### allan2/update-sspi branch

**Base:** Old (from before SSPI was merged to Devolutions)
**Commits:** 15 commits of SSPI/picky dependency work

**SSPI-related commits:**
1. `47f2985b` - chore: bump picky and sspi
2. `b7ef331c` - Use SmartCardType Default
3. `2ccd0f7a` - Switch to crates.io deps, picky 7 RC20
4. `d69e6f96` - Exact version for picky
5. `d1c538f3` - Add scard feature to connector
6. `99eb60c3` - Remove scard feature flag; use SSPI WASM fix
7. `bab7cfa9` - More cleanup
8. `72e0913b` - Use git sspi for all crate
9. `132a7ac0` - Specify branch...
10. `0d3de6a5` - Try disabling zlib for Mac
11. `5fe12fcc` - Update sspi branch
12. `e34a19ee`, `4db7550a`, `ffe810b7` - Placeholder commits (".")
13. `15cc3fa2` - Use published version

**Purpose:** Working on SSPI dependency issues (Windows authentication)

**Status:** ⚠️ **LIKELY OBSOLETE** - Devolutions merged SSPI updates in PR #1028

---

### glamberson/update-sspi-with-clipboard-fix (Our fork)

**Base:** allan2/update-sspi
**Our additions:** 8 commits

**OUR commits:**
1. `2d0ed673` - **Server clipboard initiation** ⭐ (ONLY real patch)
2-8. Debug logging commits (our debugging artifacts)

**Status:** Based on OUTDATED allan2 branch

---

## KEY FINDING: DEVOLUTIONS HAS SSPI NOW

**Devolutions Commit `5bd31912` (Nov 2024):**
```
build(deps): bump picky and sspi (#1028)
```

**This merged allan2's SSPI work!**

**Evidence:**
- Devolutions master now has updated sspi/picky versions
- allan2's update-sspi branch was working on same thing
- PR #1028 brought SSPI into Devolutions

**Implication:** **We don't need allan2's fork anymore!**

---

## CAN WE USE DEVOLUTIONS DIRECTLY?

### Test: Does Our Code Build Against Devolutions Master?

**Change needed in wrd-server Cargo.toml:**

```toml
# FROM (current):
ironrdp-server = { git = "https://github.com/glamberson/IronRDP", branch = "update-sspi-with-clipboard-fix" }

# TO (simplified):
ironrdp-server = { git = "https://github.com/glamberson/IronRDP", branch = "from-devolutions-master" }
# (new branch based directly on Devolutions, with ONLY our clipboard patch)
```

**Steps:**
1. Create branch from Devolutions/master
2. Cherry-pick ONLY clipboard patch (`2d0ed673`)
3. Test our wrd-server builds
4. If yes → Switch to this simpler fork

---

## PROPOSED CLEAN FORK STRUCTURE

```
Devolutions/IronRDP (official)
    ↓
    └─ glamberson/IronRDP
           ↓
           └─ Branch: clipboard-server-patch
                  ↓
                  └─ ONLY commit: Server clipboard initiation
```

**Benefits:**
- ✅ No middle fork (allan2)
- ✅ Stay close to upstream (easy rebase)
- ✅ Minimal divergence (1 commit vs 23)
- ✅ Clear what we've changed

---

## VERIFICATION NEEDED

**Test if Devolutions master works for us:**

```bash
cd /home/greg/repos/ironrdp-work/IronRDP

# Create clean branch from Devolutions
git checkout -b test-devolutions-direct devolutions/master

# Cherry-pick ONLY clipboard patch
git cherry-pick 2d0ed673

# Test with wrd-server
cd /home/greg/wayland/wrd-server-specs

# Temporarily update Cargo.toml to point to this branch
# ironrdp-server = { git = "...", branch = "test-devolutions-direct" }

# Try to build
cargo build --release
```

**If it builds:** ✅ We can use Devolutions directly!
**If it fails:** Need to understand what allan2 provided

---

## LIKELY OUTCOME

**Devolutions master probably works because:**
- ✅ SSPI was merged (#1028)
- ✅ All dependencies updated
- ✅ Active maintenance (recent commits)
- ✅ We only need clipboard patch (1 commit)

**Our current 23-commit divergence:**
- 15 commits from allan2 (SSPI work - probably in Devolutions now)
- 7 commits from us (debug logging - not needed)
- 1 commit from us (clipboard patch - keep this)

**Simplified fork = 1 commit divergence instead of 23!**

---

## RECOMMENDED ACTION PLAN

### Step 1: Create Clean Branch from Devolutions (15 min)

```bash
cd /home/greg/repos/ironrdp-work/IronRDP

# Ensure we have latest Devolutions
git fetch devolutions master

# Create clean branch
git checkout -b from-devolutions-clean devolutions/master

# Cherry-pick ONLY clipboard patch
git cherry-pick 2d0ed673

# If conflicts, resolve (likely none)
# Push to origin
git push origin from-devolutions-clean
```

---

### Step 2: Test with wrd-server (10 min)

```bash
cd /home/greg/wayland/wrd-server-specs

# Update Cargo.toml (all ironrdp dependencies)
# Change branch: "update-sspi-with-clipboard-fix" → "from-devolutions-clean"

# Test build
cargo clean
cargo build --release

# If successful → We're good!
# If fails → Investigate what's missing
```

---

### Step 3: If Successful, Clean Up (5 min)

```bash
# Rename branch to something cleaner
git branch -m from-devolutions-clean server-clipboard-patch

# Update wrd-server Cargo.toml to final branch name
# Delete old branch (update-sspi-with-clipboard-fix)
```

**Result:** Simple fork with 1 commit divergence instead of 23!

---

## RISKS AND MITIGATION

### Risk 1: Devolutions SSPI Different from allan2

**Likelihood:** LOW (they merged similar work)
**Test:** Build will fail if incompatible
**Mitigation:** Compare Cargo.toml versions

### Risk 2: We Depend on allan2's Specific SSPI Changes

**Likelihood:** LOW (we don't use Windows auth heavily)
**Test:** Runtime errors would appear
**Mitigation:** Test clipboard on VM

### Risk 3: Breaking Changes in Recent Devolutions

**Likelihood:** MEDIUM (they're active)
**Test:** Compilation errors
**Mitigation:** Review recent Devolutions commits for breaking changes

---

## WHAT TO CHECK BEFORE SWITCHING

**Compare dependency versions:**

```bash
# Devolutions master sspi/picky versions
git show devolutions/master:Cargo.toml | grep -E "^sspi|^picky"

# allan2 branch sspi/picky versions
git show allan2/update-sspi:Cargo.toml | grep -E "^sspi|^picky"

# Are they the same or compatible?
```

**Check for server-specific changes in allan2:**

```bash
# What did allan2 change in ironrdp-server?
git diff devolutions/master allan2/update-sspi -- crates/ironrdp-server/

# Are any of those changes critical for us?
```

---

## EXPECTED RESULT

**Most Likely:**
- ✅ Devolutions master has everything we need (SSPI was merged)
- ✅ allan2 branch is obsolete (work was upstreamed)
- ✅ We can fork Devolutions directly
- ✅ Simplified to 1-commit divergence

**Next Steps:**
1. Create branch from Devolutions
2. Apply clipboard patch
3. Test build
4. If successful → Switch permanently

---

**SHALL I EXECUTE THE TEST?**

Create clean branch from Devolutions master, apply clipboard patch, test if wrd-server builds?

This will definitively answer if we can simplify the fork chain.
