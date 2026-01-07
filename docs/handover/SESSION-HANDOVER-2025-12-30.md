# Session Handover: lamco-rdp-server Launch Preparation

**Date:** 2025-12-30
**Session Focus:** Documentation cleanup, website content, monetization setup
**Status:** Ready for Lemon Squeezy setup and website publishing

---

## Executive Summary

This session prepared lamco-rdp-server for public launch by:
1. Cleaning up and archiving 350+ documentation files
2. Creating comprehensive website content (~11,500 words)
3. Defining licensing/pricing structure (BSL 1.1, 5 commercial tiers)
4. Drafting Lemon Squeezy and GitHub Sponsors setup guides

**Next immediate actions:** Set up Lemon Squeezy products, publish website content

---

## Project Context

### What Is lamco-rdp-server?

A **Wayland-native RDP server for Linux** built on XDG Desktop Portals. Key differentiators:
- First RDP server using native Wayland capture (not X11/Xwayland)
- AVC444 encoding for crystal-clear text (4:4:4 chroma)
- Hardware acceleration: NVENC (NVIDIA) + VA-API (Intel/AMD)
- Premium features: Adaptive FPS, Latency Governor, Predictive Cursor
- Service Advertisement Registry for capability discovery

### Repository Structure

```
/home/greg/wayland/wrd-server-specs/
├── src/                    # Rust source code
├── docs/                   # Canonical documentation
│   ├── website/            # NEW: Website content drafts
│   ├── strategy/           # NEW: Business/monetization docs
│   └── architecture/       # Technical architecture docs
├── archive/                # Archived old docs, logs, scripts
├── scripts/                # Build/test scripts
└── .github/
    └── FUNDING.yml         # GitHub funding links
```

### Related Repositories/Workspaces

- **lamco-wayland/** - lamco-portal, lamco-pipewire, lamco-video crates
- **lamco-rdp-workspace/** - lamco-rdp-input, lamco-rdp-clipboard crates
- All crates published to crates.io under `lamco-*`

---

## What Was Accomplished This Session

### 1. Documentation Cleanup

**Commits:**
- `27e8475` - docs: reorganize and archive documentation (192 files)
- `b0fa56d` - chore: clean up root directory and archive logs/old files (44 files)

**Result:**
- 6 root docs (README, INSTALL, etc.)
- 54 canonical docs in docs/
- 285 archived files in archive/
- 45 log files (~1GB) archived

### 2. Codebase Analysis Documents Created

| Document | Purpose |
|----------|---------|
| `docs/CODEBASE-REALITY-CHECK.md` | Accurate inventory of implemented features |
| `docs/STRATEGY-COMPLIANCE-SUMMARY.md` | Plan vs implementation comparison |
| `docs/VIDEO-CODEC-REFERENCE.md` | Consolidated codec documentation |

### 3. Website Content Created

**Location:** `docs/website/`

| File | URL Target | Words |
|------|------------|-------|
| `PRODUCT-PAGE.md` | `/products/lamco-rdp-server/` | ~1,200 |
| `TECHNOLOGY-VIDEO-ENCODING.md` | `/technology/video-encoding/` | ~2,000 |
| `TECHNOLOGY-COLOR-MANAGEMENT.md` | `/technology/color-management/` | ~1,800 |
| `TECHNOLOGY-PERFORMANCE.md` | `/technology/performance/` | ~2,200 |
| `TECHNOLOGY-WAYLAND.md` | `/technology/wayland/` | ~1,600 |
| `COMPARISON.md` | `/comparison/` | ~1,500 |
| `ROADMAP.md` | `/roadmap/` | ~1,200 |
| `INDEX.md` | (reference) | - |

**Also:**
- `docs/WEBSITE-PRICING-CONTENT.md` - Pricing page content
- `docs/WEBSITE-CONTENT-OUTLINE.md` - Site structure
- `docs/WEBSITE-CONTENT-DEVELOPMENT.md` - Full content requirements

**Total:** ~11,500 words of technical marketing content

### 4. Business/Monetization Documents Created

**Location:** `docs/strategy/`

| File | Purpose |
|------|---------|
| `DISTRIBUTION-AND-MARKET-ANALYSIS.md` | Packaging, distros, markets analysis |
| `LICENSING-AND-MONETIZATION-PLAN.md` | BSL license, pricing strategy |
| `COMMERCIAL-LICENSE-TIERS.md` | Detailed tier breakdown |
| `PAYMENT-PLATFORM-COMPARISON.md` | Lemon Squeezy vs alternatives |
| `LEMONSQUEEZY-SETUP-GUIDE.md` | Step-by-step product creation |
| `GITHUB-SPONSORS-PROFILE.md` | Sponsor tiers and profile content |

### 5. Zip Archive for Website Process

**File:** `lamco-rdp-server-website-content.zip` (57 KB)
**Contains:** All 15 website and setup documents

---

## Key Decisions Made

### Licensing: BSL 1.1

- **Change Date:** December 31, 2028 (becomes Apache-2.0)
- **Free Use:** Personal, non-profit, ≤3 employees, <$1M revenue
- **Commercial Required:** >3 employees AND >$1M revenue
- **Non-Competitive Clause:** Cannot create competing RDP/VDI product
- **License file:** Already exists at `/LICENSE`

### Pricing Tiers

| Tier | Price | Servers | Target |
|------|-------|---------|--------|
| Monthly | $4.99/mo | 1 | Try before commit |
| Annual | $49/yr | 5 | Small teams |
| Perpetual | $99 | 10 | Growing teams |
| Corporate | $599 | 100 | Enterprise |
| Service Provider | $2,999 | Unlimited | MSPs, VDI providers |

**Plus donations:** One-time (PWYW) and Monthly Supporter ($5+/mo)

### Payment Platforms

- **Lemon Squeezy:** All license sales and donations (MoR = they handle taxes)
- **GitHub Sponsors:** Developer donations (0% fees, adds credibility)
- **Store URL:** lamco.lemonsqueezy.com (account exists, products not yet created)

### Distribution Strategy

- **Primary:** Flatpak (Flathub)
- **Secondary:** .deb/.rpm via OBS (Open Build Service)
- **Downloads:** Host on lamco.ai (8TB bandwidth available)
- **Release pipeline:** OBS for multi-distro builds

### Support

- **Email:** office@lamco.io
- **Priority support:** For paid license holders (not further specified)

---

## Pending Tasks

### Immediate (Before Launch)

#### Lemon Squeezy Setup
- [ ] Create 7 products per `LEMONSQUEEZY-SETUP-GUIDE.md`
- [ ] Configure receipt messages
- [ ] Get checkout URLs
- [ ] Test one purchase

#### Website Publishing
- [ ] Publish content from `docs/website/` to lamco.ai
- [ ] Create `/products/lamco-rdp-server/` page
- [ ] Create `/pricing/` page
- [ ] Create `/download/` page
- [ ] Create `/technology/` section (4 pages)
- [ ] Create `/comparison/` page
- [ ] Create `/roadmap/` page
- [ ] Update navigation
- [ ] Update `/products/` to include lamco-rdp-server
- [ ] Update `/open-source/` to reference lamco-rdp-server
- [ ] Create `/legal/privacy/lamco-rdp-server/` privacy policy

#### GitHub Setup
- [ ] Set up GitHub Sponsors profile per `GITHUB-SPONSORS-PROFILE.md`
- [ ] Update FUNDING.yml with GitHub username
- [ ] Decide on public repo timing/location

### Short-Term (Release Infrastructure)

#### Packaging
- [ ] Create OBS account (build.opensuse.org)
- [ ] Set up OBS project for lamco-rdp-server
- [ ] Create .spec file for RPM builds
- [ ] Create debian/ directory for deb builds
- [ ] Create Flatpak manifest
- [ ] Submit to Flathub

#### Downloads
- [ ] Set up /releases/ directory on lamco.ai
- [ ] Create release workflow (tag → build → publish)
- [ ] Generate first release packages

### Medium-Term (Post-Launch)

- [ ] Getting Started documentation
- [ ] Configuration reference documentation
- [ ] Hardware encoding setup guide
- [ ] Troubleshooting guide
- [ ] Announce on Hacker News, Reddit, etc.

---

## Technical Context

### Feature Status (from CODEBASE-REALITY-CHECK.md)

**Complete:**
- Video streaming (Portal → PipeWire → H.264 → RDP)
- AVC420 and AVC444 encoding
- Hardware encoding (NVENC, VA-API)
- Keyboard/mouse input
- Clipboard (text, images, files)
- TLS 1.3 encryption
- PAM authentication
- Adaptive FPS (5-60)
- Latency Governor
- Predictive Cursor
- Damage tracking (SIMD)
- Service Advertisement Registry
- Compositor detection (GNOME, KDE, Sway, Hyprland)

**Partial/Needs Testing:**
- Multi-monitor (code exists, needs validation)
- Dynamic resize (handler exists)
- File clipboard (implemented, needs validation)

**Not Started:**
- Audio playback (RDPSND) - P2 priority
- Microphone input - P3
- Drive redirection - P3

### Key Source Files

| Area | Location |
|------|----------|
| Config system | `src/config/types.rs` (800+ lines) |
| Video encoding | `src/egfx/` (8 files, ~230KB) |
| Hardware encode | `src/egfx/hardware/nvenc/`, `src/egfx/hardware/vaapi/` |
| Color management | `src/egfx/color_space.rs`, `src/egfx/color_convert.rs` |
| Multi-monitor | `src/multimon/` (1,584 lines) |
| Performance | `src/performance/` |
| Cursor | `src/cursor/` |
| Services | `src/services/` |
| Compositor | `src/compositor/` |

### Dependencies (Published Crates)

| Crate | Purpose | crates.io |
|-------|---------|-----------|
| lamco-portal | XDG Portal integration | ✓ Published |
| lamco-pipewire | PipeWire capture | ✓ Published |
| lamco-video | Frame processing | ✓ Published |
| lamco-rdp-input | Input translation | ✓ Published |
| lamco-clipboard-core | Clipboard primitives | ✓ Published |
| lamco-rdp-clipboard | RDP clipboard | ✓ Published |

### IronRDP Contributions

- PR #1053: Clipboard fix (merged)
- PR #1057: EGFX/H.264 support (open)
- PRs #1063-1066: Clipboard file transfer (open)
- Issue #1067: ZGFX compression (ready to submit)

---

## Important Files Reference

### Website Content (for publishing)
```
docs/website/
├── PRODUCT-PAGE.md
├── TECHNOLOGY-VIDEO-ENCODING.md
├── TECHNOLOGY-COLOR-MANAGEMENT.md
├── TECHNOLOGY-PERFORMANCE.md
├── TECHNOLOGY-WAYLAND.md
├── COMPARISON.md
├── ROADMAP.md
└── INDEX.md

docs/WEBSITE-PRICING-CONTENT.md
```

### Setup Guides
```
docs/strategy/
├── LEMONSQUEEZY-SETUP-GUIDE.md    # Step-by-step Lemon Squeezy
├── GITHUB-SPONSORS-PROFILE.md     # GitHub Sponsors content
├── COMMERCIAL-LICENSE-TIERS.md    # License details
└── LICENSING-AND-MONETIZATION-PLAN.md
```

### Reference Documents
```
docs/
├── CODEBASE-REALITY-CHECK.md      # Feature inventory
├── STRATEGY-COMPLIANCE-SUMMARY.md # Plan vs implementation
├── VIDEO-CODEC-REFERENCE.md       # Codec documentation
├── WEBSITE-CONTENT-OUTLINE.md     # Site structure
└── WEBSITE-CONTENT-DEVELOPMENT.md # Full requirements

docs/strategy/
├── DISTRIBUTION-AND-MARKET-ANALYSIS.md
└── PAYMENT-PLATFORM-COMPARISON.md
```

### Zip Archive
```
lamco-rdp-server-website-content.zip  # All content for website process
```

---

## Session Notes

### Brand Voice (from lamco.ai analysis)
- Tagline: "Complex Tech, Simple Tools"
- Tone: Professional but friendly, accessible
- Technical depth with plain language explanations
- Benefits first, specs second
- Honest about limitations

### Key Differentiators to Emphasize
1. "The Only Wayland-Native RDP Server" - factually true
2. AVC444 for text clarity - visible difference vs competitors
3. Service Advertisement Registry - graceful capability discovery
4. Hardware encoding with full VUI color support
5. Premium features (Adaptive FPS, Latency Governor, Predictive Cursor)

### Support Email
- **office@lamco.io** - All support and sales inquiries
- Priority response for paid license holders

---

## Next Session Recommendations

1. **Start with Lemon Squeezy** - Follow `LEMONSQUEEZY-SETUP-GUIDE.md` to create products
2. **Then website** - Publish content, will need checkout URLs from step 1
3. **Then GitHub Sponsors** - Can be done in parallel, lower priority than licenses
4. **Then packaging** - OBS setup for release infrastructure

The content is ready; execution is what remains.

---

## Questions to Resolve (Not Blocking)

1. **Public repo:** When/where to publish lamco-rdp-server source? (GitHub? Own git server?)
2. **Version number:** What version for first public release? (1.0.0? 0.9.0?)
3. **Thank you page:** Create lamco.ai/thank-you/ for post-purchase redirect?
4. **Custom domain:** Set up store.lamco.ai for Lemon Squeezy? (Optional)

---

*End of handover document*
