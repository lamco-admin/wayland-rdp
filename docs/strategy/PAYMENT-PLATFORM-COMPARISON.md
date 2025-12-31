# Payment Platform Comparison

**Generated:** 2025-12-30
**Purpose:** Evaluate options for accepting license payments and donations

---

## What You Need

| Requirement | Priority |
|-------------|----------|
| Accept one-time payments ($99, $599, $2999) | Must have |
| Accept subscriptions ($4.99/mo, $49/yr) | Must have |
| Accept donations (one-time, monthly) | Must have |
| Not handle tax/VAT compliance yourself | Strong preference |
| Minimal setup/maintenance | Strong preference |
| Honor system (no license key generation) | Yes |
| Low fees | Nice to have |

---

## The Key Decision: Who Handles Taxes?

### Option A: Merchant of Record (MoR)

**How it works:** The platform is the legal seller. They handle:
- Sales tax (US states)
- VAT (EU, UK)
- GST (Australia, etc.)
- Invoicing
- Chargebacks
- Refunds

**You receive:** Net payout after fees and taxes

**Platforms:** Lemon Squeezy, Paddle, Gumroad

**Fees:** 5-10%

**Effort:** Minimal - create products, share links

### Option B: Payment Processor (Direct)

**How it works:** YOU are the seller. The platform just processes cards.

**You handle:**
- Sales tax registration and remittance
- VAT MOSS registration (if selling to EU)
- Invoicing
- Tax reporting

**Platforms:** Stripe, PayPal

**Fees:** ~3%

**Effort:** Significant tax/legal overhead

---

## Platform Comparison

### Lemon Squeezy (Your Current Choice)

| Aspect | Details |
|--------|---------|
| Model | Merchant of Record |
| Fees | 5% + $0.50 per transaction |
| Subscriptions | Yes |
| One-time | Yes |
| Donations | Yes (pay-what-you-want products) |
| Tax handling | Full (US, EU, worldwide) |
| Checkout | Hosted pages, embeddable |
| Payout | Weekly or monthly |

**Pros:**
- Already have account set up
- Full tax compliance handled
- Clean checkout experience
- Good for software products
- Affiliate system available

**Cons:**
- Higher fees than direct Stripe
- Newer company (founded 2021)

**Your cost on $99 sale:** ~$5.45 (5% + $0.50)
**Your cost on $4.99/mo sub:** ~$0.75/mo (5% + $0.50)

---

### Paddle

| Aspect | Details |
|--------|---------|
| Model | Merchant of Record |
| Fees | 5% + $0.50 per transaction |
| Subscriptions | Yes |
| One-time | Yes |
| Donations | Not really designed for this |
| Tax handling | Full |
| Checkout | Hosted overlay |

**Pros:**
- Established (used by many software companies)
- Good subscription management
- License key generation built-in

**Cons:**
- Approval process (they review your product)
- More enterprise-focused
- Not great for donations
- Similar fees to Lemon Squeezy

**Verdict:** Similar to Lemon Squeezy, but with approval friction and less donation-friendly.

---

### Gumroad

| Aspect | Details |
|--------|---------|
| Model | Merchant of Record |
| Fees | 10% flat |
| Subscriptions | Yes (called "memberships") |
| One-time | Yes |
| Donations | Yes |
| Tax handling | Full |
| Checkout | Hosted pages |

**Pros:**
- Very simple to use
- Good for creators
- Built-in audience features

**Cons:**
- 10% fee is high
- More creator/content focused than software
- Less professional appearance

**Your cost on $99 sale:** $9.90 (10%)

**Verdict:** Too expensive, wrong vibe for professional software.

---

### Stripe (Direct)

| Aspect | Details |
|--------|---------|
| Model | Payment Processor |
| Fees | 2.9% + $0.30 per transaction |
| Subscriptions | Yes (Stripe Billing) |
| One-time | Yes |
| Donations | Yes |
| Tax handling | NO - you handle it |
| Checkout | Stripe Checkout (hosted) or Payment Links |

**Pros:**
- Lowest fees
- Most flexible
- Payment Links = no code needed
- Professional

**Cons:**
- YOU must handle tax compliance
- EU VAT alone is complex (register in each country or use VAT MOSS)
- US sales tax varies by state
- More admin work

**Your cost on $99 sale:** ~$3.17 (2.9% + $0.30)

**The tax problem:**
- Selling to EU? Need to charge and remit VAT (20%+ depending on country)
- Selling to US? Some states require sales tax collection
- This is why MoR services exist

**Verdict:** Best fees, but tax burden may not be worth it for your volume.

---

### Stripe + Tax Service (Hybrid)

You could use Stripe + a tax automation service:

| Service | Cost |
|---------|------|
| Stripe Tax | 0.5% per transaction |
| TaxJar | $19-99/month |
| Avalara | Enterprise pricing |

**Total:** ~3.4% + tax service fees + YOU still file returns

**Verdict:** Complexity not worth it unless you're doing significant volume.

---

### Ko-fi

| Aspect | Details |
|--------|---------|
| Model | Payment Processor (PayPal/Stripe underneath) |
| Fees | 0% platform fee (just PayPal/Stripe fees ~3%) |
| Subscriptions | Yes ("Ko-fi Gold" members can offer) |
| One-time | Yes |
| Donations | Yes - this is their focus |
| Tax handling | No |

**Pros:**
- 0% platform fee on donations
- Good donation UX ("Buy me a coffee")
- Simple

**Cons:**
- YOU handle taxes
- Not designed for commercial licenses
- Less professional for software sales
- Limited product features

**Verdict:** Good for donations only, not for license sales.

---

### GitHub Sponsors

| Aspect | Details |
|--------|---------|
| Model | Donation platform |
| Fees | 0% (GitHub waives all fees) |
| Subscriptions | Yes (monthly tiers) |
| One-time | Yes |
| Tax handling | Partial (1099 for US) |

**Pros:**
- Zero fees
- Integrated with GitHub
- Developers trust it
- Good for open source credibility

**Cons:**
- Only for sponsorship/donations, NOT product sales
- Can't sell commercial licenses here
- Requires GitHub account to sponsor

**Verdict:** Use alongside your main payment platform for donations only.

---

### Polar.sh

| Aspect | Details |
|--------|---------|
| Model | Hybrid (MoR for some features) |
| Fees | 5% |
| Subscriptions | Yes |
| One-time | Yes |
| Donations | Yes |
| Tax handling | Yes (MoR) |

**Pros:**
- Built specifically for open source
- Issue funding features
- Modern, developer-focused
- Growing community

**Cons:**
- Newer platform
- Smaller than alternatives
- More focused on pure open source than BSL

**Verdict:** Interesting alternative, worth watching, but Lemon Squeezy is more established.

---

## Recommendation

### Best Overall: Stick with Lemon Squeezy

**Why:**
1. You already have the account
2. MoR means zero tax headaches
3. Handles all your use cases (licenses + donations + subscriptions)
4. Reasonable fees for your volume
5. Professional checkout experience

**Add GitHub Sponsors** for additional donation surface area (0% fees, developer trust).

### Fee Reality Check

At your price points, the fee difference is modest:

| Sale | Lemon Squeezy | Stripe Direct | Savings |
|------|---------------|---------------|---------|
| $4.99/mo | $0.75 | $0.44 | $0.31/mo |
| $49/yr | $2.95 | $1.72 | $1.23/yr |
| $99 | $5.45 | $3.17 | $2.28 |
| $599 | $30.45 | $17.67 | $12.78 |
| $2,999 | $150.45 | $87.27 | $63.18 |

The tax compliance burden of direct Stripe isn't worth $2-60 per sale unless you're doing high volume.

---

## Simplest Possible Setup

If you want absolute minimum complexity:

### For Licenses (Lemon Squeezy)
1. Create 5 products in Lemon Squeezy
2. Get checkout URLs
3. Put links on lamco.ai/pricing

### For Donations (Two Options)

**Option A: Also Lemon Squeezy**
- Create "Support Development" pay-what-you-want product
- One platform for everything

**Option B: GitHub Sponsors + Ko-fi**
- GitHub Sponsors for developer donations (0% fee)
- Ko-fi for general donations (0% platform fee)
- Lemon Squeezy for licenses only

Option A is simpler. Option B costs slightly less on donations.

---

## Action Plan

### Immediate
1. Keep Lemon Squeezy for licenses and donations (already set up)
2. Create the 5 license products + donation product
3. Enable GitHub Sponsors (takes ~10 min, 0% fees, adds credibility)

### Optional Later
- Add Ko-fi if you want a "buy me a coffee" style button
- Consider Polar.sh if it gains more traction

### Skip
- Direct Stripe (tax burden not worth it)
- Paddle (similar to LS, but approval friction)
- Gumroad (10% too expensive)
