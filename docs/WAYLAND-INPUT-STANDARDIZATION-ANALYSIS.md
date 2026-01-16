# Wayland Input Injection Standardization: Deep Analysis

**Date:** January 2026
**Author:** Research compilation for lamco-rdp-server
**Purpose:** Understand whether pushing for a Wayland standard for input injection is realistic

---

## Executive Summary

**The uncomfortable truth:** There is NO Wayland standard for pointer injection, and this is unlikely to change soon.

### Why No Standard Exists

| Component | Standard Status | Reality |
|-----------|-----------------|---------|
| Screen capture | `ext-image-copy-capture-v1` ✓ | Standardized in 2024 |
| Keyboard injection | `virtual-keyboard-unstable-v1` ✓ | Standardized in wayland-protocols |
| **Pointer injection** | **NONE** | wlr-virtual-pointer (wlroots) OR libei (GNOME/KDE) |
| **Touch injection** | **NONE** | Same split |

### Why It Hasn't Been Fixed

1. **No one has proposed it** - No ext-virtual-pointer merge request exists
2. **Competing philosophies** - wlroots prefers Wayland protocols, GNOME prefers libei
3. **Both camps have working solutions** - No urgency to unify
4. **libei is deliberately NOT a Wayland protocol** - This was an explicit design decision

### Is Pushing for a Standard Realistic?

**Short answer:** Technically possible, politically difficult.

- **Technical requirements:** Achievable (implementations exist)
- **Political requirements:** Would need to navigate GNOME's preference for libei
- **Effort:** 6-12 months minimum including discussion period
- **Outcome:** Uncertain - could be NACKed or simply ignored

---

## 1. How Wayland Protocols Get Standardized

### 1.1 The wayland-protocols Governance Structure

[wayland-protocols](https://gitlab.freedesktop.org/wayland/wayland-protocols) is a standardization body, not just a repository.

**Current Members (as of January 2026):**

| Project | Points of Contact | Notes |
|---------|-------------------|-------|
| GTK/Mutter (GNOME) | Jonas Ådahl, Carlos Garnacho | Can veto wp/xdg protocols |
| KWin (KDE) | Vlad Zahorodnii, David Edmundson, Xaver Hugl | Can veto wp/xdg protocols |
| Mesa | Daniel Stone, Mike Blumenkrantz | Can veto wp/xdg protocols |
| Mir (Canonical) | Christopher James Halse Rogers, Alan Griffiths | Can veto wp/xdg protocols |
| Qt | Eskil Abrahamsen Blomfeldt | Can veto wp/xdg protocols |
| **Smithay/COSMIC** | Victoria Brekenfeld | Can veto wp/xdg protocols |
| Weston | Pekka Paalanen, Derek Foreman | Can veto wp/xdg protocols |
| **wlroots/Sway** | Simon Ser, Simon Zeni | Can veto wp/xdg protocols |
| Chromium | Fangzhou Ge, Nick Yamane, Max Ihlenfeldt | Can veto wp/xdg protocols |

**Key insight:** You (through Smithay) are represented on the governance body!

### 1.2 Protocol Namespaces and Requirements

| Namespace | Purpose | ACKs Required | Veto Power | Implementations |
|-----------|---------|---------------|------------|-----------------|
| `wp-` | Plumbing protocols | 3 members | ANY member can NACK | 3 (1+2 or 2+1) |
| `xdg-` | Window management | 3 members | ANY member can NACK | 3 (1+2 or 2+1) |
| `ext-` | Everything else | 1 member | No veto | 1 client + 1 server |

**For ext-virtual-pointer, you would need:**
- 1 member to sponsor (e.g., Smithay/COSMIC or wlroots)
- 1 other member to ACK
- 1 open-source client implementation (wlrctl, lan-mouse, etc.)
- 1 open-source server implementation (Sway, Hyprland, etc.)

**All of these requirements are already met!** The protocol just hasn't been proposed.

### 1.3 The Proposal Process

```
1. Submit merge request to gitlab.freedesktop.org/wayland/wayland-protocols
   └── Include protocol XML file
   └── Include justification for namespace placement

2. 30-day minimum discussion period
   └── Members can ACK, NACK, or comment
   └── Gather required implementations (already exist for virtual-pointer)

3. If requirements met, sponsoring member can merge
```

---

## 2. Why Input Injection Was Never Standardized

### 2.1 The History

**2017:** Peter Hutterer (Red Hat) proposed an [RFC for input injection](https://lists.freedesktop.org/archives/wayland-devel/2017-March/033518.html)
- Was described as "a thought experiment"
- Raised concerns about authentication and security
- Never progressed beyond discussion

**2018:** Purism proposed [virtual-keyboard-unstable-v1](https://lists.freedesktop.org/archives/wayland-devel/2018-August/039239.html)
- Motivated by Librem 5 phone on-screen keyboard
- Successfully merged into wayland-protocols
- **Note:** No corresponding virtual-pointer was proposed because OSK doesn't need it

**2019:** wlroots created `wlr-virtual-pointer-unstable-v1`
- Motivated by VNC/remote desktop use cases
- Kept in wlr-protocols, not proposed to wayland-protocols
- "We have a virtual keyboard protocol, but we're missing a virtual pointer one"

**2021-2023:** Peter Hutterer developed libei
- Explicitly designed as NOT a Wayland protocol
- Announced: "Emulated input is not specifically Wayland-y"
- Released libei 1.0.0 in June 2023

### 2.2 Peter Hutterer's Reasoning (libei Author)

From the [libei RFC discussion](https://www.mail-archive.com/wayland-devel@lists.freedesktop.org/msg40894.html):

> "The design starts from the baseline that there is no emulated input in Wayland (the protocol). Emulated input is not specifically Wayland-y - clients that emulate input generally don't care about Wayland itself."

> "The only connection to Wayland is merely that input events are received through the Wayland protocol. So a Wayland protocol for emulating input is not a great fit, it merely ticks the convenient box of 'we already have IPC through the Wayland protocol, why not just do it there'."

**The key benefit of libei being separate:**
- Can be gated behind portals (security/permission dialogs)
- Can negotiate different backends (portal → DBus → socket)
- Same API regardless of negotiation method

### 2.3 Why wlroots Hasn't Proposed Standardization

From [wlroots issue #2378](https://github.com/swaywm/wlroots/issues/2378):

> "It doesn't make sense for a Wayland library like wlroots to add support for libeis anyways. Compositors can do it on their own."

wlroots maintains [wlr-protocols](https://gitlab.freedesktop.org/wlroots/wlr-protocols) separately. The README states:

> "New protocols should not be submitted to wlr-protocols."

This suggests wlroots expects protocols to move to wayland-protocols, but they haven't pushed for it.

---

## 3. The Political Reality

### 3.1 Why Both Camps Are "Fine"

**GNOME/KDE Camp:**
- Have libei integrated into mutter and KWin
- Portal integration provides security boundary
- GNOME 45+, KDE Plasma 6.1+ ship with libei support
- No motivation to support wlr-virtual-pointer

**wlroots Camp:**
- Have wlr-virtual-pointer working across all wlroots compositors
- Tools like wlrctl, lan-mouse, wayvnc work today
- No motivation to adopt libei (rejected it)

**The result:** Neither camp is motivated to unify. Both have working solutions for their users.

### 3.2 What Would Happen If You Proposed ext-virtual-pointer

**Scenario A: Smooth Acceptance**
- Smithay/COSMIC sponsors (Victoria Brekenfeld)
- wlroots ACKs (Simon Ser)
- Implementations already exist
- Merged to wayland-protocols as ext-virtual-pointer-v1

**Scenario B: GNOME Objects**
- Jonas Ådahl or Carlos Garnacho (GTK/Mutter) argue:
  - "libei is the standard approach for input injection"
  - "Adding another protocol fragments the ecosystem further"
  - "Use the RemoteDesktop portal instead"
- For ext namespace, they cannot NACK (veto), but they can refuse to implement
- Result: Protocol exists but only wlroots compositors implement it

**Scenario C: Discussion Stalls**
- 30-day discussion period extends indefinitely
- No member wants to take sides
- Proposal dies from inaction

**Most likely outcome:** B or C. GNOME has invested heavily in libei and is unlikely to enthusiastically support a competing approach.

### 3.3 The Transient Seat Precedent

Interestingly, [ext-transient-seat-v1](https://wayland.app/protocols/ext-transient-seat-v1) was standardized and explicitly mentions virtual input protocols:

> "This protocol integrates with virtual input systems. It's designed specifically to work alongside virtual input extensions like virtual keyboard and virtual pointer protocols."

This suggests the wayland-protocols body expects virtual input protocols to exist. The standardization just hasn't happened.

---

## 4. What It Would Take to Standardize

### 4.1 The Technical Path (Achievable)

1. **Take wlr-virtual-pointer-unstable-v1 XML**
2. **Rename to ext-virtual-pointer-v1** (drop "unstable", use ext namespace)
3. **Submit merge request** to gitlab.freedesktop.org/wayland/wayland-protocols
4. **Justify inclusion:**
   - Complements existing virtual-keyboard-unstable-v1
   - Works with ext-transient-seat-v1 (already designed for this)
   - Implementations exist: Sway, Hyprland, River, labwc (servers); wlrctl, lan-mouse (clients)
5. **Get 1 ACK** from another member

**Estimated effort:** 2-4 weeks to prepare and submit

### 4.2 The Political Path (Uncertain)

1. **Build consensus BEFORE submitting:**
   - Discuss on wayland-devel mailing list
   - Get informal support from key stakeholders
   - Address GNOME's concerns proactively

2. **Frame it correctly:**
   - "Complementary to libei, not competing"
   - "For use cases where portal isn't appropriate"
   - "Already implemented in 10+ compositors"

3. **Find champions:**
   - Victoria Brekenfeld (Smithay/COSMIC) - likely supportive
   - Simon Ser (wlroots) - likely supportive
   - Someone from GNOME/KDE - critical but uncertain

**Estimated effort:** 3-6 months of discussion, relationship building

### 4.3 The Alternative: Do Nothing

Accept the status quo:
- wlroots compositors use wlr-virtual-pointer
- GNOME/KDE use libei
- Applications implement both (like lan-mouse does)
- Portals provide the "universal" API (when compositors implement RemoteDesktop)

**This is what everyone has been doing for 5+ years.**

---

## 5. Recommendations

### If You Want to Push for Standardization

**Phase 1: Gauge Interest (2-4 weeks)**
1. Email wayland-devel mailing list with proposal summary
2. CC: Victoria Brekenfeld, Simon Ser, Jonas Ådahl
3. Frame as "standardizing existing practice" not "competing with libei"
4. Explicitly address: "What about libei?"

**Phase 2: Build Consensus (1-3 months)**
1. Respond to feedback, adjust proposal
2. Get informal ACKs from 2+ members
3. Address any technical concerns

**Phase 3: Submit MR (2-4 weeks)**
1. Prepare protocol XML
2. Submit to wayland-protocols
3. Wait 30+ days
4. Hope for merge

**Success probability:** 30-50% (GNOME's position is the wild card)

### If You Want a Working Solution Now

**For lamco-rdp-server today:**
1. Use libei/portal where available (GNOME, KDE)
2. Use wlr-virtual-pointer where available (wlroots)
3. Document wlroots as "experimental" until ecosystem matures

**This is the pragmatic path.** Standardization can happen in parallel.

### If You Want to Help the Ecosystem

**Option A: Push xdg-desktop-portal-wlr PR #325**
- Already implements RemoteDesktop portal
- Would give wlroots compositors portal support
- Apps could use standard portal API everywhere

**Option B: Contribute to Smithay #1388**
- Adds libei support to Smithay
- Would enable COSMIC to support RemoteDesktop portal
- Aligns with your existing Smithay involvement

**Option C: Build libei→wlr bridge**
- Standalone daemon that implements libei using wlr-protocols
- Works with existing wlroots compositors
- Could become the reference implementation for wlroots

---

## 6. Key References

### Governance & Process
- [wayland-protocols GOVERNANCE.md](https://chromium.googlesource.com/external/anongit.freedesktop.org/git/wayland/wayland-protocols/+/HEAD/GOVERNANCE.md)
- [wayland-protocols MEMBERS.md](https://raw.githubusercontent.com/wayland-mirror/wayland-protocols/main/MEMBERS.md)
- [wayland-protocols GitLab](https://gitlab.freedesktop.org/wayland/wayland-protocols)

### Historical Discussions
- [2017 RFC: Interface for injection of input events](https://lists.freedesktop.org/archives/wayland-devel/2017-March/033518.html) - Peter Hutterer
- [2018 virtual-keyboard proposal](https://lists.freedesktop.org/archives/wayland-devel/2018-August/039239.html) - Purism/Dorota Czaplejewicz
- [libei RFC discussion](https://www.mail-archive.com/wayland-devel@lists.freedesktop.org/msg40894.html) - Why not Wayland protocol
- [libei 1.0.0 announcement](https://lists.freedesktop.org/archives/wayland-devel/2023-June/042731.html)

### Existing Protocols
- [virtual-keyboard-unstable-v1](https://wayland.app/protocols/virtual-keyboard-unstable-v1) - Standardized
- [wlr-virtual-pointer-unstable-v1](https://wayland.app/protocols/wlr-virtual-pointer-unstable-v1) - wlroots only
- [ext-transient-seat-v1](https://wayland.app/protocols/ext-transient-seat-v1) - Designed for virtual input
- [pointer-warp-v1](https://wayland.app/protocols/pointer-warp-v1) - Staging (2024)

### Related Issues
- [wlr-protocols issue #36: Virtual pointer protocol](https://github.com/swaywm/wlr-protocols/issues/36)
- [wlroots issue #2378: libei support](https://github.com/swaywm/wlroots/issues/2378) - Rejected
- [libei issue #2: Wayland virtual-{pointer,keyboard} backends](https://gitlab.freedesktop.org/libinput/libei/-/issues/2)

---

## 7. Conclusion

### The Reality

Wayland input injection standardization failed not due to technical barriers, but due to:
1. **Different philosophies** - Wayland protocol vs. separate IPC (libei)
2. **Lack of urgency** - Both camps have working solutions
3. **No champion** - No one has pushed for unification

### Your Position

As a Smithay contributor, you have:
- **Standing** - Smithay/COSMIC is a wayland-protocols member
- **Technical capability** - You could propose and implement
- **Motivation** - You need this for lamco-rdp-server

### The Question

Is standardization worth pursuing given:
- 6-12 months of effort
- Uncertain outcome (GNOME may object)
- Working alternatives exist (wlr-virtual-pointer + libei)

**My assessment:** The pragmatic approach is to implement both backends (libei + wlr-protocols) while optionally pursuing standardization. Don't let standardization efforts block shipping a working product.

**If you do pursue standardization:** Start with a wayland-devel email to gauge interest before investing significant effort. The response will tell you whether it's worth pursuing.
