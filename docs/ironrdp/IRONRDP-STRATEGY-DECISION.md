# IRONRDP FORK STRATEGY - DECISION DOCUMENT
## Critical Dependency Analysis and Maintenance Plan
**Date:** 2025-12-10
**Purpose:** Define clear strategy for IronRDP usage, fork maintenance, and upstream relationship

---

## EXECUTIVE SUMMARY

IronRDP is our **most critical dependency** - we depend on it for all RDP protocol work. We currently use a **fork** with custom patches. This document analyzes:
1. What IronRDP provides vs what we need
2. Why we maintain a fork
3. Long-term maintenance strategies
4. Risk mitigation
5. Clear action plan

**Current Status:**
- Fork: `glamberson/IronRDP`
- Branch: `update-sspi-with-clipboard-fix`
- Divergence: 8 commits ahead of `allan2/IronRDP`
- Critical patch: Server clipboard initiation

**Recommendation:** **Maintain Strategic Fork** with clear boundaries and monthly rebase schedule.

---

## IRONRDP CAPABILITY ANALYSIS

### What IronRDP Provides Well ✅

| Component | Maturity | Our Usage | Status |
|-----------|----------|-----------|--------|
| **ironrdp-server** | Good | Core server skeleton | ✅ Using |
| **ironrdp-pdu** | Excellent | All PDU encoding/decoding | ✅ Using |
| **ironrdp-cliprdr** | Good | Clipboard PDUs | ✅ Using + Patched |
| **ironrdp-graphics** | Good | RemoteFX encoding | ✅ Using |
| **ironrdp-svc** | Good | Static virtual channels | ✅ Using |
| **ironrdp-core** | Excellent | Core types | ✅ Using |
| **ironrdp-tokio** | Good | Async runtime integration | ✅ Using |
| **ironrdp-acceptor** | Good | TLS/NLA handling | ✅ Using |
| **ironrdp-tls** | Good | TLS 1.3 via rustls 0.23 | ✅ Using |
| **ironrdp-async** | Good | Async traits | ✅ Using |

**Summary:** IronRDP handles **95% of RDP protocol** correctly. Well-maintained, active development.

### What IronRDP Lacks ⚠️

| Feature | Status | Our Need | Impact |
|---------|--------|----------|--------|
| **MS-RDPEGFX (H.264)** | Minimal/incomplete | High (v1.1) | Need to implement |
| **MS-RDPEDISP (resolution)** | Limited | Medium (v1.1) | Need to implement |
| **Server clipboard initiation** | Missing | Critical | **Our patch** |
| **FileContents logic** | PDUs only | High (v1.0) | Need to implement |
| **Audio (MS-RDPEA)** | Basic (ironrdp-rdpsnd) | Low (v2.0+) | Future |

**Summary:** Protocol gaps exist but are **expected** - IronRDP focuses on client. Server features are our responsibility.

---

## WHY WE HAVE A FORK

### The Critical Patch: Server Clipboard Initiation

**Problem:**
- RDP traditionally: Client copies → Server pastes (client initiates)
- Our need: Server copies → Client pastes (**server initiates**)
- IronRDP assumption: Servers only respond to client Format List

**Our Patch (commit `2d0ed673`):**
```rust
// In ironrdp-cliprdr/src/backend.rs
// Allow servers to send FormatList (initiate clipboard)
// Bypass client state machine check
```

**Why This Matters:**
- Linux user copies text → We must announce to Windows client
- Without this: Copy from Linux doesn't work
- This is **fundamental to bidirectional clipboard**

**Will Upstream Accept This?**
- ❌ **Probably not** - Breaks RDP client assumptions
- Server clipboard initiation is non-standard behavior
- RDP spec: Clients control clipboard, servers respond
- **Our use case:** Server = Linux desktop (non-standard)

**Conclusion:** We **need to maintain this patch indefinitely**.

### Other Patches in Our Fork

**Commits in `update-sspi-with-clipboard-fix`:**
1. `fix(cliprdr): enable server clipboard ownership announcements` - **CRITICAL PATCH**
2. `debug(cliprdr): add extensive logging to trace PDU encoding path` - Debug only
3. `fix(server): add missing info macro import for debug logging` - Minor fix
4. `fix(svc): remove tracing calls (ironrdp-svc doesn't have tracing dep)` - Bug fix (contribute upstream)
5. `debug(cliprdr): add chunk-level encoding logging` - Debug only
6. `fix(server): remove len() calls on SvcProcessorMessages` - Bug fix (contribute upstream)
7. `fix(server): remove flush call (FramedWrite handles buffering internally)` - Bug fix (contribute upstream)
8. `debug(server): add flush and write confirmation for clipboard PDUs` - Debug only

**Summary:**
- 1 critical patch (server clipboard initiation) - **Can't upstream**
- 3 bug fixes - **Should upstream**
- 4 debug commits - **Can discard**

---

## FORK MAINTENANCE STRATEGIES

### Option A: Maintain Fork Indefinitely (Current)

**Approach:**
- Keep `glamberson/IronRDP` fork
- Maintain `update-sspi-with-clipboard-fix` branch
- Monthly rebase against `allan2/IronRDP` main
- Cherry-pick upstream changes

**Pros:**
- ✅ Full control over clipboard behavior
- ✅ Can iterate quickly
- ✅ Can add server-specific features easily

**Cons:**
- ❌ Maintenance burden (rebasing, conflict resolution)
- ❌ Upstream divergence risk
- ❌ Miss upstream improvements if not rebased
- ❌ Community fragmentation

**Effort:** ~4 hours/month (monthly rebase)

---

### Option B: Patch at Build Time

**Approach:**
- Use upstream `allan2/IronRDP`
- Apply patch files at build time (Cargo patch system)
- Keep patches in our repo

**Pros:**
- ✅ Stay close to upstream
- ✅ Clear what we've modified
- ✅ Easy to drop patches if upstream changes

**Cons:**
- ❌ Patches break easily on upstream changes
- ❌ Harder to test (patch might not apply cleanly)
- ❌ More complex build system

**Effort:** ~6 hours/month (patch maintenance)

**Verdict:** Not recommended - more fragile than fork.

---

### Option C: Vendored IronRDP (Git Submodule)

**Approach:**
- Git submodule of IronRDP in our repo
- Apply patches directly in submodule
- Update submodule periodically

**Pros:**
- ✅ Full control
- ✅ Clear version tracking
- ✅ Can test before updating

**Cons:**
- ❌ Submodules are painful
- ❌ Still need to manage patches
- ❌ No upstream visibility

**Verdict:** Not recommended - submodules cause more problems than they solve.

---

### Option D: Contribute Everything Upstream (Ideal but Unrealistic)

**Approach:**
- Submit all patches as PRs to `allan2/IronRDP`
- Work with maintainer to get accepted
- Use official releases

**Pros:**
- ✅ No fork maintenance
- ✅ Community benefits
- ✅ Shared maintenance burden

**Cons:**
- ❌ Server clipboard initiation likely rejected (architectural)
- ❌ Slower development velocity (PR review cycles)
- ❌ Prior experience: Not well received

**Verdict:** Ideal for bug fixes, impossible for our critical patch.

---

### Option E: Hybrid Approach (RECOMMENDED)

**Approach:**
- Maintain fork for server-specific features ONLY
- Contribute generic improvements upstream
- Keep fork minimal and well-documented
- Monthly rebase schedule

**Fork Contains:**
- Server clipboard initiation (critical patch)
- Any future server-specific protocol extensions

**Upstream Contributions:**
- Bug fixes (our commits 4, 6, 7)
- Protocol correctness improvements
- Documentation improvements
- Client features (if any)

**Pros:**
- ✅ Minimal fork (just server features)
- ✅ Good open source citizen
- ✅ Shared maintenance where possible
- ✅ Clear boundaries

**Cons:**
- ⚠️ Still maintain fork (small burden)

**Effort:** ~2 hours/month (rebase) + ~4 hours/quarter (upstream contributions)

**Verdict:** ✅ **RECOMMENDED** - Best balance of control and community engagement.

---

## RECOMMENDED STRATEGY: HYBRID APPROACH

### Clear Boundaries

**What Goes in Our Fork:**
1. ✅ Server clipboard initiation (critical patch)
2. ✅ Server-specific protocol extensions (if any)
3. ✅ Features that break RDP client assumptions

**What Goes Upstream:**
1. ✅ Bug fixes (all crates)
2. ✅ Performance improvements (generic)
3. ✅ Protocol correctness (spec compliance)
4. ✅ Documentation improvements
5. ✅ Test coverage improvements

### Fork Maintenance Schedule

**Monthly (1st of each month):**
1. Check `allan2/IronRDP` for new commits
2. Rebase `update-sspi-with-clipboard-fix` against latest main
3. Test wrd-server build and functionality
4. Resolve conflicts if any
5. Update our Cargo.toml commit hashes

**Quarterly (Every 3 months):**
1. Review our fork commits
2. Identify bug fixes to upstream
3. Create PRs for acceptable changes
4. Clean up debug commits (rebase/squash)

**Documentation:**
- `IRONRDP-FORK.md` - Document all our patches
- Commit messages: Clear "PATCH:" prefix for our changes
- Fork README: Explain purpose and maintenance

### Upstream Contribution Process

**For Each Bug Fix:**
1. Create clean branch from `allan2/IronRDP` main
2. Apply fix with clear commit message
3. Add test if applicable
4. Submit PR with:
   - Clear description
   - Why it's needed
   - How it's tested
5. Be responsive to feedback
6. If rejected: Document why, keep in fork

---

## RISK MITIGATION

### Risk 1: Upstream Makes Breaking Changes

**Likelihood:** MEDIUM (Rust RDP is evolving)
**Impact:** HIGH (Build breaks)

**Mitigation:**
- Monthly rebase catches changes early
- Maintain good test coverage
- Pin IronRDP version in Cargo.lock
- Test before deploying

**Fallback:** Stay on known-good IronRDP commit for production.

---

### Risk 2: Our Fork Diverges Too Far

**Likelihood:** MEDIUM (if we don't rebase)
**Impact:** HIGH (Can't use upstream improvements)

**Mitigation:**
- **Mandatory monthly rebase**
- Minimize fork changes (hybrid approach)
- Automated tests catch regressions
- Document all divergence

---

### Risk 3: Upstream Adds Our Feature (Clipboard Initiation)

**Likelihood:** LOW (architectural difference)
**Impact:** POSITIVE (Can drop fork!)

**Mitigation:**
- Monitor IronRDP issues/PRs
- Engage with maintainer
- Offer to help if they add this

**Action if it happens:** 🎉 Drop fork, use upstream!

---

### Risk 4: IronRDP Project Abandoned

**Likelihood:** LOW (active development)
**Impact:** HIGH (Must maintain entire protocol)

**Mitigation:**
- Monitor activity (commits, issues, PRs)
- Have contingency: Fork entire codebase
- Budget: ~1 FTE for full protocol maintenance

**Early Warning Signs:**
- No commits for 3+ months
- No issue responses for 1+ month
- Maintainer announces hiatus

**Fallback:** Full fork, hire RDP protocol expert.

---

## IRONRDP DEPENDENCY BOUNDARIES

### Crates We Use

```toml
[dependencies]
ironrdp = { git = "...", branch = "..." }                # Umbrella (server feature)
ironrdp-server = { git = "...", branch = "..." }         # Server skeleton
ironrdp-pdu = { git = "...", branch = "..." }            # PDU encode/decode
ironrdp-cliprdr = { git = "...", branch = "..." }        # Clipboard PDUs (PATCHED)
ironrdp-displaycontrol = { git = "...", branch = "..." } # Display control
ironrdp-core = { git = "...", branch = "..." }           # Core types
```

### What Each Crate of Ours Uses IronRDP For

| Our Crate | IronRDP Dependency | What For |
|-----------|-------------------|----------|
| `lamco-rdp-clipboard` | `ironrdp-cliprdr` | Clipboard PDUs, CliprdrBackend trait |
| `lamco-rdp-input` | `ironrdp-pdu` | Input PDU types |
| `lamco-rdp-server` | `ironrdp-server`, `ironrdp-core` | Server trait implementations |
| `lamco-portal-integration` | None | Independent |
| `lamco-pipewire-capture` | None | Independent |
| `lamco-video-pipeline` | None | Independent |

**Key Insight:** Only 3 of our crates depend on IronRDP! Others are independent.

---

## ACTION PLAN

### Immediate (This Week)

1. **Document Fork**
   - Create `IRONRDP-FORK.md` in our repo
   - List all patches with rationale
   - Document rebase procedure

2. **Clean Up Fork**
   - Squash debug commits
   - Keep only critical patch + bug fixes
   - Clear commit messages with "PATCH:" prefix

3. **Identify Upstream Contributions**
   - Extract bug fixes (commits 4, 6, 7)
   - Test independently
   - Prepare PRs (or save for later)

### Short-Term (This Month)

4. **Set Up Rebase Automation**
   - Script to check for upstream changes
   - Automated rebase (with manual verification)
   - CI/CD to test after rebase

5. **Version Pinning**
   - Pin exact commit hash in Cargo.toml
   - Document known-good versions
   - Test matrix (latest + pinned)

### Ongoing (Monthly)

6. **Monthly Rebase**
   - 1st of every month
   - Rebase against `allan2/IronRDP` main
   - Test wrd-server build
   - Update Cargo.toml

7. **Upstream Monitoring**
   - Watch IronRDP repo
   - Review new issues/PRs
   - Engage with community

---

## LONG-TERM VISION

### If IronRDP Adds MS-RDPEGFX (H.264)

**Our Action:**
- ✅ Use their implementation
- ✅ Integrate into our video pipeline
- ✅ Deprecate RemoteFX

**Timeline:** Monitor for v1.1 (when we need H.264)

### If IronRDP Adds Server Clipboard Initiation

**Our Action:**
- 🎉 Drop our fork!
- ✅ Use upstream
- ✅ Contribute improvements

**Likelihood:** LOW (but watch for it)

### If We Need More Server Features

**Examples:**
- Advanced input (MS-RDPEI extensions)
- Audio redirection
- USB redirection

**Our Action:**
- Add to fork (server-specific)
- Keep separate branch per feature
- Document clearly

---

## CRATE STRUCTURE IMPLICATIONS

### IronRDP Dependency Is NOT a Blocker

**Key Realization:** Most of our crates are IronRDP-independent!

**Independent Crates (No IronRDP dependency):**
- `lamco-portal-integration` - Only uses `ashpd`/`zbus`
- `lamco-pipewire-capture` - Only uses `pipewire` crate
- `lamco-video-pipeline` - Pure Rust pixel conversion
- `lamco-rdp-utils` - Generic utilities

**IronRDP-Dependent Crates:**
- `lamco-rdp-clipboard` - Uses `ironrdp-cliprdr` (our fork)
- `lamco-rdp-input` - Uses `ironrdp-pdu` (upstream OK)
- `lamco-rdp-server` - Uses `ironrdp-server` (our fork for traits)

**Implication:** We can open source most crates immediately. Only clipboard/server need our fork.

---

## ALTERNATIVE: CONTRIBUTE TO IRONRDP ECOSYSTEM

### What If We Contribute Major Features?

**Option:** Instead of fork, add server features to IronRDP as optional.

**Approach:**
```rust
// In ironrdp-cliprdr
#[cfg(feature = "server-initiate-clipboard")]
pub fn server_send_format_list(...) { /* our logic */ }
```

**Pros:**
- ✅ No fork needed
- ✅ Community benefits
- ✅ Shared maintenance

**Cons:**
- ❌ Maintainer must accept feature flag
- ❌ Slower iteration
- ❌ May conflict with client architecture

**Verdict:** Worth discussing with IronRDP maintainer. If rejected, fallback to fork.

---

## DECISION REQUIRED

### Questions for You

1. **Fork Maintenance:**
   - ✅ Approve hybrid approach (maintain fork for server features, contribute bug fixes)?
   - ⚠️ Or: Try to contribute everything upstream (slower, may be rejected)?

2. **Maintenance Budget:**
   - Monthly rebase: ~2 hours/month
   - Quarterly upstream contributions: ~4 hours/quarter
   - **Total:** ~30 hours/year
   - Acceptable?

3. **Upstream Engagement:**
   - Should we reach out to IronRDP maintainer (allan2) about server features?
   - Offer to collaborate on server-specific extensions?

4. **Fork Naming:**
   - Keep `glamberson/IronRDP`?
   - Or rename to `lamco/IronRDP` for branding?

---

## RECOMMENDATION SUMMARY

**Recommended Strategy:** ✅ **Hybrid Fork Approach**

1. **Maintain minimal fork** (server clipboard initiation)
2. **Contribute bug fixes upstream** (be good citizen)
3. **Monthly rebase schedule** (stay current)
4. **Clear documentation** (IRONRDP-FORK.md)
5. **Monitor for upstream changes** (may obsolete fork)

**Why This Works:**
- ✅ Unblocks our development (have critical patch)
- ✅ Maintains community relationship (contribute back)
- ✅ Low maintenance burden (minimal fork)
- ✅ Clear strategy (documented boundaries)

**Next Action:** Review and approve, then implement action plan.

---

**END OF IRONRDP STRATEGY**

*Awaiting decision on fork maintenance approach.*
