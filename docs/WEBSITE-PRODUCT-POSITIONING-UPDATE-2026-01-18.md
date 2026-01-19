# Website Content: Product Positioning Update
**Date:** 2026-01-18
**Purpose:** Clarify lamco-rdp-server as desktop sharing vs future lamco-VDI headless platform
**Format:** Copy-paste ready content for website team

---

## Update 1: Product Description Clarification

### Location: Product Page → Top Section (After title/hero)

### Add This Paragraph:

```html
<div class="product-positioning">
  <p><strong>lamco-rdp-server provides remote access to your existing Linux desktop session.</strong> Connect to your GNOME, KDE, or wlroots workstation via RDP and interact with your applications remotely. Perfect for remote work, system administration, and accessing your desktop from anywhere.</p>

  <p><em>Looking for headless multi-user VDI?</em> <strong>lamco-VDI</strong> (in development) will provide enterprise-grade virtual desktop infrastructure with Smithay-based custom compositor, multi-session support, and true headless deployment. <a href="#desktop-vs-vdi">Learn about the differences →</a></p>
</div>
```

---

## Update 2: NEW SECTION - Desktop Sharing vs Headless VDI

### Location: Add new section after "Clipboard Availability" section

### Section Title:
```
## Desktop Sharing vs Headless VDI
```

### Section Content:

```html
<div class="product-comparison">
  <h3>Understanding the Difference</h3>

  <p>lamco-rdp-server is designed for <strong>desktop sharing</strong> — remotely accessing an existing Linux desktop session. For enterprise multi-user VDI deployments, we're developing <strong>lamco-VDI</strong>, a separate product built on the same RDP core with a headless multi-session architecture.</p>

  <div class="comparison-grid">
    <div class="product-column current-product">
      <h4>lamco-rdp-server</h4>
      <p class="product-type">Desktop Sharing</p>

      <h5>What It Does</h5>
      <p>Provides remote access to <strong>your existing Linux desktop session</strong>. GNOME, KDE, or wlroots compositor must be running with a user logged in.</p>

      <h5>Architecture</h5>
      <ul>
        <li><strong>Compositor:</strong> Uses your existing desktop (GNOME Mutter, KDE KWin, Sway, Hyprland)</li>
        <li><strong>Session Model:</strong> Single user (the logged-in user's desktop)</li>
        <li><strong>Setup:</strong> Install package, run server, connect via RDP</li>
        <li><strong>Resource Usage:</strong> Desktop environment + RDP server (~800MB+ for GNOME)</li>
      </ul>

      <h5>Perfect For</h5>
      <ul>
        <li>✅ Remote access to your personal workstation</li>
        <li>✅ Working from home, accessing office desktop</li>
        <li>✅ System administration (remote server with DE)</li>
        <li>✅ Technical support / screen sharing</li>
        <li>✅ Development VMs with desktop environment</li>
      </ul>

      <h5>Limitations</h5>
      <ul>
        <li>❌ Requires desktop environment (GNOME/KDE/wlroots)</li>
        <li>❌ Single user per machine</li>
        <li>❌ Cannot run truly headless (needs compositor)</li>
        <li>❌ Session persistence requires deployment-specific strategies</li>
        <li>❌ Not suitable for multi-tenant VDI</li>
      </ul>

      <h5>Status</h5>
      <p class="status-badge available">✅ Available Now — v0.9.0</p>
    </div>

    <div class="product-column future-product">
      <h4>lamco-VDI</h4>
      <p class="product-type">Headless Multi-Session VDI</p>

      <h5>What It Does</h5>
      <p>Enterprise virtual desktop infrastructure with <strong>no desktop environment required</strong>. Deploy on headless servers and provide isolated Linux desktops to multiple concurrent users.</p>

      <h5>Architecture</h5>
      <ul>
        <li><strong>Compositor:</strong> Embedded Smithay compositor (custom, headless)</li>
        <li><strong>Session Model:</strong> Multi-user concurrent (10-50+ per server)</li>
        <li><strong>Setup:</strong> systemd service, PAM authentication, per-user isolation</li>
        <li><strong>Resource Usage:</strong> ~256MB per user session (no DE overhead)</li>
      </ul>

      <h5>Perfect For</h5>
      <ul>
        <li>✅ Enterprise VDI (100+ employees)</li>
        <li>✅ Cloud Linux workspaces (AWS/Azure/GCP)</li>
        <li>✅ Thin client infrastructure</li>
        <li>✅ Multi-tenant SaaS platforms</li>
        <li>✅ CI/CD with GUI testing</li>
        <li>✅ Service provider offerings</li>
      </ul>

      <h5>Advantages</h5>
      <ul>
        <li>✅ True headless (no desktop environment needed)</li>
        <li>✅ Multi-user concurrent sessions</li>
        <li>✅ Zero dialogs always (embedded Portal auto-grants)</li>
        <li>✅ Per-user resource limits (cgroup isolation)</li>
        <li>✅ Container/Kubernetes ready</li>
        <li>✅ Horizontal scaling</li>
      </ul>

      <h5>Status</h5>
      <p class="status-badge in-development">🚧 In Development — Smithay integration active</p>
    </div>
  </div>

  <h4>Technical Implementation Status</h4>

  <p>The Smithay compositor integration and headless architecture components are <strong>actively under development</strong> in dedicated branches. Core components already prototyped:</p>

  <ul>
    <li>Smithay compositor initialization and window management</li>
    <li>Headless rendering backend (virtual framebuffer)</li>
    <li>PipeWire stream producer integration</li>
    <li>Embedded Portal backend design</li>
  </ul>

  <p><strong>Estimated Timeline:</strong> lamco-VDI alpha expected in 6-9 months, leveraging 70% of code from lamco-rdp-server's proven RDP stack.</p>

  <h4>Which Product Do You Need?</h4>

  <table class="decision-table">
    <thead>
      <tr>
        <th>Your Requirement</th>
        <th>Recommended Product</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>Access my personal Linux workstation remotely</td>
        <td><strong>lamco-rdp-server</strong> (available now)</td>
      </tr>
      <tr>
        <td>Share screen for remote support or collaboration</td>
        <td><strong>lamco-rdp-server</strong> (available now)</td>
      </tr>
      <tr>
        <td>Remote access to server with GNOME/KDE installed</td>
        <td><strong>lamco-rdp-server</strong> (available now)</td>
      </tr>
      <tr>
        <td>Single-user development VM with desktop environment</td>
        <td><strong>lamco-rdp-server</strong> (available now)</td>
      </tr>
      <tr>
        <td>Multi-user concurrent sessions on one server</td>
        <td><strong>lamco-VDI</strong> (in development)</td>
      </tr>
      <tr>
        <td>Cloud VDI deployment (AWS/Azure/GCP)</td>
        <td><strong>lamco-VDI</strong> (in development)</td>
      </tr>
      <tr>
        <td>Headless server with no desktop environment</td>
        <td><strong>lamco-VDI</strong> (in development)</td>
      </tr>
      <tr>
        <td>Enterprise VDI (50+ users)</td>
        <td><strong>lamco-VDI</strong> (in development)</td>
      </tr>
      <tr>
        <td>Kubernetes/Docker deployment</td>
        <td><strong>lamco-VDI</strong> (in development)</td>
      </tr>
    </tbody>
  </table>

  <p><a href="/products/lamco-vdi/" class="cta-link">→ Learn more about lamco-VDI</a> (coming soon)</p>
</div>
```

---

## Update 3: Installation Methods - Clarify Session Persistence

### Location: Installation Methods → Flatpak Description

### Current Text:
```
Flatpak (Universal compatibility):
- Works on Ubuntu 22.04/24.04 and all distros
- Sandboxed security
- Automatic Flathub updates
- Software encoding (no GPU acceleration)
```

### Updated Text:
```
Flatpak (Universal compatibility):
- Works on Ubuntu 22.04/24.04 and all distros
- Sandboxed security
- Automatic Flathub updates
- Software encoding (OpenH264, no GPU acceleration)
- Session persistence: Portal-based (GNOME requires permission dialog on each restart; KDE/wlroots support session tokens)
- <strong>Best for:</strong> Desktop users, testing, evaluation, distributions without native packages
- <strong>Not recommended for:</strong> Unattended servers (use native package for zero-dialog operation)
```

---

### Location: Installation Methods → Native Package Description

### Current Text:
```
Native Package (Best for production):
- Hardware acceleration (NVIDIA NVENC, Intel/AMD VA-API)
- Advanced desktop environment integration
- Full systemd support
- Direct filesystem access
- Available: Fedora, RHEL, openSUSE, Debian 13
```

### Updated Text:
```
Native Package (Best for production):
- Hardware acceleration (NVIDIA NVENC, Intel/AMD VA-API)
- Zero-dialog session persistence (Mutter Direct API on GNOME, wlr-direct on Sway/Hyprland)
- Full systemd support
- PAM authentication support
- Direct filesystem access
- <strong>Best for:</strong> Production servers, VMs requiring unattended operation, maximum performance
- <strong>Available:</strong> Fedora 40+, RHEL 9 / AlmaLinux 9, openSUSE Tumbleweed/Leap 15.6, Debian 13
- <strong>Note:</strong> Ubuntu 24.04 / Debian 12 not available (use Flatpak — Rust toolchain incompatibility)
```

---

## Update 4: System Requirements - Desktop Environment Requirement

### Location: System Requirements → Server (Linux) section

### Current Text:
```
Server (Linux):
- Linux with Wayland compositor
- PipeWire (screen capture)
- XDG Desktop Portal support
```

### Updated Text:
```
Server (Linux):
- Linux with Wayland compositor (GNOME, KDE, Sway, Hyprland, etc.)
- <strong>Desktop environment must be running</strong> (user logged in)
- PipeWire (screen capture)
- XDG Desktop Portal support (v4+ recommended, v5 for full clipboard)
- Compositor-specific portal backend (xdg-desktop-portal-gnome, -kde, or -wlr)

<p class="requirement-note"><strong>Note:</strong> lamco-rdp-server shares your existing desktop session. For headless multi-user VDI without desktop environment, see <a href="/products/lamco-vdi/">lamco-VDI</a> (in development).</p>
```

---

## Update 5: Roadmap Page Addition

### Location: Roadmap Page → Add new section

### Section Title:
```
## Future Product: lamco-VDI
```

### Section Content:

```html
<div class="future-product-roadmap">
  <h3>lamco-VDI: Headless Multi-Session VDI Platform</h3>

  <p class="product-intro">While lamco-rdp-server excels at sharing existing desktop sessions, enterprise VDI requires a different architecture. <strong>lamco-VDI</strong> is our upcoming product for true headless multi-user virtual desktop infrastructure.</p>

  <h4>What Makes lamco-VDI Different</h4>

  <table class="architecture-comparison">
    <thead>
      <tr>
        <th>Aspect</th>
        <th>lamco-rdp-server</th>
        <th>lamco-VDI</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td><strong>Desktop Requirement</strong></td>
        <td>Requires GNOME/KDE/wlroots running</td>
        <td>No desktop environment needed (headless)</td>
      </tr>
      <tr>
        <td><strong>User Model</strong></td>
        <td>Single user (shares logged-in user's desktop)</td>
        <td>Multi-user concurrent (10-50+ users per server)</td>
      </tr>
      <tr>
        <td><strong>Compositor</strong></td>
        <td>External (GNOME Mutter, KDE KWin, Sway)</td>
        <td>Embedded Smithay (custom, per-user instances)</td>
      </tr>
      <tr>
        <td><strong>Permission Dialogs</strong></td>
        <td>Portal-based (deployment-dependent)</td>
        <td>Zero dialogs (embedded Portal auto-grants)</td>
      </tr>
      <tr>
        <td><strong>Resource per User</strong></td>
        <td>~800MB+ (full desktop environment)</td>
        <td>~256MB (compositor only)</td>
      </tr>
      <tr>
        <td><strong>Deployment</strong></td>
        <td>User workstations, single-user VMs</td>
        <td>Cloud VMs, container clusters, data centers</td>
      </tr>
      <tr>
        <td><strong>Scaling</strong></td>
        <td>One session per machine</td>
        <td>10-50+ concurrent users per server</td>
      </tr>
    </tbody>
  </table>

  <h4>Shared Technology Foundation</h4>

  <p>Both products leverage the same proven RDP protocol stack:</p>

  <ul>
    <li>✅ IronRDP protocol implementation</li>
    <li>✅ H.264 EGFX video pipeline (AVC420, AVC444)</li>
    <li>✅ Hardware acceleration (NVENC, VA-API)</li>
    <li>✅ Clipboard synchronization</li>
    <li>✅ Input injection (keyboard, mouse)</li>
    <li>✅ Damage tracking and bandwidth optimization</li>
    <li>✅ Service Registry runtime adaptation</li>
  </ul>

  <p><strong>Code Reuse:</strong> lamco-VDI will share ~70% of its codebase with lamco-rdp-server, ensuring the same reliability and performance while adding multi-user headless capabilities.</p>

  <h4>lamco-VDI Key Features (Planned)</h4>

  <div class="vdi-features">
    <div class="feature-item">
      <h5>🖥️ Headless Operation</h5>
      <p>No desktop environment required. Minimal Linux server installation (Ubuntu Server, Debian). Embedded Smithay compositor handles window management without GNOME/KDE overhead.</p>
    </div>

    <div class="feature-item">
      <h5>👥 Multi-User Concurrent Sessions</h5>
      <p>10-50+ users per server instance. Each user gets isolated Wayland compositor with separate environment (UID, HOME, XDG_RUNTIME_DIR). PAM authentication per connection.</p>
    </div>

    <div class="feature-item">
      <h5>🔐 Zero-Dialog Unattended Access</h5>
      <p>Embedded Portal backend auto-grants permissions. No permission dialogs ever (headless environment). Session persistence built-in via credential encryption.</p>
    </div>

    <div class="feature-item">
      <h5>📊 Resource Isolation</h5>
      <p>Per-user cgroup limits (memory, CPU, network). Prevent resource exhaustion from single user. Enforce quotas and fair scheduling.</p>
    </div>

    <div class="feature-item">
      <h5>☁️ Cloud-Native Deployment</h5>
      <p>Single binary with minimal dependencies. Container-friendly (Docker, Kubernetes). Horizontal scaling via load balancer. Auto-scaling based on demand.</p>
    </div>

    <div class="feature-item">
      <h5>🎯 Enterprise Features</h5>
      <p>systemd-logind integration. Session lifecycle management (create, suspend, reconnect, destroy). Audit logging. Prometheus monitoring. LDAP/Active Directory authentication (future).</p>
    </div>
  </div>

  <h4>Technical Implementation</h4>

  <p>lamco-VDI development is actively progressing in dedicated development branches, with core components already prototyped:</p>

  <ul>
    <li><strong>Smithay Integration:</strong> Custom compositor built on Smithay framework (pure Rust)</li>
    <li><strong>Headless Backend:</strong> Virtual framebuffer rendering (no physical display required)</li>
    <li><strong>Embedded Portal:</strong> D-Bus service implementing ScreenCast/RemoteDesktop with auto-grant</li>
    <li><strong>PipeWire Producer:</strong> Stream generation from Smithay framebuffer</li>
    <li><strong>Session Manager:</strong> Multi-user authentication and resource isolation</li>
  </ul>

  <p><strong>Development Timeline:</strong> Alpha release expected in 6-9 months. Beta testing for enterprise pilots 9-12 months. Production v1.0 in 12-15 months.</p>

  <h4>Cost Comparison: lamco-VDI vs Commercial VDI</h4>

  <table class="cost-comparison">
    <thead>
      <tr>
        <th>Solution</th>
        <th>Cost per User/Year</th>
        <th>50 Users</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>Citrix Virtual Apps</td>
        <td>$200-400</td>
        <td>$10,000-20,000</td>
      </tr>
      <tr>
        <td>VMware Horizon</td>
        <td>$150-300</td>
        <td>$7,500-15,000</td>
      </tr>
      <tr>
        <td>AWS WorkSpaces</td>
        <td>$420-900</td>
        <td>$21,000-45,000</td>
      </tr>
      <tr class="lamco-row">
        <td><strong>lamco-VDI (estimated)</strong></td>
        <td><strong>$60-120</strong></td>
        <td><strong>$3,000-6,000</strong></td>
      </tr>
    </tbody>
  </table>

  <p class="savings-highlight"><strong>Potential Savings:</strong> 70-85% reduction in total cost of ownership vs commercial VDI solutions.</p>

  <h4>Interest in lamco-VDI?</h4>

  <p>We're gathering feedback from potential enterprise users to guide lamco-VDI development priorities. If you're interested in headless multi-user VDI, please contact us at <a href="mailto:office@lamco.io">office@lamco.io</a> with:</p>

  <ul>
    <li>Your organization size and VDI requirements</li>
    <li>Current VDI solution (if any) and pain points</li>
    <li>Desired features and timeline</li>
    <li>Interest in alpha/beta testing</li>
  </ul>

  <p><em>Early feedback helps shape the product roadmap and ensures lamco-VDI meets real-world enterprise needs.</em></p>
</div>
```

---

## Update 6: Homepage Banner Update

### Location: Homepage → Product Announcement Banner

### Current Text:
```
NOW AVAILABLE: Lamco RDP Server v0.9.0 - Wayland-native RDP server for Linux
```

### Updated Text:
```
NOW AVAILABLE: Lamco RDP Server v0.9.0 - Remote access to your Linux desktop | IN DEVELOPMENT: lamco-VDI - Enterprise headless multi-user VDI platform
```

---

## Update 7: Products Page Update

### Location: Products Page → lamco-rdp-server Entry

### Current Text:
```
Lamco RDP Server (v0.9.0)
- "Wayland-native RDP server. Hardware acceleration, AVC444 encoding, runtime service discovery"
- Description: "Production ready" remote desktop for Linux
```

### Updated Text:
```
Lamco RDP Server (v0.9.0)
- "Remote access to your existing Linux desktop. Share your GNOME, KDE, or wlroots session via RDP."
- Description: Production-ready desktop sharing for remote work, system administration, and screen sharing
- Use Case: Single-user desktop access (workstations, VMs with desktop environment)
- Status: ✅ Available now (Flatpak, native packages)
```

---

### Location: Products Page → Add New Entry for lamco-VDI

### New Product Entry:

```html
<div class="product-card future-product">
  <h3>lamco-VDI (In Development)</h3>
  <p class="tagline">Enterprise headless multi-user VDI platform</p>

  <p class="description">True headless VDI server with embedded Smithay compositor. Multi-user concurrent sessions, no desktop environment required. Cloud-native deployment for enterprise virtual desktop infrastructure.</p>

  <ul class="key-features">
    <li>Headless operation (no DE required)</li>
    <li>Multi-user concurrent (10-50+ per server)</li>
    <li>Smithay-based custom compositor</li>
    <li>Zero-dialog session persistence</li>
    <li>Container/Kubernetes ready</li>
  </ul>

  <p class="use-case"><strong>Use Case:</strong> Enterprise VDI, cloud workspaces, thin client infrastructure</p>

  <p class="status"><strong>Status:</strong> 🚧 Active development (Smithay integration in progress)</p>

  <p class="timeline"><strong>Timeline:</strong> Alpha 6-9 months, Beta 9-12 months, v1.0 12-15 months</p>

  <a href="/products/lamco-vdi/" class="learn-more">Learn More →</a>
  <a href="mailto:office@lamco.io?subject=lamco-VDI Interest" class="contact-link">Express Interest →</a>
</div>
```

---

## Update 8: FAQ Additions

### Location: FAQ Page (or create if doesn't exist)

### New FAQ Entries:

**Q: Can lamco-rdp-server run on a headless server?**

A: lamco-rdp-server requires a desktop environment (GNOME, KDE, or wlroots compositor) to be running. For headless servers without a desktop environment, we're developing **lamco-VDI**, a separate product with embedded Smithay compositor designed specifically for headless multi-user VDI deployments.

If you have a server with GNOME/KDE installed (even in a headless VM), lamco-rdp-server will work — you just need a user session running (can use auto-login or VNC for initial setup).

For true headless operation with no desktop environment, wait for **lamco-VDI** (estimated 6-9 months to alpha).

---

**Q: Can multiple users connect to lamco-rdp-server simultaneously?**

A: No. lamco-rdp-server shares a **single user's desktop session**. When one user connects via RDP, they see and control that user's desktop. Multiple clients connecting simultaneously would all see the same desktop and fight for control (not a multi-user scenario).

For multi-user concurrent sessions where each user gets their own isolated Linux desktop, use **lamco-VDI** (in development) which supports 10-50+ concurrent users per server with per-user resource isolation.

---

**Q: What's the difference between lamco-rdp-server and lamco-VDI?**

A:

**lamco-rdp-server:** Shares your existing Linux desktop session. Requires desktop environment (GNOME/KDE/wlroots). Single user per machine. Available now.

**lamco-VDI:** Headless multi-user VDI server. No desktop environment required. Creates isolated sessions for multiple concurrent users. Built on Smithay compositor. In development (6-9 months to alpha).

Think of it as:
- **lamco-rdp-server** = "Remote Desktop Connection to your Linux PC"
- **lamco-VDI** = "Linux version of Citrix/VMware Horizon VDI platform"

See [Desktop Sharing vs Headless VDI comparison](/products/lamco-rdp-server/#desktop-vs-vdi) for detailed technical breakdown.

---

**Q: Can I use lamco-rdp-server for enterprise VDI?**

A: lamco-rdp-server is designed for **single-user desktop sharing**, not enterprise multi-user VDI.

**What it CAN do:**
- Remote access to individual employee workstations
- IT support accessing servers with desktop environments
- Development VMs with GNOME/KDE installed

**What it CANNOT do:**
- Host 100+ concurrent users on one server
- Run without desktop environment (headless)
- Create/destroy user sessions programmatically
- Per-user resource isolation (cgroup limits)

For enterprise VDI use cases, **lamco-VDI** (in development) is the appropriate product.

---

## Update 9: About/Vision Page Addition

### Location: About Page → Product Vision Section

### Add After Company Overview:

```html
<div class="product-vision">
  <h3>Our Remote Desktop Vision</h3>

  <p>We're building a comprehensive Linux remote desktop solution across two complementary products:</p>

  <h4>1. lamco-rdp-server: Desktop Sharing Made Simple</h4>
  <p>Remote access to your existing Linux desktop without complexity. Install, run, connect. Perfect for individual users, remote workers, and small teams. Available now.</p>

  <h4>2. lamco-VDI: Enterprise Headless VDI Platform</h4>
  <p>True multi-user virtual desktop infrastructure with no desktop environment overhead. Built for cloud deployments, enterprises, and service providers. Smithay-based custom compositor provides headless operation with 70-85% cost savings vs Citrix/VMware. In active development.</p>

  <h4>Why Two Products?</h4>

  <p>Different use cases require different architectures. Desktop sharing needs minimal setup and works with your existing environment. Enterprise VDI needs multi-user isolation, headless operation, and cloud-native scaling. Rather than compromise with a one-size-fits-all solution, we're building specialized products that excel at their specific purposes while sharing a common RDP foundation.</p>

  <p><strong>Shared Technology:</strong> 70% of the codebase is shared between products (RDP protocol, H.264 encoding, clipboard, input, Service Registry). This ensures reliability, reduces development cost, and benefits both user bases.</p>
</div>
```

---

## Update 10: Download Page - Product Recommendation

### Location: Download Page → Before Installation Options

### Add Product Selection Guide:

```html
<div class="product-selector">
  <h3>Which Product Do You Need?</h3>

  <div class="selector-grid">
    <div class="option recommended">
      <h4>I want to access my existing Linux desktop remotely</h4>
      <p>✅ <strong>You need lamco-rdp-server</strong></p>
      <p>You have a Linux workstation or VM with GNOME/KDE/wlroots installed and want to access it remotely via RDP.</p>
      <a href="#installation-options" class="btn-primary">Install lamco-rdp-server →</a>
    </div>

    <div class="option future">
      <h4>I need headless multi-user VDI for my organization</h4>
      <p>🚧 <strong>You need lamco-VDI (in development)</strong></p>
      <p>You want to deploy Linux virtual desktops for multiple concurrent users on headless cloud servers without installing desktop environments.</p>
      <a href="/products/lamco-vdi/" class="btn-secondary">Learn about lamco-VDI →</a>
      <a href="mailto:office@lamco.io?subject=lamco-VDI Interest" class="btn-secondary">Express Interest →</a>
    </div>
  </div>

  <p class="clarification"><strong>Not sure?</strong> If you have a Linux machine with GNOME or KDE already running and want to access it remotely, use lamco-rdp-server (available now). If you need to deploy 10+ concurrent user sessions on headless cloud servers, you're looking for lamco-VDI (in development).</p>
</div>
```

---

## Summary of Changes

### Content Additions

1. **Product Positioning Clarification** - Opening paragraph explains "existing desktop session" focus
2. **Desktop vs VDI Section** - Comprehensive comparison table and explanation
3. **Installation Methods Enhancement** - Best-for guidance and session persistence notes
4. **System Requirements Clarification** - Desktop environment requirement emphasized
5. **Roadmap Addition** - lamco-VDI section with technical details and timeline
6. **FAQ Entries** - Headless, multi-user, and VDI differentiation
7. **About Page Vision** - Product strategy explanation
8. **Download Page Selector** - Help users choose correct product
9. **Products Page Entry** - lamco-VDI listing
10. **Homepage Banner** - Mention both products

### Key Messaging

**For lamco-rdp-server:**
- "Remote access to **your existing Linux desktop**"
- "Share your GNOME, KDE, or wlroots session"
- "Perfect for workstations, remote work, screen sharing"
- "Requires desktop environment running"

**For lamco-VDI:**
- "Enterprise **headless multi-user VDI platform**"
- "No desktop environment required"
- "10-50+ concurrent users per server"
- "Cloud-native, Kubernetes-ready"
- "In development (Smithay integration active)"

### Technical Accuracy

All claims verified against:
- HEADLESS-DEPLOYMENT-ROADMAP.md (Smithay architecture, multi-user design)
- SESSION-PERSISTENCE-ARCHITECTURE.md (portal strategies, deployment constraints)
- Current codebase architecture (Portal client, PipeWire consumer)

### Transparency

- Clearly states what lamco-rdp-server CAN'T do (multi-user, headless without DE)
- Explains why lamco-VDI is needed (different architecture for different use case)
- Honest about development status (in progress, timeline estimates)
- Links to future product page for interested enterprises

---

**Ready for website implementation. All content is copy-paste ready HTML/markdown.**
