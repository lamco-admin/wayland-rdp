# Licensing and Publication Handover Document
**Date:** 2025-12-21
**Status:** DECISIONS MADE - READY FOR IMPLEMENTATION
**Next Session:** Implement licensing and prepare for publication

---

## EXECUTIVE SUMMARY

**Product:** lamco-rdp-server (Portal mode RDP server for Linux/Wayland)
**License Model:** BSL 1.1 (Business Source License) - Free with restrictions, auto-converts to Apache-2.0 after 3 years
**License Platform:** Lemon Squeezy (automated license management and payments)
**Current Status:** Code ready, licensing infrastructure needs setup

---

## LICENSING DECISIONS (FINAL)

### License: Business Source License 1.1 (BSL)

**SPDX Identifier:** `BUSL-1.1`

**Cargo.toml:**
```toml
license = "BUSL-1.1"
```

**Full License Parameters:**

```
Licensor:             Lamco Development
Licensed Work:        lamco-rdp-server
Change Date:          3 years from version release
Change License:       Apache-2.0

Additional Use Grant:
  You may make production use of the Licensed Work if you meet ALL of:
  1. You are a non-profit organization, OR
  2. You are a for-profit organization with:
     - 3 or fewer employees, AND
     - Less than $1,000,000 USD in annual revenue, AND
  3. Your use is not to create a competitive or derivative RDP server
     product or VDI solution

  All other production use requires a commercial license from Lamco Development.
```

**What This Means:**
- ✅ **Free for:** Non-profits, tiny businesses (≤3 employees + <$1M revenue), personal use
- ✅ **Free restriction:** Cannot build competing RDP products
- ❌ **Requires paid license:** Real businesses, competitive products
- ⏰ **Auto-converts:** Each version becomes Apache-2.0 after 3 years

---

## COMMERCIAL LICENSE PRICING

### Two License Options (Managed via Lemon Squeezy)

**Option 1: Annual Subscription**
- **Price:** $49.99/year per server
- **Includes:** Updates, bug fixes, email support
- **Renews:** Automatically unless cancelled
- **Lemon Squeezy Product ID:** TBD (create during setup)

**Option 2: Perpetual License**
- **Price:** $99.00 one-time
- **Includes:** Perpetual use of purchased version, email support for 1 year
- **Updates:** Major updates require new purchase OR annual subscription
- **Lemon Squeezy Product ID:** TBD (create during setup)

### What Commercial License Includes

**Rights:**
- Production use (unlimited servers with paid license per server)
- Modification and internal deployment
- No additional fees based on users/connections

**Support:**
- Email support: office@lamco.io
- Business hours response (no SLA)
- Bug fixes and security updates

**Restrictions:**
- Cannot redistribute commercially
- Cannot create derivative competing products
- Cannot sublicense to third parties

---

## LEMON SQUEEZY SETUP (NOT DONE YET)

### Account Setup Required

**Steps to complete:**
1. Create account at lemonsqueezy.com
2. Set up store: "Lamco Development"
3. Connect bank account for payouts
4. Create two products:
   - "lamco-rdp-server Annual License" - $49.99/year recurring
   - "lamco-rdp-server Perpetual License" - $99.00 one-time
5. Configure license key generation (automatic)
6. Set up webhook for notifications
7. Get API key for validation (if implementing enforcement)

### Lemon Squeezy Configuration Details

**Store URL:** TBD (will be: lamco.lemonsqueezy.com or custom domain)

**Products to create:**
```
Product 1: lamco-rdp-server Annual Commercial License
- Price: $49.99/year
- Billing: Annual subscription
- License keys: Auto-generate on purchase
- Description: "Commercial license for lamco-rdp-server. Includes
  production use rights, updates, and email support for one year."

Product 2: lamco-rdp-server Perpetual Commercial License
- Price: $99.00
- Billing: One-time payment
- License keys: Auto-generate on purchase
- Description: "Perpetual license for lamco-rdp-server. Lifetime
  right to use purchased version with 1 year of email support."
```

**Revenue Split:**
- Lemon Squeezy: 5%
- Stripe fees: ~3%
- You receive: ~92% ($45.99 per $49.99 sale)

---

## LICENSE FILE IMPLEMENTATION

### File: LICENSE

**Full text needed:**
```
Business Source License 1.1

License text: https://mariadb.com/bsl11/

Parameters

Licensor:             Lamco Development
Licensed Work:        lamco-rdp-server
                      The Licensed Work is (c) 2025 Lamco Development
Additional Use Grant: You may make production use of the Licensed Work if:

                      1. You are a non-profit organization, OR
                      2. You are a for-profit organization with ALL of:
                         - 3 or fewer employees, AND
                         - Less than $1,000,000 USD in annual revenue, AND
                      3. Your use does not include creating a competitive
                         or derivative RDP server product or VDI solution.

                      For all other production use, you must purchase a
                      commercial license from Lamco Development.
                      Contact: office@lamco.io

Change Date:          2028-12-21 (three years from first release)

Change License:       Apache-2.0

For information about alternative licensing arrangements for the Licensed Work,
please contact office@lamco.io.

Notice

The Business Source License (this document, or the "License") is not an Open
Source license. However, the Licensed Work will eventually be made available
under an Open Source License, as stated in this License.

[Full BSL 1.1 license text follows below]
[Insert from: https://mariadb.com/bsl11/]
```

### File: LICENSE-APACHE (For Reference)

Include Apache-2.0 license text in repo with note:
```
This software will convert to Apache-2.0 license on 2028-12-21
(three years from first release). This file is included for reference.
```

---

## REPOSITORY AND PUBLICATION PLAN

### Current State

**Repository:** `/home/greg/wayland/wrd-server-specs`
- **Purpose:** Private development repo (can have Claude refs, messy commits)
- **Branch:** main (just updated to latest clean work)
- **Branches preserved:**
  - `feature/lamco-compositor-clipboard` - VDI compositor (Smithay 0.7)
  - `feature/headless-development` - Compositor experiments
- **Remote:** github.com/lamco-admin/wayland-rdp (private)

### Public Repository (TO BE CREATED)

**Name:** `lamco-rdp-server`
**URL:** github.com/lamco-admin/lamco-rdp-server (doesn't exist yet)
**Purpose:** Clean public repo for Portal mode product
**License:** BSL 1.1 with terms above

**What goes in public repo:**
- ✅ All src/ code (Portal mode only, no compositor)
- ✅ Cargo.toml, Cargo.lock
- ✅ LICENSE file (BSL 1.1 with parameters)
- ✅ LICENSE-APACHE (reference for future)
- ✅ README.md (clean, user-facing)
- ✅ INSTALL.md
- ✅ CONFIGURATION.md
- ✅ CONTRIBUTING.md
- ✅ .github/workflows/ (CI/CD)
- ❌ NO docs/archive, docs/status-reports, docs/strategy
- ❌ NO session notes or Claude references
- ❌ NO compositor code

---

## CARGO.TOML UPDATES NEEDED

**Current:**
```toml
license = "MIT OR Apache-2.0"
```

**Change to:**
```toml
license = "BUSL-1.1"
```

**Repository field (already correct):**
```toml
repository = "https://github.com/lamco-admin/lamco-rdp-server"
```

---

## README.md LICENSING SECTION

**Add to README:**

```markdown
## License

lamco-rdp-server is licensed under the Business Source License 1.1 (BSL).

### Free Use

You may use lamco-rdp-server for free if:
- You are a non-profit organization, OR
- Your organization has 3 or fewer employees AND less than $1M annual revenue, AND
- You are not creating a competitive RDP server or VDI product

### Commercial License

Larger organizations or commercial deployments require a commercial license:
- **Annual License:** $49.99/year per server
- **Perpetual License:** $99.00 one-time per server

Purchase: [Lemon Squeezy checkout URL - TBD]
Contact: office@lamco.io

### Future Open Source

This software will automatically convert to Apache-2.0 license three years
after each version's release. See LICENSE file for details.
```

---

## CRATES.IO PUBLICATION PLAN

### Can Publish to crates.io: YES

**BUSL-1.1 is valid SPDX identifier** - crates.io accepts it

**When to publish:**
- ✅ License file created
- ✅ Cargo.toml updated with BUSL-1.1
- ✅ README explains licensing
- ✅ Code is production-ready (already is)

**Publication command:**
```bash
cargo publish
```

**Effect:**
- Anyone can download from crates.io
- Anyone can see source code
- License restricts commercial production use
- You sell commercial licenses via Lemon Squeezy

---

## NEXT SESSION TODO LIST

### Priority 1: Licensing Setup (1-2 hours)

1. **Create LICENSE file**
   - Copy BSL 1.1 template from mariadb.com/bsl11/
   - Fill in parameters (Licensor, Additional Use Grant, Change Date, Change License)
   - Add to repo root

2. **Create LICENSE-APACHE file**
   - Copy Apache-2.0 text
   - Add note: "This software converts to this license on 2028-12-21"
   - Add to repo root

3. **Update Cargo.toml**
   - Change `license = "BUSL-1.1"`
   - Verify repository field is correct
   - Commit

4. **Update README.md**
   - Add License section (see template above)
   - Add link to Lemon Squeezy store (placeholder until created)
   - Commit

### Priority 2: Lemon Squeezy Setup (30-60 minutes)

1. **Create Lemon Squeezy account**
   - Go to lemonsqueezy.com
   - Sign up
   - Verify email
   - Connect bank account (for payouts)

2. **Create store**
   - Store name: "Lamco Development"
   - Configure currency: USD
   - Set tax settings (automatic)

3. **Create products**
   - Product 1: "lamco-rdp-server Annual License" - $49.99/year recurring
   - Product 2: "lamco-rdp-server Perpetual License" - $99.00 one-time
   - Enable license key generation on both
   - Configure confirmation emails

4. **Get checkout URLs**
   - Copy checkout link for each product
   - Update README.md with actual links
   - Test purchase flow (use test mode)

5. **Get API key** (if doing enforcement)
   - Settings → API
   - Generate API key
   - Store securely (for validation integration later)

### Priority 3: Public Repository Creation (1 hour)

1. **Create GitHub repo**
   - github.com/lamco-admin/lamco-rdp-server
   - Public
   - No init files (will push from local)

2. **Export clean code**
```bash
cd /tmp
git clone /home/greg/wayland/wrd-server-specs lamco-rdp-server-public
cd lamco-rdp-server-public
git checkout main

# Remove private docs
rm -rf docs/archive docs/status-reports docs/strategy docs/ironrdp docs/decisions
rm -rf .claude/research-backups

# Keep only user docs
mkdir -p docs-clean
# Move/create: README.md, INSTALL.md, CONFIGURATION.md, TROUBLESHOOTING.md

# Create fresh commit
git checkout --orphan main-clean
git add -A
git commit -m "Initial release: lamco-rdp-server v0.1.0"

# Push to new repo
git remote remove origin
git remote add origin https://github.com/lamco-admin/lamco-rdp-server.git
git push -u origin main-clean:main
```

3. **Configure GitHub repo**
   - Add description
   - Add topics: rdp, wayland, remote-desktop, linux, rust
   - Enable Issues
   - Add FUNDING.yml (link to Lemon Squeezy)

### Priority 4: Documentation (2-3 hours)

Create for public repo:
1. **INSTALL.md** - Installation instructions
2. **CONFIGURATION.md** - Configuration reference
3. **TROUBLESHOOTING.md** - Common issues
4. **CONTRIBUTING.md** - How to contribute

---

## LICENSE ENFORCEMENT DECISION

### Option A: Honor System (RECOMMENDED FOR NOW)

**Implementation:** NONE
- No license key validation in code
- No phone-home checks
- Just licensing terms in LICENSE file
- Trust users to comply

**Why:**
- Simple (zero code changes)
- Focus on product, not DRM
- Professional businesses will comply
- Can add later if needed

**README says:**
"Commercial use requires a license. Purchase at [Lemon Squeezy link].
No license key validation - honor system."

### Option B: Soft Enforcement (FUTURE)

**Implementation:** Optional license key check
```rust
// In main.rs - optional flag
#[arg(long)]
commercial_license_key: Option<String>,

// If provided, validate via Lemon Squeezy API
// If valid, allow unrestricted use
// If not provided, show reminder (but still work)
```

**Why:**
- Gentle reminder for commercial users
- Doesn't break software if no key
- Can implement later

**Recommendation:** Start with Option A (honor system), add Option B later if needed

---

## PUBLISHED CRATES STATUS

### Already Published (v0.2.0) ✅

| Crate | Version | License | Status |
|-------|---------|---------|--------|
| lamco-portal | 0.2.0 | MIT OR Apache-2.0 | Published |
| lamco-pipewire | 0.1.2 | MIT OR Apache-2.0 | Published |
| lamco-video | 0.1.1 | MIT OR Apache-2.0 | Published |
| lamco-clipboard-core | 0.2.0 | MIT OR Apache-2.0 | Published |
| lamco-rdp-clipboard | 0.2.0 | MIT OR Apache-2.0 | Published |
| lamco-rdp-input | 0.1.0 | MIT OR Apache-2.0 | Published |

**These stay MIT/Apache** - they're open source infrastructure

### To Be Published

**lamco-rdp-server (this product):**
- Version: 0.1.0
- License: BUSL-1.1 (NOT open source)
- Status: Code ready, needs LICENSE file
- Publication: After licensing setup complete

---

## CONTACT INFORMATION

**Email for licensing:** office@lamco.io
- Commercial license inquiries
- Support requests
- General questions

**Email for package:** contact@lamco.ai (in Cargo.toml authors field)

---

## CURRENT BRANCH AND CODE STATUS

### Repository Structure

**Location:** /home/greg/wayland/wrd-server-specs
**Remote:** github.com/lamco-admin/wayland-rdp (private dev repo)
**Branch:** main (just updated from feature/lamco-rdp-server-prep)

**Main branch contains:**
- ✅ Portal mode code (production-ready)
- ✅ Clean commits (no Claude refs in recent history)
- ✅ Uses published lamco-* v0.2.0 crates
- ✅ All tests passing (79 passed)
- ✅ Build successful
- ❌ NO compositor code (separate branches)

### Other Important Branches (PRESERVE)

**feature/lamco-compositor-clipboard:**
- **Contains:** VDI compositor (Smithay 0.7)
- **Size:** 4,586 lines compositor code
- **Status:** Complete, production-ready (marked "READY TO SHIP" Nov 20)
- **Purpose:** Future VDI product (separate repo/license later)
- **DO NOT MERGE:** This is different product

**feature/headless-development:**
- **Contains:** Alternative compositor approach (Smithay 0.3)
- **Status:** Experimental
- **Purpose:** Alternative architecture exploration
- **DO NOT MERGE:** Keep for reference

---

## WHAT GETS PUBLISHED WHERE

### Public lamco-rdp-server Repo (Portal Mode Only)

**Include:**
- src/ (all current source - NO compositor)
- Cargo.toml (with BUSL-1.1 license)
- LICENSE (BSL 1.1 with full terms)
- LICENSE-APACHE (reference for future)
- README.md (with licensing section)
- INSTALL.md, CONFIGURATION.md, TROUBLESHOOTING.md
- Examples/
- .github/workflows/ (CI/CD)

**Exclude:**
- docs/archive/
- docs/status-reports/
- docs/strategy/
- docs/ironrdp/
- docs/decisions/
- All session notes
- All Claude references
- .claude/research-backups/

**Size:** ~10,800 lines of Portal mode code

### Private wrd-server-specs Repo (Development)

**Keep everything:**
- All branches
- All docs (100+ files)
- Session notes
- Claude collaboration docs
- Experimental code
- Compositor code in branches

**Purpose:** Messy development, planning, research

---

## VDI PRODUCT (SEPARATE - NOT NOW)

**Code location:** `feature/lamco-compositor-clipboard` branch
**Smithay version:** 0.7.0
**Status:** Complete compositor implementation
**Size:** ~4,586 lines

**Future actions (NOT THIS SESSION):**
- Extract to separate repo: `lamco-vdi-server`
- Separate licensing (likely higher pricing)
- Separate from Portal mode completely

**For now:** Just preserve the branch, don't touch

---

## TECHNICAL READINESS CHECKLIST

### Code Status ✅

- [x] All published crates v0.2.0 integrated
- [x] Path patches removed
- [x] Build successful (1m 32s)
- [x] Tests passing (79/79 + 1 ignored)
- [x] Zero TODOs in codebase
- [x] Clean git history on main
- [x] Production-ready

### Remaining Before Publication

- [ ] Create LICENSE file (BSL 1.1 with parameters)
- [ ] Create LICENSE-APACHE file (reference)
- [ ] Update Cargo.toml license field to BUSL-1.1
- [ ] Update README.md with licensing section
- [ ] Create INSTALL.md
- [ ] Create CONFIGURATION.md
- [ ] Create TROUBLESHOOTING.md
- [ ] Create CONTRIBUTING.md
- [ ] Set up Lemon Squeezy account and products
- [ ] Create public GitHub repo
- [ ] Export clean code to public repo
- [ ] Publish to crates.io

**Estimated time:** 6-8 hours total

---

## LEMON SQUEEZY INTEGRATION (OPTIONAL ENFORCEMENT)

### If You Want License Validation

**Rust crate available:** `lemonsqueezy` (crates.io/crates/lemonsqueezy)

**Optional feature in main.rs:**
```rust
#[cfg(feature = "license-check")]
async fn validate_commercial_license(key: &str) -> Result<bool> {
    // Call Lemon Squeezy API
    // https://api.lemonsqueezy.com/v1/licenses/validate
    // Returns: valid/invalid
}

// Only check if --license-key provided
// Don't block if not provided (honor system)
```

**Recommendation:** Don't implement yet. Start honor system, add later if needed.

---

## MARKETING/MESSAGING

### Positioning Statement

"**lamco-rdp-server** - Professional RDP server for Wayland/Linux

Free for personal use, students, non-profits, and small businesses.
Commercial licenses available for larger organizations.

Built with production-tested Rust code. Becomes Apache-2.0 open source in 3 years."

### Key Messages

- **Free tier:** Generous (covers hobbyists, students, tiny startups)
- **Commercial pricing:** Affordable ($49.99/year vs competitors at $100-300)
- **Future open source:** Builds trust, attracts early adopters
- **Quality:** Production-tested, published crates, clean architecture

---

## COMPETITOR PRICING COMPARISON

| Product | Model | Price | Notes |
|---------|-------|-------|-------|
| **TSplus** | Per server | $240/year | Proprietary Windows-based |
| **Parallels RAS** | Per user | $60-90/year | Enterprise focus |
| **RemotePC** | Per user | $30-50/year | Consumer focus |
| **xrdp** | Free | $0 | Open source but X11 only |
| **GNOME RD** | Free | $0 | Open source but GNOME only |
| **lamco-rdp-server** | Per server | **$49.99/year** | **Most affordable commercial Wayland option** |

**Competitive advantage:** Lowest price + Native Wayland + Future open source

---

## IronRDP DEPENDENCY STATUS

**Current:** Using git patches to pull IronRDP master

```toml
[patch.crates-io]
ironrdp = { git = "https://github.com/Devolutions/IronRDP", branch = "master" }
# ... all IronRDP crates
```

**Why:** Need EGFX support from PR #1057 (still under review)

**When PR #1057 merges:**
1. IronRDP will publish new version (e.g., 0.6.0)
2. Remove entire [patch.crates-io] section
3. Update dependencies to crates.io versions
4. Test EGFX thoroughly
5. Publish updated lamco-rdp-server

**Timeline:** Waiting on IronRDP maintainers (external blocker)

---

## COMPOSITOR CODE LOCATIONS (PRECISE)

### Implementation #1: VDI Core Compositor ⭐

**Branch:** `feature/lamco-compositor-clipboard`
**Smithay Version:** 0.7.0 (latest)
**Location:** src/compositor/ (30+ files, ~4,586 lines)
**Backend:** X11 (for testing) + Headless capable
**Status:** Complete ("READY TO SHIP" Nov 20, 2025)
**Purpose:** Future VDI product
**Features:**
- Full Wayland protocol support
- Software renderer
- RDP bridge integration
- Clipboard via Smithay SelectionHandler
- Window management

**DO NOT MERGE TO MAIN** - This is separate VDI product

### Implementation #2: Experimental Alternative

**Branch:** `feature/headless-development`
**Smithay Version:** 0.3 (old)
**Location:** src/compositor/ + src/headless/compositor.rs
**Status:** Experimental (has CCW merge commits)
**Purpose:** Alternative architecture exploration

**PRESERVE BUT LOWER PRIORITY**

### Main Branch: NO COMPOSITOR ✅

**Branch:** main
**Contains:** Portal mode ONLY (no compositor code)
**Ready for:** lamco-rdp-server public repo

---

## FEATURE BOUNDARY DECISIONS (FINAL)

### What's Open Source ✅ (Published)

**Infrastructure crates (20,033 lines):**
- lamco-portal - XDG Portal integration
- lamco-pipewire - PipeWire screen capture
- lamco-video - Video frame processing
- lamco-clipboard-core - Clipboard utilities
- lamco-rdp-clipboard - IronRDP clipboard backend
- lamco-rdp-input - Input event translation

**License:** MIT OR Apache-2.0
**Status:** Published to crates.io
**Anyone can:** Use commercially, modify, redistribute

### What's Source-Available (BSL) - This Product

**lamco-rdp-server (10,831 lines):**
- Server orchestration
- IronRDP integration
- EventMultiplexer (QoS)
- ClipboardManager (state machine)
- Multi-monitor layout
- EGFX video handler
- Configuration system
- All glue code

**License:** BUSL-1.1 (free non-commercial, paid commercial)
**Status:** Ready to publish after LICENSE file created
**Anyone can:** Use non-commercially, see source, modify for personal use
**Businesses must:** Buy license ($49.99/year or $99 perpetual)
**Future:** Apache-2.0 in 3 years

### What's Proprietary (Future)

**VDI compositor (separate product):**
- Code in: `feature/lamco-compositor-clipboard`
- License: TBD (likely higher commercial pricing)
- Status: Complete but not published
- Separate repo when ready

**DO NOT extract more open source** - You've given enough (65% of original codebase)

---

## WORKFLOW SUMMARY

### Development Workflow (Private Repo)

```bash
cd /home/greg/wayland/wrd-server-specs
# Work on main branch (Portal mode)
# Work on compositor branches (VDI mode)
# Commit freely, Claude refs OK here
# This repo stays private
```

### Publication Workflow (Public Repo)

```bash
# When ready to publish version:
1. Ensure main branch clean
2. Export to /tmp/lamco-rdp-server-public
3. Remove private docs
4. Push to github.com/lamco-admin/lamco-rdp-server
5. Publish to crates.io
```

---

## IMMEDIATE NEXT STEPS (START OF NEXT SESSION)

**Do in order:**

1. **Create LICENSE file** with BSL 1.1 + your parameters
2. **Create LICENSE-APACHE** reference file
3. **Update Cargo.toml** license field to BUSL-1.1
4. **Commit** licensing changes
5. **Set up Lemon Squeezy** account and products
6. **Update README.md** with licensing section and Lemon Squeezy links
7. **Create public repo** github.com/lamco-admin/lamco-rdp-server
8. **Export clean code** to public repo
9. **(Optional) Publish to crates.io**

**Time estimate:** 4-6 hours for complete setup

---

## KEY REMINDERS FOR NEXT SESSION

1. **License is BUSL-1.1** with specific Additional Use Grant (see above)
2. **No enforcement** - honor system (no license key validation code needed)
3. **Lemon Squeezy** handles payments: $49.99/year or $99 perpetual
4. **3 year conversion** to Apache-2.0
5. **Contact:** office@lamco.io
6. **Public repo:** github.com/lamco-admin/lamco-rdp-server (CREATE)
7. **NO compositor code** in lamco-rdp-server (Portal mode only)
8. **Compositor code** stays in feature/lamco-compositor-clipboard branch (future VDI product)

---

## CONTEXT FOR NEXT SESSION

**You are working on:** lamco-rdp-server (Portal mode RDP server)

**Current state:**
- Code is production-ready
- Uses published lamco-* v0.2.0 crates
- Zero TODOs
- Tests passing
- Main branch is clean

**Blocking issue:** Need to set up licensing before publication

**Next milestone:** Create LICENSE, set up Lemon Squeezy, create public repo, publish

**DO NOT:**
- Merge compositor code to main
- Extract more open source crates
- Worry about VDI product yet

**DO:**
- Set up BSL 1.1 licensing
- Create Lemon Squeezy products
- Prepare for public release

---

**END OF HANDOVER**
