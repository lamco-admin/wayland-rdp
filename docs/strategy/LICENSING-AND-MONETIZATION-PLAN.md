# Licensing and Monetization Plan

**Generated:** 2025-12-30
**Purpose:** Structure BSL licensing, Lemon Squeezy storefront, and donation model

---

## License Structure (BSL 1.1)

### Who Pays

| Employees | Annual Revenue | License Required? |
|-----------|----------------|-------------------|
| ≤3 | Any | No (free) |
| Any | ≤$1M | No (free) |
| >3 | >$1M | **Yes** |

**Plain English:** Free unless you're a company with both more than 3 employees AND more than $1 million in annual revenue.

### Pricing Options

| Option | Price | What You Get |
|--------|-------|--------------|
| **Annual** | $49/year | License valid for 1 year, renew annually |
| **Perpetual** | $99 one-time | License valid until BSL converts to open source (~3 years) |

**Value proposition for perpetual:** Pay $99 once instead of potentially $147 over 3 years.

### BSL Timeline

```
2025                    2028 (approx)
  │                          │
  ▼                          ▼
┌─────────────────────────────┐
│     BSL Licensed Period     │──────▶ Converts to Apache-2.0/MIT
│   (Commercial use requires  │        (Fully open source)
│    license for large orgs)  │
└─────────────────────────────┘
```

---

## Lemon Squeezy Storefront Setup

### Products to Create

#### 1. Commercial License - Annual ($49/year)

```
Product Name: lamco-rdp-server Commercial License (Annual)
Price: $49.00 USD
Billing: Recurring (yearly)
Description:
  Annual commercial license for lamco-rdp-server.
  Required for organizations with >3 employees AND >$1M annual revenue.
  Includes all features, updates during subscription period.

Checkout fields:
  - Company name (required)
  - Number of servers (optional, for records)
```

#### 2. Commercial License - Perpetual ($99)

```
Product Name: lamco-rdp-server Commercial License (Perpetual)
Price: $99.00 USD
Billing: One-time
Description:
  Perpetual commercial license for lamco-rdp-server.
  Valid until the BSL license converts to open source (approximately 2028).
  Required for organizations with >3 employees AND >$1M annual revenue.
  One payment, no renewals needed.

Checkout fields:
  - Company name (required)
  - Number of servers (optional, for records)
```

#### 3. Support the Project (Donation)

```
Product Name: Support lamco-rdp-server Development
Price: Pay what you want (suggested amounts below)
Billing: One-time

Suggested amounts:
  - $5 - Buy me a coffee
  - $15 - Buy me lunch
  - $50 - Meaningful support
  - $100 - Generous supporter
  - Custom amount

Description:
  Support ongoing development of lamco-rdp-server and the lamco-*
  open source crates. Your contribution helps fund:
  • Continued development and new features
  • Bug fixes and security updates
  • Documentation improvements
  • Open source contributions to IronRDP

  lamco-rdp-server is free for personal use and small businesses.
  Donations help keep it that way.
```

#### 4. Monthly Supporter (Optional - Recurring Donation)

```
Product Name: Monthly Supporter
Price: Pay what you want (minimum $3/month)
Billing: Recurring (monthly)

Suggested amounts:
  - $3/month - Supporter
  - $10/month - Backer
  - $25/month - Sponsor

Description:
  Become a monthly supporter of lamco-rdp-server development.
  Cancel anytime. Recurring support helps with planning and sustainability.
```

---

## Website Integration

### Pricing Page Structure (lamco.ai/pricing)

```markdown
# Pricing

## Free for Most Users

lamco-rdp-server is **free** for:
- Personal and home use
- Small businesses (≤3 employees)
- Companies under $1M annual revenue
- Educational and non-profit use
- Evaluation and testing

No registration, no feature limits, no time limits.

---

## Commercial License

Required only for organizations with **both**:
- More than 3 employees, AND
- More than $1 million in annual revenue

| Plan | Price | Duration |
|------|-------|----------|
| Annual | $49/year | Renews yearly |
| Perpetual | $99 once | Until open source conversion (~2028) |

[Buy Annual License]  [Buy Perpetual License]

Both options include:
✓ All features unlocked
✓ All updates during license period
✓ Email support

---

## Support Development

Even if you don't need a commercial license, you can support
ongoing development:

[♥ Donate]  [Become a Monthly Supporter]

Your support funds continued development, documentation,
and open source contributions.
```

### Download Page (lamco.ai/download)

```markdown
# Download

## Quick Install

**Flatpak (Recommended)**
```
flatpak install flathub ai.lamco.rdp-server
```

**Ubuntu/Debian**
```
wget https://lamco.ai/releases/lamco-rdp-server_1.0.0_amd64.deb
sudo dpkg -i lamco-rdp-server_1.0.0_amd64.deb
```

**Fedora/RHEL**
```
wget https://lamco.ai/releases/lamco-rdp-server-1.0.0.x86_64.rpm
sudo dnf install lamco-rdp-server-1.0.0.x86_64.rpm
```

## All Downloads

| Format | Size | SHA256 |
|--------|------|--------|
| [Flatpak](link) | ~XX MB | `abc123...` |
| [.deb (Ubuntu 22.04+)](link) | ~XX MB | `def456...` |
| [.rpm (Fedora 40+)](link) | ~XX MB | `ghi789...` |
| [Generic Linux (tar.gz)](link) | ~XX MB | `jkl012...` |
| [Source](github-link) | - | - |

## License

lamco-rdp-server is licensed under the Business Source License 1.1.

**Free for:** Personal use, small businesses (≤3 employees),
companies under $1M revenue.

**Commercial license required for:** Organizations with >3 employees
AND >$1M revenue. [View pricing →](/pricing)

---

♥ If lamco-rdp-server is useful to you, consider [supporting development](/pricing#support).
```

---

## Lemon Squeezy Checkout Customization

### Brand Settings

```
Store name: lamco
Store URL: lamco.lemonsqueezy.com
Custom domain: store.lamco.ai (optional, requires DNS setup)

Colors:
  Primary: Match lamco.ai brand

Logo: lamco logo

Receipt email from: sales@lamco.ai or greg@lamco.ai
```

### Checkout Flow

1. User clicks "Buy License" on lamco.ai
2. Redirects to lamco.lemonsqueezy.com/checkout/...
3. User completes payment
4. Receives email receipt with license confirmation
5. (Optional) Redirect back to lamco.ai/thank-you

### License Delivery

**Simple approach (recommended for now):**
- Receipt email serves as license proof
- Include in receipt: "This receipt confirms your commercial license for lamco-rdp-server. Keep this email for your records."

**Future enhancement:**
- Generate license keys via Lemon Squeezy webhooks
- License key validation in software (optional, honor system works too)

---

## Donation Psychology and Placement

### Where to Ask for Donations

| Location | Approach |
|----------|----------|
| **Pricing page** | Dedicated section after license info |
| **Download page** | Gentle reminder at bottom |
| **README** | "Support" section with link |
| **CLI startup** | Optional: "Support development at lamco.ai/donate" (can be disabled) |
| **Documentation** | Footer link |
| **GitHub** | Sponsors button + FUNDING.yml |

### Messaging That Works

**Don't:** "Please donate, I need money"
**Do:** "Your support helps keep this free for everyone"

**Don't:** "Donate $100!"
**Do:** Offer range of amounts, let people choose

**Effective framing:**
- "Buy me a coffee" ($5) - Low barrier, familiar
- "Fund an afternoon of development" ($50)
- "Cover server costs for a month" ($X actual cost)
- "Support the open source ecosystem"

### GitHub Sponsors Alternative

You could also enable GitHub Sponsors alongside Lemon Squeezy:
- Some developers prefer donating through GitHub
- Integrates with profile and repos
- GitHub doesn't take a cut from first year (then 0%)

**FUNDING.yml for repo:**
```yaml
custom:
  - https://lamco.ai/pricing#support
  - https://lamco.lemonsqueezy.com/donate
github: [your-github-username]
```

---

## Revenue Projections (Hypothetical)

### Scenario Analysis

| Scenario | Commercial | Donations | Monthly |
|----------|------------|-----------|---------|
| **Conservative** | 2 perpetual/year ($198) | $20/month | ~$37/month |
| **Moderate** | 10 perpetual/year ($990) | $100/month | ~$183/month |
| **Optimistic** | 5 annual + 20 perpetual ($2,245) | $200/month | ~$387/month |

**Reality check:** Early-stage open source projects typically see:
- Few commercial licenses initially (companies evaluate before buying)
- Donations correlate with visibility/community size
- Growth is slow then accelerates if product gains traction

---

## Implementation Checklist

### Lemon Squeezy Setup

- [ ] Create "Commercial License - Annual" product ($49/year recurring)
- [ ] Create "Commercial License - Perpetual" product ($99 one-time)
- [ ] Create "Support Development" donation product (pay what you want)
- [ ] Optional: Create "Monthly Supporter" product ($3+ recurring)
- [ ] Configure receipt email template
- [ ] Add company name field to license products
- [ ] Test checkout flow

### Website Integration

- [ ] Create /pricing page with license tiers and donation section
- [ ] Add checkout links (Lemon Squeezy hosted checkout URLs)
- [ ] Add donation callout to download page
- [ ] Add "Support" link to navigation/footer

### Repository

- [ ] Add FUNDING.yml with Lemon Squeezy links
- [ ] Update README with support section
- [ ] Add LICENSE file with BSL 1.1 text

### Optional Enhancements

- [ ] Custom domain for Lemon Squeezy (store.lamco.ai)
- [ ] GitHub Sponsors setup
- [ ] Thank you page on lamco.ai for post-purchase redirect
- [ ] Webhook integration for license key generation

---

## Open Questions for You

1. **License keys:** Do you want to generate/validate license keys, or is honor system (receipt = proof) sufficient for now?

2. **Support tier:** Want to offer paid support as a separate product? (e.g., "$199/year priority email support")

3. **Custom domain:** Worth setting up store.lamco.ai, or is lamco.lemonsqueezy.com fine?

4. **Monthly donations:** Include monthly supporter tier, or keep it simple with one-time only?

5. **Volume licensing:** Should there be a discount for companies running many servers? (e.g., "$299 for unlimited servers")

---

## Recommended Immediate Actions

1. **In Lemon Squeezy:**
   - Create the two license products (annual + perpetual)
   - Create the donation product
   - Copy the checkout URLs

2. **Document the license terms:**
   - Create clear LICENSE file with BSL 1.1 text
   - Specify the change date (when it becomes open source)
   - Specify the "Additional Use Grant" (the ≤3 employees / ≤$1M clause)

3. **Add to website:**
   - Pricing page with embedded Lemon Squeezy links
   - Keep it simple initially, enhance later

The beauty of Lemon Squeezy is you don't touch transactions at all - they handle payment processing, receipts, and payouts. You just create products and share links.
