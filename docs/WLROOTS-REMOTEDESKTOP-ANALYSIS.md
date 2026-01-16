# wlroots RemoteDesktop Portal Support: Deep Analysis

**Date:** January 2026
**Author:** Research compilation for lamco-rdp-server
**Purpose:** Evaluate feasibility and effort for wlroots compositor support

---

## Executive Summary

wlroots-based compositors (Sway, Hyprland, River, labwc, etc.) currently **do not support** the `org.freedesktop.portal.RemoteDesktop` interface required for input injection. This is a fundamental gap that affects all remote desktop solutions, not just lamco-rdp-server.

### Why PRs Are Stalled (The Real Reason)

**It's NOT technical blockers - it's review bandwidth and process issues:**

| PR | Age | Status | Actual Blocker |
|----|-----|--------|----------------|
| #263 | 3 years | Draft | Superseded by #325 |
| #325 | 1 year | Open | Git history cleanup requested by emersion |
| #359 | 1 month | Open | Build config issue (libei dependency), minor bugs |

**PR #325 is functionally complete.** The only feedback from emersion was about commit organization ("recipe style"). Contributors have restructured commits multiple times. No fundamental objections to the feature itself.

**PR #359 (InputCapture with libei)** is very recent and working - tested with deskflow across network.

### The Architectural Split

There are **two competing standards** for input emulation on Wayland:

| Approach | Used By | Pros | Cons |
|----------|---------|------|------|
| **libei/libeis** | GNOME, KDE, portals | Flatpak-safe, sandboxed, standardized | Compositor must implement libeis server |
| **virtual-keyboard/pointer** | wlroots compositors | Works today, simple | No portal support, no sandbox, GNOME won't add |

**Key insight:** "It's possible to implement libei on top of wlroots protocols but not the other way around." - This means a bridge layer is feasible.

### Three Paths Forward

1. **Push existing PRs to merge** - Help get #325/#359 merged (lowest effort, highest impact)
2. **Implement libei→wlr-protocols bridge** - Make libei work with wlroots without compositor changes
3. **Direct wlr-protocols in your app** - Bypass portals entirely for wlroots

---

## 1. The Ideal Wayland-Native Architecture (What Should Exist)

Before diving into the messy reality, let's establish what a **properly designed** Wayland remote desktop stack would look like:

### 1.1 The Wayland Protocol Namespaces

```
ext-*     = Cross-compositor standards (the goal)
wp-*      = Wayland-protocols staging (becoming standards)
zwp-*     = Wayland-protocols unstable (older standards)
zwlr-*    = wlroots-specific (not standard, but widely used)
```

### 1.2 What a Complete Wayland-Native Stack Would Look Like

```
┌─────────────────────────────────────────────────────────────────┐
│                    IDEAL ARCHITECTURE                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Screen Capture:    ext-image-copy-capture-v1     ← EXISTS ✓   │
│                     (standardized, cross-compositor)            │
│                                                                 │
│  Keyboard Input:    virtual-keyboard-unstable-v1  ← EXISTS ✓   │
│                     (zwp namespace = standard)                  │
│                                                                 │
│  Pointer Input:     ext-virtual-pointer-v1        ← MISSING ✗  │
│                     (DOES NOT EXIST)                            │
│                                                                 │
│  Touch Input:       ext-virtual-touch-v1          ← MISSING ✗  │
│                     (DOES NOT EXIST)                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.3 The Gap: Why libei Exists

**The Wayland protocol designers never standardized pointer/touch injection.**

This created a vacuum:
- **wlroots** filled it with `wlr-virtual-pointer-unstable-v1` (compositor-specific)
- **GNOME/freedesktop** filled it with `libei` (entirely separate IPC, not Wayland)
- **KDE** adopted libei for portal compatibility

**libei is NOT a Wayland protocol.** It's a separate Unix socket protocol that happens to be used alongside Wayland. This is an important architectural distinction:

```
┌────────────────────────────────────────────────────────────────┐
│  WAYLAND PROTOCOLS (through wl_display)                        │
│  ─────────────────────────────────────                         │
│  • Screen capture (ext-image-copy-capture-v1)                  │
│  • Keyboard injection (virtual-keyboard-unstable-v1)           │
│  • Pointer injection (wlr-virtual-pointer - NOT standard)      │
│                                                                │
│  Connected via: $WAYLAND_DISPLAY socket                        │
└────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────┐
│  LIBEI PROTOCOL (separate from Wayland)                        │
│  ─────────────────────────────────────                         │
│  • Keyboard injection                                          │
│  • Pointer injection                                           │
│  • Touch injection                                             │
│                                                                │
│  Connected via: separate Unix socket from compositor           │
│  Obtained via: Portal D-Bus call (ConnectToEIS)                │
└────────────────────────────────────────────────────────────────┘
```

### 1.4 The Architectural Decision Point

You have two philosophical approaches:

**Approach A: "Pure Wayland" (wlroots philosophy)**
```
Use Wayland protocols wherever they exist:
  • Screen capture: ext-image-copy-capture-v1 ✓
  • Keyboard: virtual-keyboard-unstable-v1 ✓
  • Pointer: wlr-virtual-pointer-unstable-v1 (non-standard but Wayland)

Pros: Everything goes through wl_display, single connection model
Cons: wlr-virtual-pointer only works on wlroots compositors
```

**Approach B: "Portal Standard" (GNOME/KDE philosophy)**
```
Use D-Bus portals + libei for input:
  • Screen capture: Portal ScreenCast (wraps compositor protocols)
  • Input: Portal RemoteDesktop + libei

Pros: Works on GNOME, KDE, any compositor implementing libeis
Cons: libei is not a Wayland protocol, requires compositor support
```

**The uncomfortable truth:** Neither approach is "correct" because the Wayland ecosystem failed to standardize input injection. Both are valid responses to that gap.

### 1.5 Where Your Smithay Work Fits

Your PR #1902 (`ext-image-copy-capture-v1`) is doing the RIGHT thing:
- It implements the **actual Wayland standard** for screen capture
- It replaces the compositor-specific `wlr-screencopy`
- This is the direction the ecosystem is moving

For input injection, there is no equivalent standard to implement. Your options are:
1. Use the wlroots-specific protocols (Path 3)
2. Use libei (Path 2) - not Wayland, but the portal standard
3. Wait for `ext-virtual-pointer-v1` to be created (could be years, may never happen)

---

## 2. Current Architecture (The Reality)

### 2.1 How GNOME/KDE Do It (Working)

```
Application → RemoteDesktop Portal → libei/libeis → Compositor → Input Events
                    ↓
            ConnectToEIS() method
```

GNOME 45+ and KDE Plasma 6.1+ have full libei integration. The compositor runs an EIS (Emulated Input Server) that the portal connects to.

### 2.2 How wlroots Currently Works (Not Working)

```
Application → RemoteDesktop Portal → ??? → (no implementation)
                    ↓
            "Portal not found" error
```

xdg-desktop-portal-wlr only implements:
- ✅ `org.freedesktop.portal.Screenshot`
- ✅ `org.freedesktop.portal.ScreenCast`
- ❌ `org.freedesktop.portal.RemoteDesktop` (NOT IMPLEMENTED)
- ❌ `org.freedesktop.portal.InputCapture` (NOT IMPLEMENTED)

### 2.3 The Smithay Approach (Your PR #1902)

The PR you linked implements `ext-image-capture-source-v1` and `ext-image-copy-capture-v1` - the new standardized Wayland screen capture protocols. This is for **screen capture**, not input injection.

For RemoteDesktop input injection, Smithay has a separate effort:
- **Smithay PR #1388** (Draft): Implements Ei protocol support using the `reis` library
- **Status**: Work in progress by @ids1024
- **Provides**: Basic input emulation for keyboard and pointer via RemoteDesktop portal

---

## 3. The wlroots Decision: Why No libei in Core

**Issue #2378** (swaywm/wlroots) - **Closed as Won't Fix**

wlroots maintainer @emersion explicitly stated:

> "It doesn't make sense for a Wayland library like wlroots to add support for libeis anyways. Compositors can do it on their own."

**Rationale:**
- libei is display-manager-agnostic, not Wayland-specific
- Compositors have different security policies and seat handling
- Each compositor should decide how to map libeis events

**Implication:** Every wlroots compositor (Sway, Hyprland, River, etc.) must independently implement libeis support. There's no "one fix" for the ecosystem.

---

## 4. Existing Work and Active Development

### 4.1 xdg-desktop-portal-wlr RemoteDesktop Efforts

| PR/Issue | Status | What It Does |
|----------|--------|--------------|
| [#2](https://github.com/emersion/xdg-desktop-portal-wlr/issues/2) | Open (2018!) | Original feature request |
| [#263](https://github.com/emersion/xdg-desktop-portal-wlr/pull/263) | Draft | Basic RemoteDesktop using virtual-keyboard/pointer |
| [#323](https://github.com/emersion/xdg-desktop-portal-wlr/issues/323) | Open | Request for libei support |
| [#325](https://github.com/emersion/xdg-desktop-portal-wlr/pull/325) | Draft | Newer RemoteDesktop attempt |

**PR #263 Technical Issues:**
- Keyboard mapping problems (X sends wrong key)
- Special characters don't work across compositors
- Segfaults on portal restart
- No libei support (uses legacy virtual-keyboard/pointer)

### 4.2 Hyprland-Specific Work

| Resource | Status | Notes |
|----------|--------|-------|
| [XDPH #252](https://github.com/hyprwm/xdg-desktop-portal-hyprland/issues/252) | Open | Feature request, 66+ upvotes |
| [xdg-desktop-portal-hypr-remote](https://github.com/gac3k/xdg-desktop-portal-hypr-remote) | Experimental | Third-party implementation with libei |
| [XDPH #268](https://github.com/hyprwm/xdg-desktop-portal-hyprland/pull/268) | Draft | Official attempt, stalled |

**Third-Party Solution (gac3k):**
- Uses libei + wlr virtual-keyboard/pointer protocols
- Proof of concept, works with Deskflow
- Not production-ready, needs refinement

### 4.3 Smithay/COSMIC Work

| Resource | Status | Notes |
|----------|--------|-------|
| [Smithay #1388](https://github.com/Smithay/smithay/issues/1388) | Draft PR | Ei protocol support via `reis` library |
| [cosmic-comp #450](https://github.com/pop-os/cosmic-comp/issues/450) | Closed | Waynergy works as workaround |

**Smithay #1388 provides:**
- Basic input emulation for keyboard and pointer
- RemoteDesktop portal integration
- InputCapture portal support
- Working `type-text` example in `reis`

**Still needed:**
- Input capture for receiver contexts
- Better high-level server-side APIs in `reis`

---

## 5. The Three Paths Forward (DETAILED)

### How Each Path Relates to the Wayland-Native Ideal

| Path | Philosophy | Standards Used | Non-Standard Parts |
|------|------------|----------------|-------------------|
| **Path 1** (Push PRs) | Portal + wlr protocols | virtual-keyboard-unstable-v1 | wlr-virtual-pointer |
| **Path 2** (libei bridge) | Portal + libei | Portal D-Bus interface | libei (not Wayland) |
| **Path 3** (Direct protocols) | Pure Wayland | virtual-keyboard-unstable-v1 | wlr-virtual-pointer |

**Key insight:** ALL paths require non-standard components for pointer injection because no standard exists.

- **Paths 1 & 3** use `wlr-virtual-pointer` (Wayland protocol, but compositor-specific)
- **Path 2** uses `libei` (not a Wayland protocol, but the freedesktop/portal standard)

If you value "Wayland-native" over "portal-standard", Paths 1/3 are more aligned. If you value "works everywhere portals work", Path 2 is the answer.

---

### PATH 1: Push Existing PRs to Merge

This path involves helping get the existing xdg-desktop-portal-wlr PRs merged upstream.

#### PR #325: RemoteDesktop Protocol Implementation

**What it implements:**
- `org.freedesktop.portal.RemoteDesktop` D-Bus interface
- Pointer input via `wlr-virtual-pointer-unstable-v1`
- Keyboard input (adapted from Wayvnc codebase)
- ScreenCast integration through RemoteDesktop (enables Zoom screen sharing)
- Device selection (pointer/keyboard/touch) via D-Bus configuration

**Current status:** Open, functionally complete

**Exact blockers:**
1. **Git history cleanup** - emersion requested "recipe style" commits:
   - Each commit should be a logical, self-contained unit
   - Don't introduce code that's later removed/modified
   - Preserve original authorship with `Co-authored-by` trailers
   - The code works, it's just the commit presentation

2. **wlroots MR #4980** - Needed to prevent compositor crashes during mouse scrolling

**Contributors:** David96 (primary), Andrea Feletto (original protocol work)

**What "help" means:**
- Fork and rebase with clean commit history
- Address the scroll crash issue (may need wlroots MR #4980 merged first)
- Test extensively on Sway, River, labwc
- Engage emersion directly on the PR

**Technical dependencies:**
- xkbcommon (keyboard handling)
- wlr-virtual-pointer-unstable-v1 (all wlroots compositors support this)
- virtual-keyboard-unstable-v1 (all wlroots compositors support this)

#### PR #359: InputCapture Protocol Implementation

**What it implements:**
- `org.freedesktop.portal.InputCapture` D-Bus interface
- libei integration for event streaming
- Invisible layer-shell surface for event capture
- Pointer constraints (cursor lock)
- Keyboard inhibitors (prevent system shortcuts from disrupting)

**Current status:** Open, working (tested with deskflow across network)

**Exact blockers:**
1. **libei dependency meson config** - marked as `required: false` but actually required
2. **deskflow-specific bugs** - client enters loop, server mode has issues

**What "help" means:**
- Fix meson.build to require libei properly
- Debug the deskflow issues (may be deskflow bugs, not portal bugs)
- Test with other libei clients

#### Your Options for Path 1

| Action | Effort | Impact |
|--------|--------|--------|
| Review/test PRs, provide feedback | 1-2 days | Low - PRs already tested |
| Fork and clean up git history | 3-5 days | Medium - may unblock merge |
| Fix libei build config in #359 | 1 day | Low-Medium |
| Engage maintainers directly | Ongoing | High if successful |

**Recommendation:** This path has lowest effort IF maintainers are responsive. The PRs are done; it's process/review bandwidth blocking them.

---

### PATH 2: libei→wlr-protocols Bridge (Standalone Daemon)

This is the **most architecturally interesting** option. Create a standalone bridge daemon that:
1. Implements a libeis server (accepts EI protocol connections)
2. Translates EI events to wlr virtual-keyboard/pointer protocols
3. Works with ANY wlroots compositor without compositor modifications

**Proof of concept exists:** [xdg-desktop-portal-hypr-remote](https://github.com/gac3k/xdg-desktop-portal-hypr-remote)

#### Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Application Layer                            │
│  (lamco-rdp-server, RustDesk, Remmina, GNOME Connections, etc.)    │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ D-Bus
┌─────────────────────────────────────────────────────────────────────┐
│                    RemoteDesktop Portal                             │
│           (org.freedesktop.portal.RemoteDesktop)                    │
│                              │                                       │
│                    ConnectToEIS() ──► returns socket FD             │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Unix Socket (EI Protocol)
┌─────────────────────────────────────────────────────────────────────┐
│                     BRIDGE DAEMON (NEW)                             │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              libeis Server Implementation                    │   │
│  │  - Accepts EI connections from portal                        │   │
│  │  - Handles capability negotiation                            │   │
│  │  - Receives: keyboard, pointer, touch, scroll events         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              ▼ Event Translation                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Wayland Protocol Clients                        │   │
│  │  - zwp_virtual_keyboard_v1 (keymap, key, modifiers)         │   │
│  │  - zwlr_virtual_pointer_v1 (motion, button, axis, frame)    │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Wayland Socket
┌─────────────────────────────────────────────────────────────────────┐
│                    wlroots Compositor                               │
│              (Sway, Hyprland, River, labwc, etc.)                  │
└─────────────────────────────────────────────────────────────────────┘
```

#### libei/libeis Protocol Details

**IPC Mechanism:** Unix domain sockets

**Protocol flow:**
1. **Handshake**: Client and server exchange version info and capabilities
2. **Connection**: Server creates `ei_connection` object
3. **Seat Discovery**: Server advertises available seats (keyboard, pointer, touch)
4. **Capability Binding**: Client binds to desired input capabilities
5. **Event Streaming**: Client sends input events, server routes to input stack

**Supported capabilities:**
- `EI_DEVICE_CAP_POINTER` - relative motion
- `EI_DEVICE_CAP_POINTER_ABSOLUTE` - absolute positioning
- `EI_DEVICE_CAP_KEYBOARD` - key events
- `EI_DEVICE_CAP_TOUCH` - touchscreen
- `EI_DEVICE_CAP_SCROLL` - scroll wheel
- `EI_DEVICE_CAP_BUTTON` - mouse buttons

**Event types:**
- Motion (relative/absolute)
- Button press/release
- Key press/release
- Scroll (delta, discrete, stop, cancel)
- Touch (down, up, motion)
- Frame (event grouping)
- Ping/pong (keepalive)

#### Rust Implementation Options

**Option A: Use `reis` crate**
- Pure Rust libei/libeis implementation
- Used by Smithay #1388
- Status: "incomplete and subject to change" but functional
- Has working `type-text` example
- URL: https://crates.io/crates/reis

**Option B: FFI bindings to libei/libeis C libraries**
- More complete, battle-tested
- Used by GNOME, KDE, mutter
- Higher maintenance burden for Rust project

**Recommendation:** Use `reis` - you're already in the Smithay ecosystem, and contributing improvements upstream benefits COSMIC too.

#### wlr Protocol Translation

**virtual-keyboard-unstable-v1:**
```
EI keyboard events  →  zwp_virtual_keyboard_v1
─────────────────────────────────────────────
ei_key_press(key)   →  vk.keymap(fd, format, size)  // one-time setup
                    →  vk.key(time, key, state=pressed)
ei_key_release(key) →  vk.key(time, key, state=released)
ei_modifiers(...)   →  vk.modifiers(depressed, latched, locked, group)
```

**wlr-virtual-pointer-unstable-v1:**
```
EI pointer events    →  zwlr_virtual_pointer_v1
──────────────────────────────────────────────
ei_motion(dx, dy)    →  vp.motion(time, dx, dy)
ei_motion_abs(x, y)  →  vp.motion_absolute(time, x, y, x_extent, y_extent)
ei_button(btn, st)   →  vp.button(time, button, state)
ei_scroll(dx, dy)    →  vp.axis(time, axis, value)
                     →  vp.axis_source(source)
                     →  vp.axis_discrete(axis, steps)
ei_frame()           →  vp.frame()
```

#### Implementation Effort

| Component | Effort | Notes |
|-----------|--------|-------|
| D-Bus portal service skeleton | 2-3 days | Standard portal boilerplate |
| libeis server (using reis) | 1-2 weeks | Core protocol handling |
| virtual-keyboard client | 3-5 days | Keymap handling is tricky |
| virtual-pointer client | 2-3 days | Straightforward |
| Event translation layer | 3-5 days | Timing, batching, edge cases |
| Testing & debugging | 1-2 weeks | Multi-compositor testing |
| **Total** | **4-6 weeks** | |

#### Advantages of This Path

1. **Works with ALL wlroots compositors** - No compositor modifications needed
2. **Standard interface** - Apps use RemoteDesktop portal normally
3. **You control the implementation** - No waiting on upstream maintainers
4. **Reusable by others** - Could be upstream to xdg-desktop-portal-wlr
5. **Flatpak compatible** - Portal provides security boundary

#### Disadvantages

1. **Significant development effort** - More than direct protocols
2. **Another daemon to maintain** - Though could be part of portal
3. **reis crate is incomplete** - May need to contribute fixes upstream

---

### PATH 3: Direct wlr-protocols in lamco-rdp-server

Bypass the portal system entirely and inject input directly using Wayland protocols.

#### Protocol Details

**zwp_virtual_keyboard_manager_v1 / zwp_virtual_keyboard_v1:**

```c
// Manager interface - get from registry
zwp_virtual_keyboard_manager_v1
  └── create_virtual_keyboard(seat) → zwp_virtual_keyboard_v1

// Virtual keyboard interface
zwp_virtual_keyboard_v1
  ├── keymap(fd, format, size)     // Provide XKB keymap via mmap'd fd
  ├── key(time, key, state)        // Inject key event (press/release)
  ├── modifiers(depressed, latched, locked, group)  // Update modifier state
  └── destroy()
```

**zwlr_virtual_pointer_manager_v1 / zwlr_virtual_pointer_v1:**

```c
// Manager interface - get from registry
zwlr_virtual_pointer_manager_v1
  ├── create_virtual_pointer(seat) → zwlr_virtual_pointer_v1
  └── create_virtual_pointer_with_output(seat, output) → zwlr_virtual_pointer_v1

// Virtual pointer interface
zwlr_virtual_pointer_v1
  ├── motion(time, dx, dy)                              // Relative motion
  ├── motion_absolute(time, x, y, x_extent, y_extent)   // Absolute position
  ├── button(time, button, state)                       // Mouse button
  ├── axis(time, axis, value)                           // Scroll continuous
  ├── axis_source(source)                               // wheel/finger/etc
  ├── axis_stop(time, axis)                             // Stop scrolling
  ├── axis_discrete(axis, steps)                        // Discrete scroll
  ├── frame()                                           // Bundle events
  └── destroy()
```

#### Existing Implementations to Study

| Project | Language | What to learn |
|---------|----------|---------------|
| [wlrctl](https://git.sr.ht/~brocellous/wlrctl) | C | Clean protocol usage, keyboard handling |
| [lan-mouse](https://github.com/feschber/lan-mouse) | Rust | Rust bindings, multi-compositor |
| [ydotool](https://github.com/ReimuNotMoe/ydotool) | C++ | uinput fallback approach |
| [wayvnc](https://github.com/any1/wayvnc) | C | VNC→Wayland translation |
| [waynergy](https://github.com/r-c-f/waynergy) | C | Synergy client approach |

#### Rust Implementation

**Using wayland-client crate:**

```rust
use wayland_client::{Connection, QueueHandle};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

// Connect to compositor
let conn = Connection::connect_to_env()?;
let display = conn.display();

// Get registry, bind to managers
let vk_manager: ZwpVirtualKeyboardManagerV1 = ...;
let vp_manager: ZwlrVirtualPointerManagerV1 = ...;

// Create virtual devices
let keyboard = vk_manager.create_virtual_keyboard(&seat);
let pointer = vp_manager.create_virtual_pointer(Some(&seat));

// Inject events
pointer.motion(timestamp, dx, dy);
pointer.button(timestamp, BTN_LEFT, ButtonState::Pressed);
pointer.frame();

keyboard.key(timestamp, keycode, KeyState::Pressed);
```

**Keymap handling (the hard part):**

```rust
// Must provide XKB keymap to compositor before sending keys
let keymap_string = xkb_keymap_get_as_string(...);
let fd = create_shm_fd(keymap_string.len())?;
write_to_fd(fd, keymap_string)?;
keyboard.keymap(KeymapFormat::XkbV1, fd, keymap_string.len());
```

#### Implementation Effort

| Component | Effort | Notes |
|-----------|--------|-------|
| Wayland connection setup | 1-2 days | Use wayland-client crate |
| Protocol bindings | 1-2 days | wayland-protocols-wlr crate |
| Virtual pointer impl | 2-3 days | Straightforward |
| Virtual keyboard impl | 1 week | Keymap handling is complex |
| RDP→Wayland translation | 3-5 days | Scancode mapping |
| Testing on compositors | 3-5 days | Sway, Hyprland, River, labwc |
| **Total** | **2-4 weeks** | |

#### Advantages

1. **Works TODAY** - All wlroots compositors already support these protocols
2. **Simplest implementation** - No portal, no libei, direct injection
3. **Battle-tested** - wlrctl, lan-mouse, wayvnc all use this
4. **You control it entirely** - No dependencies on others

#### Disadvantages

1. **No Flatpak support** - Requires Wayland socket access
2. **No security boundary** - Any app with socket access can inject
3. **GNOME/KDE incompatible** - These protocols don't exist there
4. **Two code paths** - Portal for GNOME/KDE, direct for wlroots

#### Security Considerations

```
┌──────────────────────────────────────────────────────────────┐
│  PORTAL APPROACH (Path 2)                                    │
│  ─────────────────────────                                   │
│  App → Portal → Permission Dialog → EIS → Compositor         │
│                      ▲                                       │
│                      │                                       │
│              User grants access                              │
│              (one-time or session)                           │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│  DIRECT APPROACH (Path 3)                                    │
│  ─────────────────────────                                   │
│  App → Wayland Socket → virtual-keyboard/pointer             │
│              ▲                                               │
│              │                                               │
│       Socket access = full input access                      │
│       (no permission dialog)                                 │
└──────────────────────────────────────────────────────────────┘
```

For lamco-rdp-server specifically:
- Users are **explicitly running an RDP server**
- They've already granted network access
- The security context is different from sandboxed apps
- **Direct approach is acceptable** with proper documentation

---

## 6. Compositor-by-Compositor Analysis

### 6.1 Sway

| Aspect | Status |
|--------|--------|
| Portal | xdg-desktop-portal-wlr |
| RemoteDesktop | ❌ Not implemented |
| libei in compositor | ❌ Not implemented |
| virtual-keyboard | ✅ Supported |
| virtual-pointer | ✅ Supported (wlr-virtual-pointer) |
| Active Development | PRs #263, #325 stalled |
| Maintainer Interest | Low - emersion focused elsewhere |

**Outlook:** Unlikely to get official RemoteDesktop portal support soon. Community PRs exist but haven't been merged in years.

### 6.2 Hyprland

| Aspect | Status |
|--------|--------|
| Portal | xdg-desktop-portal-hyprland |
| RemoteDesktop | ❌ Not implemented |
| libei in compositor | ❌ Not implemented |
| virtual-keyboard | ✅ Supported |
| virtual-pointer | ✅ Supported |
| Active Development | PR #268 draft, third-party solution exists |
| Maintainer Interest | Medium - "not atm" but community pressure |

**Outlook:** Most likely wlroots compositor to get RemoteDesktop support due to active community. Third-party [xdg-desktop-portal-hypr-remote](https://github.com/gac3k/xdg-desktop-portal-hypr-remote) shows the path.

### 6.3 River

| Aspect | Status |
|--------|--------|
| Portal | xdg-desktop-portal-wlr |
| RemoteDesktop | ❌ Not implemented |
| Development | Minimal focus on this area |

**Outlook:** Will likely follow whatever xdg-desktop-portal-wlr does.

### 6.4 labwc

| Aspect | Status |
|--------|--------|
| Portal | xdg-desktop-portal-wlr |
| RemoteDesktop | ❌ Not implemented |
| Development | Focus on Openbox compatibility |

**Outlook:** Same as River - dependent on xdg-desktop-portal-wlr progress.

### 6.5 COSMIC (Smithay-based)

| Aspect | Status |
|--------|--------|
| Portal | cosmic-comp built-in |
| RemoteDesktop | 🟡 In development (Smithay #1388) |
| libei in compositor | 🟡 In development via `reis` |
| Active Development | Yes - @ids1024 working on it |

**Outlook:** COSMIC is likely to have proper RemoteDesktop portal support. Your best bet for a wlroots-alternative that works.

---

## 7. Effort Estimates

### For lamco-rdp-server to Support wlroots Compositors

| Approach | Development Time | Maintenance Burden | Coverage |
|----------|------------------|-------------------|----------|
| Wait for ecosystem | 0 | 0 | Unknown timeline |
| virtual-keyboard/pointer | 2-4 weeks | Medium | All wlroots |
| Hybrid (portal + fallback) | 3-5 weeks | High | All compositors |
| Contribute to xdg-desktop-portal-wlr | 4-8 weeks | Low after merge | All wlroots |

### For the Ecosystem to Fix This

| Component | Who | Effort | Likelihood |
|-----------|-----|--------|------------|
| xdg-desktop-portal-wlr RemoteDesktop | Community | 4-8 weeks | Low (stalled 5+ years) |
| Sway libeis | Sway team | 6-12 weeks | Very Low |
| Hyprland libeis | Hyprland team | 4-8 weeks | Medium |
| Smithay/COSMIC completion | System76/ids1024 | 2-4 weeks | High |

---

## 8. Recommendations (Decision Framework)

### If You Want Maximum Impact with Minimum Effort

**→ Path 1: Push PR #325 to merge**

The code is done. Fork it, clean up the git history to emersion's liking, and engage directly. This benefits the entire wlroots ecosystem.

**Concrete steps:**
1. Fork xdg-desktop-portal-wlr
2. Cherry-pick commits from PR #325
3. Reorganize into clean "recipe style" commits
4. Open new PR with clean history
5. Ping emersion for review

**Risk:** Maintainer may still not prioritize review. Timeline uncertain.

### If You Want Full Control and Faster Results

**→ Path 3: Direct wlr-protocols in lamco-rdp-server**

Implement `virtual-keyboard-unstable-v1` and `wlr-virtual-pointer-unstable-v1` directly in lamco-rdp-server. This is what lan-mouse and wayvnc do.

**Concrete steps:**
1. Add wayland-client and wayland-protocols-wlr dependencies
2. Study lan-mouse's Rust implementation (closest to what you need)
3. Implement pointer injection (straightforward)
4. Implement keyboard injection (trickier - keymap handling)
5. Add detection logic: use portal if available, wlr-protocols if not

**Risk:** Two code paths to maintain. No Flatpak support on wlroots.

### If You Want the Most Correct/Reusable Solution

**→ Path 2: Build libei→wlr bridge daemon**

This creates a proper solution that works for everyone, not just lamco-rdp-server.

**Concrete steps:**
1. Fork xdg-desktop-portal-wlr or start fresh
2. Implement RemoteDesktop portal interface using D-Bus
3. Implement libeis server using `reis` crate
4. Translate EI events to wlr virtual-keyboard/pointer
5. Package and offer upstream

**Risk:** Significant effort. May duplicate work if PR #325 merges.

### My Recommendation for lamco-rdp-server

**Start with Path 3 (direct protocols), with Path 1 as parallel effort:**

```
Phase 1 (Now - 2-4 weeks):
├── Implement direct wlr-protocol input backend
├── Add config option: input_backend = "auto" | "portal" | "wlr-protocols"
├── Document security implications
└── Test on Sway, Hyprland, River, labwc

Phase 2 (Parallel):
├── Fork xdg-desktop-portal-wlr
├── Clean up PR #325
└── Submit for review

Phase 3 (If Phase 2 stalls):
├── Convert PR #325 fork to standalone daemon
├── Add libei bridge layer
└── Offer as alternative to xdg-desktop-portal-wlr
```

**Why this approach:**
- Path 3 gives you working wlroots support NOW
- Path 1 (parallel) may get merged, making Path 2 unnecessary
- If Path 1 fails, you have the foundation for Path 2
- No wasted effort - each phase builds on the previous

---

## 9. Protocol Landscape: Standards Summary

Understanding this is critical for making an informed decision.

### Screen Capture Protocols

| Protocol | Status | Supported By |
|----------|--------|--------------|
| `wlr-screencopy-unstable-v1` | wlroots-specific, deprecated | Sway, Hyprland, River, etc. |
| `ext-image-copy-capture-v1` | **Wayland standard** | GNOME 47+, KDE 6.2+, wlroots 0.18+ |
| `org.freedesktop.portal.ScreenCast` | Portal (D-Bus) | All major DEs |

**Your Smithay PR #1902** implements the new `ext-image-copy-capture-v1` standard - this is the correct direction.

### Input Injection Protocols

| Protocol | Status | Supported By |
|----------|--------|--------------|
| `virtual-keyboard-unstable-v1` | **Wayland standard** (zwp namespace) | wlroots, KWin, COSMIC |
| `wlr-virtual-pointer-unstable-v1` | wlroots-specific (zwlr namespace) | wlroots only |
| `org.freedesktop.portal.RemoteDesktop` + libei | Portal (D-Bus) | GNOME, KDE |

**Key insight:** There is **NO standard Wayland protocol for pointer injection**. The options are:
- `wlr-virtual-pointer-unstable-v1` (wlroots only)
- libei/libeis (compositor-specific implementation required)
- compositor-specific D-Bus interfaces

### What This Means for Your Decision

If you want to use **only standardized Wayland protocols**:
- Screen capture: Use `ext-image-copy-capture-v1` ✓ (your Smithay PR)
- Keyboard: Use `virtual-keyboard-unstable-v1` ✓ (widely supported)
- Pointer: **No standard exists** - must use wlr-specific or libei

There is no "pure Wayland" solution for pointer injection. Every approach requires either:
- A compositor-specific protocol (wlr-virtual-pointer)
- A non-Wayland protocol (libei over Unix socket)
- A portal (D-Bus + libei)

**The architectural reality:**
```
Screen Capture:  wlr-screencopy → ext-image-copy-capture (standardized!)
Input Injection: wlr-virtual-pointer → ??? (no standard in sight)
                 libei → (works but not a Wayland protocol)
```

This is why the wlroots ecosystem has fragmented - they chose direct Wayland protocols where possible, but input injection has no standard, so they use their own protocols and reject libei in wlroots core.

---

## 10. Key References

### xdg-desktop-portal-wlr PRs (Study These)
- [PR #325](https://github.com/emersion/xdg-desktop-portal-wlr/pull/325) - RemoteDesktop implementation (functionally complete)
- [PR #359](https://github.com/emersion/xdg-desktop-portal-wlr/pull/359) - InputCapture with libei (working)
- [PR #263](https://github.com/emersion/xdg-desktop-portal-wlr/pull/263) - Original attempt (superseded)
- [Issue #2](https://github.com/emersion/xdg-desktop-portal-wlr/issues/2) - Original 2018 feature request

### wlroots/Sway
- [xdg-desktop-portal-wlr](https://github.com/emersion/xdg-desktop-portal-wlr)
- [wlroots libei rejection #2378](https://github.com/swaywm/wlroots/issues/2378) - Why libei won't be in wlroots core

### Hyprland
- [XDPH RemoteDesktop #252](https://github.com/hyprwm/xdg-desktop-portal-hyprland/issues/252)
- [xdg-desktop-portal-hypr-remote](https://github.com/gac3k/xdg-desktop-portal-hypr-remote) - **Working libei→wlr bridge PoC**

### Smithay/COSMIC
- [Smithay Ei support PR #1388](https://github.com/Smithay/smithay/pull/1388) - Using `reis` crate
- [Smithay screen capture PR #1902](https://github.com/Smithay/smithay/pull/1902) - Your ext-image-copy-capture work

### Rust Libraries
- [reis crate](https://crates.io/crates/reis) - Pure Rust libei/libeis implementation
- [wayland-protocols-wlr](https://crates.io/crates/wayland-protocols-wlr) - wlr protocol bindings
- [wayland-client](https://crates.io/crates/wayland-client) - Wayland client library

### Reference Implementations (Study for Path 3)
- [lan-mouse](https://github.com/feschber/lan-mouse) - **Rust, multi-compositor** (best reference)
- [wlrctl](https://git.sr.ht/~brocellous/wlrctl) - C, clean implementation
- [wayvnc](https://github.com/any1/wayvnc) - C, VNC→Wayland

### Protocols & Standards
- [RemoteDesktop Portal Spec](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html)
- [libei Documentation](https://libinput.pages.freedesktop.org/libei/)
- [virtual-keyboard-unstable-v1](https://wayland.app/protocols/virtual-keyboard-unstable-v1)
- [wlr-virtual-pointer-unstable-v1](https://wayland.app/protocols/wlr-virtual-pointer-unstable-v1)
- [ext-image-copy-capture-v1](https://wayland.app/protocols/ext-image-copy-capture-v1)

---

## 11. Conclusion

The wlroots ecosystem has a structural gap in RemoteDesktop support that has persisted for 5+ years. The maintainers have made a deliberate architectural decision to leave libei implementation to individual compositors, and no major wlroots compositor has completed this work.

**For lamco-rdp-server:**
- GNOME and KDE work today via standard portals
- COSMIC will likely work soon (Smithay #1388)
- wlroots compositors require either:
  - Waiting for ecosystem (uncertain timeline)
  - Implementing non-portal workarounds (security tradeoffs)
  - Contributing upstream (significant effort)

The most pragmatic approach is to **document wlroots as currently unsupported** while **monitoring Hyprland and COSMIC progress**, with an optional experimental virtual-keyboard/pointer backend for users who accept the tradeoffs.
