# IronRDP Fork Analysis - Current State
## Fresh Analysis Based on Actual Fork Status
**Date:** 2025-12-11
**Analysis:** Live examination of fork, not documentation

---

## CURRENT FORK STATUS

### Repository Information

**Your Fork:** `glamberson/IronRDP`
**Branch:** `from-devolutions-clean`
**Base:** Devolutions/IronRDP master (commit `0903c9ae`, Dec 9 2025)
**Divergence:** **1 commit** ahead of upstream

**Remotes Configured:**
- `origin`: glamberson/IronRDP (your fork)
- `devolutions`: Devolutions/IronRDP (official upstream)
- `allan2`: allan2/IronRDP (former intermediary, no longer used)

**Status:** ✅ Clean, minimal divergence, up-to-date with upstream

---

## THE SINGLE COMMIT DIFFERENCE

### Commit: `90be203b` - Server Clipboard Initiation

**File Changed:** `crates/ironrdp-cliprdr/src/lib.rs`
**Lines:** +38, -20 (net: +18 lines)
**Function:** `Cliprdr::initiate_copy()`

**What It Does:**

**BEFORE (Devolutions behavior):**
```rust
match (self.state, R::is_server()) {
    (CliprdrState::Ready, _) => {
        // Send FormatList (both client and server)
    }
    (CliprdrState::Initialization, false) => {
        // CLIENT ONLY: send capabilities + temp dir + FormatList
    }
    _ => {
        error!("Wrong state"); // BLOCKS servers in Init state!
    }
}
```

**AFTER (Your patch):**
```rust
if R::is_server() {
    // SERVERS: Always send FormatList, bypass state machine
    info!("SERVER initiate_copy: sending FormatList");
    pdus.push(ClipboardPdu::FormatList(...));
} else {
    // CLIENTS: Original state machine (unchanged)
    match self.state {
        CliprdrState::Ready => { /* send FormatList */ }
        CliprdrState::Initialization => { /* send caps + dir + list */ }
        _ => { error!("Wrong state"); }
    }
}
```

**Why This Matters:**

1. **MS-RDPECLIP Spec Compliance**
   - Section 2.2.3.1: "The Format List PDU is sent by **either the client or the server** when its local system clipboard is updated"
   - Servers are **allowed** to announce clipboard ownership anytime
   - IronRDP's state machine was **client-centric** (assumes client initiates)

2. **Your Use Case**
   - Linux user copies text → wrd-server must announce to Windows client
   - Without patch: Server stuck in Init state, cannot announce
   - With patch: Server can announce immediately

3. **No Client Impact**
   - Client logic **completely unchanged**
   - Only affects server behavior (R::is_server())
   - No breaking changes

---

## IRONRDP ARCHITECTURE UNDERSTANDING

### Tier System (From ARCHITECTURE.md)

**Core Tier** (Strict Quality, API Boundaries):
- **Must be #[no_std]**
- **Must be fuzzed**
- **No I/O allowed**
- **Minimal dependencies**
- Examples: `ironrdp-pdu`, `ironrdp-core`, `ironrdp-cliprdr`, `ironrdp-graphics`, `ironrdp-svc`, `ironrdp-dvc`

**Extra Tier** (Relaxed):
- Can do I/O
- Can depend on std
- Examples: `ironrdp-client`, `ironrdp-async`, `ironrdp-tokio`

**Community Tier** (NOT Core-Maintained):
- `ironrdp-server` ⭐ (maintained by @mihneabuz)
- `ironrdp-acceptor` ⭐ (maintained by @mihneabuz)
- "Core maintainers will not invest a lot of time into these"

**Critical Insight:**
- **Server support is community-maintained**
- **You are expected to extend and improve**
- **Core team focuses on client + PDUs**

---

## WHAT IRONRDP SERVER PROVIDES

From `crates/ironrdp-server/README.md`:

**Current Support:**
- Enhanced RDP Security (TLS 1.2/1.3)
- FastPath input events
- x224 input events and disconnect
- Bitmap display updates with RDP 6.0 compression

**Extension Points (Traits):**
- `RdpServerInputHandler` - Input event callbacks
- `RdpServerDisplay` - Display update notifications

**What's Missing:**
- RemoteFX encoder (exists in `ironrdp-graphics` but not integrated)
- H.264/AVC encoder (PDU structures exist, no logic)
- Multi-monitor coordination
- Advanced codecs
- **Most server business logic**

---

## PROTOCOL SUPPORT ANALYSIS

### Available IronRDP Crates

| Crate | Purpose | Tier | Server Support |
|-------|---------|------|----------------|
| **ironrdp-pdu** | All PDU encode/decode | Core | ✅ Full (wire format) |
| **ironrdp-core** | Traits (Encode/Decode) | Core | ✅ Full |
| **ironrdp-graphics** | Image codecs | Core | ⚠️ Partial (RemoteFX, bitmap) |
| **ironrdp-cliprdr** | Clipboard PDUs | Core | ⚠️ **Needs your patch** |
| **ironrdp-displaycontrol** | Display/resolution PDUs | Core | ⚠️ PDUs only, no logic |
| **ironrdp-svc** | Static virtual channels | Core | ✅ Traits |
| **ironrdp-dvc** | Dynamic virtual channels | Core | ✅ Traits |
| **ironrdp-rdpsnd** | Audio output PDUs | Core | ⚠️ PDUs only |
| **ironrdp-rdpdr** | Device redirection | Core | ⚠️ PDUs only |
| **ironrdp-input** | Input utilities | Core | ✅ Good |
| **ironrdp-server** | Server skeleton | Community | ⚠️ Basic |
| **ironrdp-acceptor** | Connection acceptance | Community | ✅ Good |

### MS-RDPEGFX (Graphics Pipeline) Support

**Location:** `crates/ironrdp-pdu/src/rdp/vc/dvc/gfx/`

**What Exists:**
- `gfx/mod.rs` (12,893 bytes) - Main PDU structures
- `gfx/graphics_messages/avc_messages.rs` - H.264 AVC PDU messages
- `gfx/graphics_messages/client.rs` - Client-side PDUs
- `gfx/graphics_messages/server.rs` - Server-side PDUs

**Status:** ⚠️ **PDU wire format only (~20% complete)**

**What's Missing:**
- Surface management logic
- H.264 encoder integration
- AVC444 color mode handling
- Frame acknowledgement logic
- Cache management
- **All business logic**

**Implication:** You'll implement ~80% for H.264 support

---

## YOUR USAGE IN WRD-SERVER

**Dependencies (from Cargo.toml):**
```toml
ironrdp = { git = "https://github.com/glamberson/IronRDP",
           branch = "from-devolutions-clean" }
ironrdp-server = { ... }
ironrdp-tokio = { ... }
ironrdp-pdu = { ... }
ironrdp-displaycontrol = { ... }
ironrdp-cliprdr = { ... }  # Uses your patch
ironrdp-core = { ... }
```

**All pointing to:** `from-devolutions-clean` branch (1 commit ahead)

---

## DECISION FRAMEWORK

### Decision 1: Fork Maintenance

**Current State:**
- 1 commit divergence
- Based on latest Devolutions master (Dec 9)
- No conflicts, clean

**Options:**

**A) Monthly Rebase**
- Run: 1st of every month
- Effort: ~30 minutes (likely just fast-forward)
- Risk: Very low (only 1 commit)
- Benefit: Stay current with security fixes

**B) Quarterly Rebase**
- Run: Every 3 months
- Effort: 1-2 hours
- Risk: Medium (more changes to merge)
- Benefit: Less frequent interruption

**C) As-Needed**
- Run: When you need upstream features
- Effort: Variable
- Risk: Could diverge significantly
- Benefit: Minimal overhead when stable

**Recommendation:** Monthly rebase (trivial with 1 commit)

---

### Decision 2: Upstream Contribution

**Your Patch:**
- Protocol-correct (MS-RDPECLIP compliant)
- Well-documented (spec references in commit)
- Server-only (no client regression risk)
- Enables proper server implementation

**Options:**

**A) Submit PR to Devolutions**
- Create clean PR from upstream
- Include spec references (Section 2.2.3.1)
- Explain server use case
- **Benefit:** If accepted, no fork needed
- **Risk:** May be rejected (community-tier crate, changes state machine)

**B) Submit to Community Maintainer (@mihneabuz)**
- ironrdp-server is his domain
- Direct engagement may be more receptive
- **Benefit:** More likely to be accepted
- **Risk:** Still may be rejected

**C) Keep Fork, Don't Submit**
- Maintain permanent fork
- No upstream engagement risk
- **Benefit:** Full control, no rejection
- **Risk:** Permanent maintenance burden

**D) Hybrid: Test Waters First**
- Open GitHub issue describing the problem
- Ask if PR would be welcome
- Gauge response before investing time
- **Benefit:** Low effort, test receptivity
- **Risk:** None

**Recommendation:** Option D (test waters), then decide A or C

---

### Decision 3: Per-Protocol Strategy

| Protocol | IronRDP Has | Decision | Your Work |
|----------|-------------|----------|-----------|
| **MS-RDPBCGR** (Core) | ✅ Full | Use IronRDP | None |
| **MS-RDPECLIP** (Clipboard) | ⚠️ 90% | Use + Fork | Loop prevention, format conversion (done) |
| **MS-RDPECLIP** (File transfer) | ⚠️ 30% (PDUs) | Use PDUs + implement | FileGroupDescriptorW parsing (~6-8 hours) |
| **MS-RDPEGFX** (H.264) | ⚠️ 20% (PDUs) | **Decision needed** | Surface mgmt, encoder, logic (~2-3 weeks) |
| **MS-RDPEDISP** (Resolution) | ⚠️ 40% (PDUs) | Use PDUs + implement | PipeWire integration (~2-3 days) |
| **MS-RDPEI** (Input) | ✅ 80% | Use IronRDP | Portal integration (done) |
| **RemoteFX** | ✅ Full | Use temporarily | None (deprecated, will replace) |

**Key Decision: MS-RDPEGFX Implementation**

**Option A: Separate Crate** (`lamco-rdp-egfx`)
- Independent from IronRDP fork
- Clean boundaries
- Can open source separately
- Could contribute to IronRDP later
- **Pro:** Flexibility, minimal fork divergence
- **Con:** More crates to maintain

**Option B: Implement in IronRDP Fork**
- Add logic to `crates/ironrdp-pdu/src/rdp/vc/dvc/gfx/`
- Tighter integration
- Offer to upstream when stable
- **Pro:** Single codebase, natural location
- **Con:** Fork divergence grows (thousands of lines)

**Option C: Wait for IronRDP**
- Stick with RemoteFX
- Wait for community to implement H.264
- **Pro:** No work
- **Con:** RemoteFX deprecated (CVE), poor quality, may wait years

**Recommendation:** Option A (separate crate) - Keeps fork minimal

---

## TECHNICAL DEBT ASSESSMENT

### What You've Changed Beyond the Fork

**In wrd-server (not in IronRDP fork):**
- FIFO clipboard request/response correlation
- on_ready() cascade bug fix (removed test FormatList)
- LibreOffice 16-request cancellation logic
- User intent respect (removed hash blocking)
- Performance optimizations (token bucket, DMA-BUF cache)

**None of these are IronRDP's responsibility** - All are your server implementation

**IronRDP's Job:**
- PDU encode/decode ✅
- Protocol state machines ✅
- TLS/NLA ✅
- Basic server skeleton ✅

**Your Job:**
- Clipboard loop prevention ✅
- PipeWire integration ✅
- Portal integration ✅
- Multi-monitor logic ⏳
- H.264 encoding ⏳
- Server orchestration ✅

---

## UPSTREAM RELATIONSHIP STRATEGY

### Recommended Approach

**Phase 1: Test Reception (This Week)**
1. Open GitHub issue on Devolutions/IronRDP
2. Title: "Server clipboard initiation blocked by state machine"
3. Describe: MS-RDPECLIP spec allows servers to announce, but state machine blocks
4. Ask: Would PR be welcome?
5. Reference: MS-RDPECLIP Section 2.2.3.1
6. **Wait for response (24-48 hours)**

**Phase 2A: If Receptive**
1. Create clean PR from Devolutions master
2. Cherry-pick your commit
3. Include test case (server initiates clipboard)
4. Submit PR
5. Be responsive to feedback

**Phase 2B: If Unreceptive or Silent**
1. Document decision to maintain fork
2. Accept permanent fork for server features
3. Monthly rebase to stay current
4. Focus on your product

**Phase 3: Long-term Maintenance**
- Monthly rebase (regardless of upstream response)
- Monitor Devolutions releases for security fixes
- Consider contributing bug fixes (if receptive)
- Keep fork minimal (only server-specific changes)

---

## MAINTENANCE OVERHEAD ESTIMATE

### Monthly Rebase (Recommended)

**Scenario 1: No Conflicts (90% of months)**
```bash
git fetch devolutions master
git rebase devolutions/master  # Fast-forward
git push origin from-devolutions-clean --force-with-lease
# Test build: cargo build --release
# Test clipboard: Manual verification
```
**Time:** 30 minutes
**Frequency:** 1st of every month
**Annual:** ~6 hours/year

**Scenario 2: Conflicts (10% of months)**
- Devolutions changes clipboard code
- Manual merge of your patch
- Test clipboard thoroughly
**Time:** 2 hours
**Frequency:** Once or twice per year
**Annual:** ~4 hours/year

**Total Annual Overhead:** ~10 hours/year (acceptable)

---

## ALTERNATIVE FORK PATHS ANALYSIS

### Path 1: Maintain Current Fork (Recommended)

**Pros:**
- Minimal divergence (1 commit)
- Easy to rebase
- Clear what you've changed
- Can upstream if accepted

**Cons:**
- Monthly maintenance (small)
- Permanent dependency on fork

**Verdict:** ✅ Best approach - Clean, maintainable

---

### Path 2: Fork IronRDP Entirely (Not Recommended)

**Pros:**
- Complete control
- No upstream dependency

**Cons:**
- Massive divergence (43 crates)
- Can't benefit from upstream improvements
- Security fixes require manual backport
- Lose community bug fixes

**Verdict:** ❌ Too much overhead

---

### Path 3: Patch at Runtime (Not Possible)

**Idea:** Hook or monkey-patch `initiate_copy()` at runtime

**Problems:**
- Rust doesn't support runtime patching
- Would require unsafe hacks
- Breaks with every IronRDP update

**Verdict:** ❌ Not feasible in Rust

---

### Path 4: Duplicate Clipboard Code (Not Recommended)

**Idea:** Copy `ironrdp-cliprdr` into your crate, modify

**Problems:**
- 7,000+ lines to maintain
- Lose upstream bug fixes
- Must track MS-RDPECLIP spec changes yourself
- Reinventing wheel

**Verdict:** ❌ Too much duplication

---

## FINAL RECOMMENDATIONS

### IronRDP Fork Strategy

**Decision:** ✅ **Maintain Minimal Fork**
- Keep `from-devolutions-clean` branch (1 commit)
- Monthly rebase against Devolutions master
- Test upstream receptivity (GitHub issue first)
- If accepted: Drop fork
- If rejected: Accept permanent fork (~10 hours/year maintenance)

### Protocol Implementation Strategy

**Use IronRDP For:**
- Core protocol (MS-RDPBCGR) ✅
- PDU encode/decode (all protocols) ✅
- TLS/NLA ✅
- Input utilities ✅
- Channel management ✅

**Implement Yourself:**
- H.264/RDPEGFX logic (separate crate: `lamco-rdp-egfx`)
- File transfer logic (use IronRDP PDUs)
- Clipboard loop prevention (done)
- Multi-monitor coordination
- Portal integration (done)
- PipeWire integration (done)
- Server orchestration (done)

### Maintenance Schedule

**Monthly (1st of month):**
- Fetch Devolutions master
- Rebase `from-devolutions-clean`
- Build test
- Manual clipboard test
- **Time:** 30 minutes

**Quarterly:**
- Review Devolutions releases
- Check for new protocols/features
- Assess if upstream changes affect you
- **Time:** 1 hour

**Annual:**
- Consider contributing improvements
- Reassess fork strategy
- **Time:** 2 hours

---

## NEXT ACTIONS

**Immediate (This Week):**

1. **Test Upstream Receptivity** (30 minutes)
   - Open GitHub issue on Devolutions/IronRDP
   - Describe server clipboard initiation problem
   - Ask if PR would be welcome
   - Wait for response

2. **Document Fork Maintenance** (1 hour)
   - Create `IRONRDP-FORK-MAINTENANCE.md`
   - Monthly rebase procedure
   - Conflict resolution steps
   - Testing checklist

**Next Week:**

3. **Based on Upstream Response**
   - If receptive: Prepare PR
   - If unreceptive: Document permanent fork decision
   - If silent: Wait 7 days, then assume permanent fork

4. **Review Protocol Decisions**
   - Decide H.264 implementation approach (separate crate recommended)
   - Plan file transfer implementation
   - Assess multi-monitor needs

---

## APPENDIX: COMPLETE FORK DIFF

**File:** `crates/ironrdp-cliprdr/src/lib.rs`
**Function:** `Cliprdr::initiate_copy()`
**Change Type:** Logic modification (state machine bypass for servers)
**Spec Reference:** MS-RDPECLIP Section 2.2.3.1
**Testing:** Verified on GNOME and KDE Linux desktops with Windows clients

**Full commit message:**
```
fix(cliprdr): enable server clipboard ownership announcements

Servers can now announce clipboard changes via initiate_copy() regardless
of internal state (Initialization or Ready). The existing state machine logic
was designed for clients where clipboard must be initialized before use.

Per MS-RDPECLIP Section 2.2.3.1: 'The Format List PDU is sent by either
the client or the server when its local system clipboard is updated with
new clipboard data.'

Changes:
- Servers bypass state machine check and always send CB_FORMAT_LIST (0x0002)
- Clients preserve original Initialization->Ready state machine behavior
- Enables server->client clipboard flow for RDP server implementations

Fixes: RDP servers unable to announce clipboard ownership in Init state
Protocol: MS-RDPECLIP (Remote Desktop Protocol: Clipboard Virtual Channel Extension)
Tested: WRD-Server (Wayland RDP Server) on GNOME and KDE Linux desktops
```

---

**END OF CURRENT STATE ANALYSIS**

*This analysis is based on live examination of the fork as of 2025-12-11.*
*Previous documents may contain outdated information from before fork simplification.*
