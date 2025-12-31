# Website Content Index

**Generated:** 2025-12-30
**Purpose:** Index of all website content for lamco-rdp-server

---

## Content Files

| File | URL | Purpose |
|------|-----|---------|
| [PRODUCT-PAGE.md](PRODUCT-PAGE.md) | `/products/lamco-rdp-server/` | Main product page |
| [TECHNOLOGY-VIDEO-ENCODING.md](TECHNOLOGY-VIDEO-ENCODING.md) | `/technology/video-encoding/` | H.264, AVC444, encoders |
| [TECHNOLOGY-COLOR-MANAGEMENT.md](TECHNOLOGY-COLOR-MANAGEMENT.md) | `/technology/color-management/` | Color spaces, VUI, accuracy |
| [TECHNOLOGY-PERFORMANCE.md](TECHNOLOGY-PERFORMANCE.md) | `/technology/performance/` | Adaptive FPS, latency, damage tracking |
| [TECHNOLOGY-WAYLAND.md](TECHNOLOGY-WAYLAND.md) | `/technology/wayland/` | Portals, PipeWire, compositors |
| [COMPARISON.md](COMPARISON.md) | `/comparison/` | vs xrdp, VNC, NoMachine |
| [ROADMAP.md](ROADMAP.md) | `/roadmap/` | Planned features, feedback |

### Pricing & Downloads (in parent docs/)

| File | URL | Purpose |
|------|-----|---------|
| [../WEBSITE-PRICING-CONTENT.md](../WEBSITE-PRICING-CONTENT.md) | `/pricing/` | License tiers, donations |
| (to create) | `/download/` | Package downloads |

---

## Site Navigation Update

### Proposed Navigation Structure

```
Home
Products ─────────► QuickCapture
                    WiFi Intelligence
                    lamco-rdp-server ◄── NEW
Developers
Open Source
Technology ◄─────── NEW section
    ├── Video Encoding
    ├── Color Management
    ├── Performance
    └── Wayland Integration
Pricing ◄────────── NEW (or under Products)
Download ◄───────── NEW
About
Contact
```

### Products Page Addition

Add to `/products/` alongside existing products:

```markdown
### Remote Desktop

**lamco-rdp-server**

Wayland RDP Server for Linux. Professional remote desktop with
hardware-accelerated H.264 encoding and crystal-clear text rendering.

- Native Wayland via XDG Portals
- AVC444 for sharp text
- NVENC and VA-API hardware encoding
- Free for personal use

[Learn More →](/products/lamco-rdp-server/)
```

### Open Source Page Update

Update `/open-source/` to reference lamco-rdp-server:

```markdown
## Rust Infrastructure Crates

These memory-safe libraries power [lamco-rdp-server](/products/lamco-rdp-server/),
our Wayland RDP server. They're available under MIT/Apache-2.0 for anyone
building remote desktop or screen capture solutions.
```

---

## Content Checklist

### Product Pages
- [x] Main product page (PRODUCT-PAGE.md)
- [ ] Download page (to create)

### Technology Pages
- [x] Video encoding (TECHNOLOGY-VIDEO-ENCODING.md)
- [x] Color management (TECHNOLOGY-COLOR-MANAGEMENT.md)
- [x] Performance features (TECHNOLOGY-PERFORMANCE.md)
- [x] Wayland integration (TECHNOLOGY-WAYLAND.md)

### Supporting Pages
- [x] Feature comparison (COMPARISON.md)
- [x] Roadmap / feature requests (ROADMAP.md)
- [x] Pricing content (../WEBSITE-PRICING-CONTENT.md)
- [ ] Privacy policy (to create)
- [ ] Getting started guide (documentation)

### Site Updates
- [ ] Add to Products page
- [ ] Add to Open Source page
- [ ] Add Technology nav section
- [ ] Add Pricing nav link
- [ ] Add Download nav link

---

## Content Tone Summary

Based on lamco.ai site analysis:

### Do
- Lead with benefits ("Crystal-clear text")
- Follow with technical depth (tables, specs)
- Use plain language for complex concepts
- Include specific numbers and benchmarks
- Provide configuration examples
- Address different user segments
- Be honest about limitations

### Don't
- Oversell or make vague claims
- Use jargon without explanation
- Bury important information
- Hide pricing
- Overuse emoji

### Headline Patterns
- "Wayland RDP Server for Linux"
- "Crystal-Clear Text. Hardware Accelerated."
- "The Only Wayland-Native RDP Server"

### CTA Patterns
- "Download Free →"
- "View Pricing →"
- "Learn More →"
- "View on crates.io →"

---

## Integration Points

### Lemon Squeezy
- Checkout URLs needed for:
  - Monthly license button
  - Annual license button
  - Perpetual license button
  - Corporate license button
  - Service Provider license button
  - Donation button
  - Monthly supporter button

### GitHub
- FUNDING.yml configured
- GitHub Sponsors profile needed
- Public repo for Issues/Discussions

### Downloads
- Host on lamco.ai/releases/
- Flatpak via Flathub
- .deb and .rpm via OBS

---

## Word Count Summary

| Document | Words | Reading Time |
|----------|-------|--------------|
| Product Page | ~1,200 | 5 min |
| Video Encoding | ~2,000 | 8 min |
| Color Management | ~1,800 | 7 min |
| Performance | ~2,200 | 9 min |
| Wayland Integration | ~1,600 | 6 min |
| Comparison | ~1,500 | 6 min |
| Roadmap | ~1,200 | 5 min |
| **Total** | **~11,500** | **~46 min** |

This is substantial content that positions lamco-rdp-server as a serious, technically sophisticated product with comprehensive documentation.
