# lamco-rdp-server Website Content Development

**Generated:** 2025-12-30
**Purpose:** Content requirements for lamco.ai integration, Lemon Squeezy, GitHub, and release pipeline

---

## Part 1: Website Analysis Summary

### Current Site Structure

```
lamco.ai/
├── / (Home)
├── /products/
│   ├── /quickcapture/
│   └── /wifiintelligence/
│       └── /why/
├── /developers/ (NetKit Kotlin)
├── /open-source/ (Rust crates)
├── /about/
├── /contact/
├── /news/
└── /legal/
    └── /privacy/
        ├── /quickcapture/
        └── /wifiintelligence/
```

### Missing Pages (404)
- `/remote-desktop/` - Referenced but doesn't exist
- `/beta/` - Referenced but doesn't exist
- `/legal/` - No index page

### Brand Voice & Tone

| Attribute | Observation |
|-----------|-------------|
| **Tagline** | "Complex Tech, Simple Tools" |
| **Voice** | Professional but friendly, accessible |
| **Technical depth** | High - includes code samples, specs tables |
| **Emotional appeals** | Yes - "peace of mind", "protect your hustle" |
| **Jargon level** | Minimal - explains terms in plain language |
| **Emoji use** | Sparse - occasional 🧪 📦 in announcements |
| **CTA style** | Action-oriented - "Get Started", "Explore", "Download" |

### Content Patterns

**Product pages include:**
1. Hero section with tagline and key benefits
2. Feature sections with icons/imagery
3. Technical specifications table
4. Pricing tiers (if applicable)
5. Screenshots or demo content
6. FAQ section
7. CTAs throughout

**Pricing presentation:**
- Clear tier comparison tables
- Benefits listed per tier
- One-time vs subscription clearly marked
- Free tier prominently featured

---

## Part 2: New Pages Required

### 2.1 Product Page: /products/lamco-rdp-server/

**URL:** `https://lamco.ai/products/lamco-rdp-server/`

**Purpose:** Primary product page for lamco-rdp-server

**Sections:**

#### Hero Section
```
Headline: Wayland RDP Server for Linux
Subheadline: Professional remote desktop with hardware-accelerated
             H.264 encoding and premium performance features.

CTA Primary: Download Free
CTA Secondary: View Pricing
```

#### Key Benefits (3-4 cards)
```
1. Wayland Native
   First RDP server built for XDG Desktop Portals.
   No X11 dependency, no Xwayland required.

2. Crystal-Clear Text
   AVC444 encoding delivers full 4:4:4 chroma resolution.
   Text and UI elements render sharp, not fuzzy.

3. Hardware Accelerated
   NVIDIA NVENC and Intel/AMD VA-API support.
   Offload encoding to your GPU.

4. Premium Performance
   Adaptive frame rate, latency optimization,
   and predictive cursor technology.
```

#### Feature Grid

| Category | Features |
|----------|----------|
| **Video** | AVC420, AVC444, 5-60 FPS adaptive, BT.709/BT.601 color |
| **Encoders** | OpenH264 (software), NVENC (NVIDIA), VA-API (Intel/AMD) |
| **Input** | Keyboard, mouse, multi-monitor coordinate mapping |
| **Clipboard** | Text, images (PNG/JPEG/DIB), bidirectional sync |
| **Security** | TLS 1.3, PAM authentication |
| **Compositors** | GNOME, KDE Plasma, Sway, Hyprland |

#### Technical Specifications Table
(Use content from VIDEO-CODEC-REFERENCE.md)

#### Pricing Summary
```
Free for personal use and small businesses.
Commercial licenses from $4.99/month.

[View Full Pricing]
```

#### System Requirements
```
- Linux with Wayland compositor
- PipeWire for screen capture
- XDG Desktop Portal support

For hardware encoding:
- NVIDIA: GPU with NVENC, libnvidia-encode
- Intel/AMD: VA-API support, libva
```

#### Compatible Clients
```
- Windows: Built-in Remote Desktop, FreeRDP
- macOS: Microsoft Remote Desktop, FreeRDP
- Linux: FreeRDP, Remmina
- Android: Microsoft Remote Desktop
- iOS: Microsoft Remote Desktop
```

---

### 2.2 Pricing Page: /pricing/ or /products/lamco-rdp-server/pricing/

**URL Option A:** `https://lamco.ai/pricing/` (site-wide, can add other products later)
**URL Option B:** `https://lamco.ai/products/lamco-rdp-server/pricing/` (product-specific)

**Recommendation:** Use `/pricing/` for simplicity and future expansion

**Content:** (From WEBSITE-PRICING-CONTENT.md, adapted to site tone)

#### Hero
```
Headline: Pricing
Subheadline: Free for most users. Commercial licenses for larger organizations.
```

#### Free Use Section
```
Headline: Free for Most Users

lamco-rdp-server is free for:
• Personal and home use
• Non-profit organizations
• Small businesses (3 or fewer employees)
• Companies under $1M annual revenue
• Educational and research use
• Evaluation and testing

No registration. No feature limits. No time limits.
```

#### Commercial Tiers Table

| Plan | Price | Servers | Best For |
|------|-------|---------|----------|
| Monthly | $4.99/mo | 1 | Single server, try before committing |
| Annual | $49/yr | 5 | Small teams |
| Perpetual | $99 | 10 | Growing teams |
| Corporate | $599 | 100 | Enterprise deployment |
| Service Provider | $2,999 | Unlimited | MSPs, VDI providers |

#### Tier Detail Cards
(Expandable or accordion style)

Each tier card includes:
- Price and billing frequency
- Server count
- Key benefits
- [Buy Now] button → Lemon Squeezy checkout

#### License Terms Section
```
What's Included
All licenses include full access to every feature:
✓ AVC420 and AVC444 encoding
✓ Hardware acceleration (NVENC, VA-API)
✓ Premium features (Adaptive FPS, Latency Governor)
✓ Software updates during license period

Non-Competitive Clause
You may not use lamco-rdp-server to create a competing
RDP server product or VDI solution.

Open Source Conversion
On December 31, 2028, lamco-rdp-server converts to Apache-2.0.
Perpetual licenses remain valid—the software becomes free for everyone.
```

#### Support Development Section
```
Headline: Support Development

Even if you don't need a commercial license, you can support
ongoing development of lamco-rdp-server and the lamco-*
open source crates.

[Donate One-Time] [Become Monthly Supporter]
```

#### FAQ
(From WEBSITE-PRICING-CONTENT.md)

---

### 2.3 Download Page: /download/ or /products/lamco-rdp-server/download/

**URL Recommendation:** `https://lamco.ai/download/`

#### Hero
```
Headline: Download lamco-rdp-server
Subheadline: Choose your installation method
```

#### Quick Install Section
```
Flatpak (Recommended)
flatpak install flathub ai.lamco.rdp-server

Ubuntu/Debian
wget https://lamco.ai/releases/lamco-rdp-server_X.X.X_amd64.deb
sudo dpkg -i lamco-rdp-server_X.X.X_amd64.deb

Fedora/RHEL
wget https://lamco.ai/releases/lamco-rdp-server-X.X.X.x86_64.rpm
sudo dnf install lamco-rdp-server-X.X.X.x86_64.rpm
```

#### All Downloads Table

| Format | Version | Size | Download | Checksum |
|--------|---------|------|----------|----------|
| Flatpak | X.X.X | ~XX MB | [Install] | SHA256 |
| .deb (Ubuntu 22.04+) | X.X.X | ~XX MB | [Download] | SHA256 |
| .rpm (Fedora 40+) | X.X.X | ~XX MB | [Download] | SHA256 |
| Generic (tar.gz) | X.X.X | ~XX MB | [Download] | SHA256 |
| Source | X.X.X | - | [GitHub] | - |

#### Hardware Encoding Setup
```
NVIDIA (NVENC)
Install NVIDIA drivers with NVENC support and ensure
libnvidia-encode is available.

Intel/AMD (VA-API)
Install VA-API drivers:
• Intel: intel-media-va-driver (Ubuntu) / intel-media-driver (Fedora)
• AMD: mesa-va-drivers

Verify with: vainfo
```

#### Getting Started Link
```
First time? See the [Getting Started Guide →]
```

#### License Reminder
```
lamco-rdp-server is free for personal use and small businesses.
Commercial license required for organizations with >3 employees
AND >$1M revenue. [View Pricing →]
```

---

### 2.4 Privacy Policy: /legal/privacy/lamco-rdp-server/

**Content structure:** (Follow existing QuickCapture/WiFi Intelligence pattern)

```
1. Overview
   - lamco-rdp-server is desktop software that runs locally
   - No data transmitted to Lamco Development
   - No telemetry, no analytics, no phone-home

2. What We Don't Collect
   - Screen content
   - Clipboard data
   - Keystrokes
   - Connection metadata
   - Usage statistics

3. What Stays Local
   - All RDP session data
   - Configuration files
   - Log files (if enabled)
   - Certificate/key material

4. Network Connections
   - RDP clients connect directly to your server
   - Optional: Check for updates (can be disabled)
   - No other outbound connections

5. Third-Party Services
   - None (unless you configure external auth)

6. Contact
   - office@lamco.io
```

---

### 2.5 Update Navigation

**Current nav:** Home | Products | Developers | Open Source | About | Contact

**Add:**
- Products dropdown should include "lamco-rdp-server"
- Consider adding "Pricing" to main nav (or keep under Products)
- Consider adding "Download" to main nav

**Products page** (`/products/`) needs:
- New card for lamco-rdp-server alongside QuickCapture and WiFi Intelligence

---

### 2.6 Open Source Page Update

**Current `/open-source/`** lists 5 Rust crates.

**Update to add context:**
```
These crates power lamco-rdp-server, our Wayland RDP server.
They're available for anyone building remote desktop infrastructure.
```

**Add link:** "See lamco-rdp-server →"

---

## Part 3: Lemon Squeezy Integration

### 3.1 Products to Create in Lemon Squeezy Dashboard

| Product Name | Price | Type | Checkout Fields |
|--------------|-------|------|-----------------|
| Commercial License (Monthly) | $4.99 | Subscription (monthly) | Company name |
| Commercial License (Annual) | $49 | Subscription (yearly) | Company name |
| Commercial License (Perpetual) | $99 | One-time | Company name |
| Corporate License (Perpetual) | $599 | One-time | Company name, Contact email |
| Service Provider License | $2,999 | One-time | Company name, Contact email, Business description |
| Support Development | Pay what you want | One-time | (none required) |
| Monthly Supporter | $4.99+ | Subscription (monthly) | (none required) |

### 3.2 Checkout URLs

After creating products, Lemon Squeezy provides checkout URLs like:
```
https://lamco.lemonsqueezy.com/checkout/buy/abc123...
```

**Website integration:**
- "Buy" buttons link directly to these checkout URLs
- Customer completes payment on Lemon Squeezy's hosted checkout
- Customer receives receipt email
- You receive notification + payout

### 3.3 Lemon Squeezy Store Settings

```
Store Name: Lamco Development
Store URL: lamco.lemonsqueezy.com
Support Email: office@lamco.io

Receipt Template:
- Include: "This receipt confirms your commercial license
  for lamco-rdp-server. Keep this email for your records."
- Include: License terms summary
- Include: Contact for questions

Optional: Custom domain (store.lamco.ai)
- Requires DNS CNAME record
- Provides branded checkout URL
```

### 3.4 Webhook Integration (Optional, for license keys)

If you later want to generate license keys:
```
Lemon Squeezy → Webhook → Your endpoint → Generate key → Email to customer
```

Not needed for honor system.

---

## Part 4: GitHub Integration

### 4.1 FUNDING.yml

**File:** `.github/FUNDING.yml` (already created)

```yaml
custom:
  - https://lamco.ai/pricing
  - https://lamco.lemonsqueezy.com

# Uncomment when GitHub Sponsors is set up:
# github: [your-username]
```

### 4.2 GitHub Sponsors Setup

**Steps:**
1. Go to https://github.com/sponsors
2. Click "Get started" or "Join the waitlist"
3. Complete tax information (W-9 for US)
4. Set up tiers (suggested):

| Tier | Price | Description |
|------|-------|-------------|
| ☕ Coffee | $5/mo | Support development |
| 🍕 Lunch | $15/mo | Meaningful support |
| 🚀 Sponsor | $50/mo | Generous supporter |
| 💎 Champion | $100/mo | Major supporter |

5. Write sponsor profile (use About page content)
6. Add to FUNDING.yml

**Benefits:**
- 0% fees (GitHub waives them)
- "Sponsor" button on repos
- Sponsor badge for supporters
- Trusted by developers

### 4.3 Repository Setup (for public lamco-rdp-server repo)

When you create the public repo:

```
README.md - Include:
- Badges (build status, version, license)
- Quick description
- Installation section
- "Support" section with funding links

LICENSE - BSL 1.1 (already drafted)

.github/
  FUNDING.yml
  ISSUE_TEMPLATE/
  PULL_REQUEST_TEMPLATE.md
```

---

## Part 5: Release Pipeline

### 5.1 Open Build Service (OBS)

**Yes, openSUSE's Open Build Service is the best option for multi-distro packaging.**

**What OBS provides:**
- Build .deb, .rpm, Arch, and more from single source
- Automated rebuilds when dependencies update
- Public download repositories
- Professional infrastructure (used by openSUSE, Packman, etc.)

**Setup:**
1. Create account at https://build.opensuse.org
2. Create project (e.g., `home:lamco/lamco-rdp-server`)
3. Upload spec file (.spec for RPM, debian/ for deb)
4. OBS builds for all configured targets

**Target distributions:**
```
openSUSE_Tumbleweed
openSUSE_Leap_15.5
openSUSE_Leap_15.6
Fedora_40
Fedora_41
Debian_12
xUbuntu_22.04
xUbuntu_24.04
Arch
```

### 5.2 Flatpak

**Flathub submission:**
1. Create Flatpak manifest (YAML)
2. Submit PR to https://github.com/flathub/flathub
3. Flathub reviews and builds
4. Published to Flathub for `flatpak install`

**App ID:** `ai.lamco.rdp-server`

### 5.3 Release Workflow

```
Development
    │
    ▼
Tag version (git tag vX.X.X)
    │
    ├──────────────────────────────────────┐
    │                                      │
    ▼                                      ▼
OBS builds                           Flatpak build
(.deb, .rpm)                         (Flathub)
    │                                      │
    ▼                                      │
Download to lamco.ai/releases/             │
    │                                      │
    ▼                                      ▼
Update /download/ page               Auto-published
    │                                 to Flathub
    ▼
GitHub Release
(source tarball, changelog)
```

### 5.4 VM Setup for Building (Optional)

If you want local build capability before OBS:

**openSUSE Tumbleweed VM:**
```bash
# Install build tools
sudo zypper install osc build rpm-build dpkg

# Configure OBS credentials
osc config

# Build locally
osc build openSUSE_Tumbleweed x86_64
```

**Alternative: GitHub Actions**
- Build on push/tag
- Upload artifacts to GitHub Releases
- Sync to lamco.ai

---

## Part 6: Content Checklist

### Website Pages

- [ ] `/products/lamco-rdp-server/` - Main product page
- [ ] `/pricing/` - Pricing page with tiers
- [ ] `/download/` - Download page with packages
- [ ] `/legal/privacy/lamco-rdp-server/` - Privacy policy
- [ ] Update `/products/` - Add lamco-rdp-server card
- [ ] Update `/open-source/` - Reference lamco-rdp-server

### Lemon Squeezy

- [ ] Create Monthly License product ($4.99/mo)
- [ ] Create Annual License product ($49/yr)
- [ ] Create Perpetual License product ($99)
- [ ] Create Corporate License product ($599)
- [ ] Create Service Provider License product ($2,999)
- [ ] Create Support Development donation product
- [ ] Create Monthly Supporter subscription
- [ ] Configure receipt email template
- [ ] Get checkout URLs for website buttons

### GitHub

- [ ] FUNDING.yml in repo (done)
- [ ] Set up GitHub Sponsors profile
- [ ] Add sponsor tiers
- [ ] Update FUNDING.yml with github username

### Release Infrastructure

- [ ] Create OBS account
- [ ] Set up OBS project
- [ ] Create .spec file for RPM
- [ ] Create debian/ directory for deb
- [ ] Create Flatpak manifest
- [ ] Submit to Flathub
- [ ] Set up /releases/ directory on lamco.ai
- [ ] Create release script/workflow

### Documentation

- [ ] Getting Started guide
- [ ] Configuration reference
- [ ] Hardware encoding setup guide
- [ ] Troubleshooting guide

---

## Part 7: Content Tone Guidelines

Based on existing lamco.ai content:

### Do
- Lead with benefits, follow with technical details
- Use plain language to explain complex concepts
- Include specific numbers (fps, latency, test counts)
- Provide code examples where relevant
- Use tables for specifications and comparisons
- Address different user segments explicitly
- Include "What's included" lists

### Don't
- Use excessive jargon without explanation
- Make vague claims without backing them up
- Overuse emoji (occasional is fine)
- Write walls of text without structure
- Hide pricing or make it confusing
- Forget accessibility considerations

### Voice Examples

**Good (matches site):**
> "lamco-rdp-server is free for personal use and small businesses.
> No registration, no feature limits, no time limits."

**Too corporate:**
> "Enterprise-grade remote desktop solution delivering
> industry-leading performance metrics."

**Too casual:**
> "This thing is awesome! You're gonna love it! 🎉🎉🎉"

### Headline Patterns

From existing site:
- "Scan. Speak. Secure." (QuickCapture)
- "Professional Kotlin WiFi Analysis" (NetKit)
- "Memory-Safe. Async-First. Tested." (Rust crates)

For lamco-rdp-server:
- "Wayland RDP Server for Linux"
- "Remote Desktop, Native to Wayland"
- "Crystal-Clear Text. Hardware Accelerated."
