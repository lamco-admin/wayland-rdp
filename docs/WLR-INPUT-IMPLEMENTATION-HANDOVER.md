# wlr-protocols Input Implementation: Research & Implementation Handover

**Date:** January 2026
**Purpose:** Self-contained research plan for implementing wlroots input injection in lamco-rdp-server
**Status:** Ready for new session to begin research and implementation

---

## 1. Executive Summary

### Goal
Add direct wlroots protocol support for input injection to lamco-rdp-server, enabling it to work on Sway, Hyprland, River, labwc, and other wlroots-based compositors WITHOUT requiring the RemoteDesktop portal.

### Why This Is Needed
- xdg-desktop-portal-wlr does NOT implement RemoteDesktop portal
- wlroots compositors have no portal-based input injection
- Direct protocol usage is the only working path for wlroots today

### Estimated Effort
2-3 weeks for a clean implementation

### Key Constraint: Licensing
- **lamco-rdp-server is BUSL-1.1** (Business Source License)
- **Cannot use GPL-3.0 dependencies** (like `input-emulation` from lan-mouse)
- Must implement from scratch, using GPL code only as reference (reading is fine)

---

## 2. Project Context

### lamco-rdp-server Architecture

lamco-rdp-server is a Wayland-native RDP server written in Rust. It currently supports:
- **Screen capture:** Via Portal ScreenCast or Mutter direct API
- **Input injection:** Via Portal RemoteDesktop API (libei under the hood)
- **Clipboard:** Via Portal or FUSE-based approach

The input system uses a clean abstraction layer:

```
src/session/strategy.rs        - SessionHandle trait definition
src/session/strategies/
├── portal_token.rs            - Portal-based implementation
├── mutter_direct.rs           - GNOME Mutter direct API
└── selector.rs                - Strategy selection logic
src/server/input_handler.rs    - IronRDP → SessionHandle bridge
src/input/                     - Keyboard/mouse translation, coordinates
```

### The SessionHandle Trait

```rust
// src/session/strategy.rs
#[async_trait]
pub trait SessionHandle: Send + Sync {
    // Video capture
    fn pipewire_access(&self) -> PipeWireAccess;
    fn streams(&self) -> Vec<StreamInfo>;
    fn session_type(&self) -> SessionType;

    // Input injection - THESE ARE WHAT WE NEED TO IMPLEMENT
    async fn notify_keyboard_keycode(&self, keycode: i32, pressed: bool) -> Result<()>;
    async fn notify_pointer_motion_absolute(&self, stream_id: u32, x: f64, y: f64) -> Result<()>;
    async fn notify_pointer_button(&self, button: i32, pressed: bool) -> Result<()>;
    async fn notify_pointer_axis(&self, dx: f64, dy: f64) -> Result<()>;

    // Clipboard
    fn portal_clipboard(&self) -> Option<ClipboardComponents>;
}
```

### What Already Exists

1. **Scancode translation:** `src/input/keyboard/` converts RDP scancodes to Linux evdev keycodes
2. **Coordinate transformation:** `src/input/coordinates/` handles multi-monitor coordinate mapping
3. **Mouse state tracking:** `src/input/mouse/` tracks button state, handles scroll
4. **Input batching:** `src/server/input_handler.rs` batches events (10ms windows)

### What Needs To Be Added

A new `WlrDirectStrategy` that:
1. Connects to Wayland compositor
2. Binds `zwp_virtual_keyboard_v1` and `zwlr_virtual_pointer_v1` protocols
3. Implements `SessionHandle` trait methods using these protocols
4. Handles XKB keymap generation and sharing

---

## 3. The Protocols

### 3.1 virtual-keyboard-unstable-v1 (zwp namespace - STANDARD)

**Protocol XML:** https://wayland.app/protocols/virtual-keyboard-unstable-v1

**Interfaces:**
- `zwp_virtual_keyboard_manager_v1` - Factory (get from registry)
- `zwp_virtual_keyboard_v1` - Virtual keyboard instance

**Key Methods:**
```
zwp_virtual_keyboard_manager_v1::create_virtual_keyboard(seat) → zwp_virtual_keyboard_v1

zwp_virtual_keyboard_v1::keymap(format, fd, size)  // MUST be called first
zwp_virtual_keyboard_v1::key(time, key, state)     // Inject key event
zwp_virtual_keyboard_v1::modifiers(depressed, latched, locked, group)
zwp_virtual_keyboard_v1::destroy()
```

**Critical Detail - Keymap Requirement:**
Before sending ANY key events, you MUST provide an XKB keymap to the compositor via a shared memory file descriptor. This is the tricky part.

```rust
// Pseudocode for keymap setup
let keymap_string = generate_xkb_keymap();  // XKB keymap as string
let fd = create_shm_fd(keymap_string.len());
write_all_to_fd(fd, keymap_string.as_bytes());
virtual_keyboard.keymap(KeymapFormat::XkbV1, fd, keymap_string.len() as u32);
```

### 3.2 wlr-virtual-pointer-unstable-v1 (zwlr namespace - WLROOTS ONLY)

**Protocol XML:** https://wayland.app/protocols/wlr-virtual-pointer-unstable-v1

**Interfaces:**
- `zwlr_virtual_pointer_manager_v1` - Factory (get from registry)
- `zwlr_virtual_pointer_v1` - Virtual pointer instance

**Key Methods:**
```
zwlr_virtual_pointer_manager_v1::create_virtual_pointer(seat) → zwlr_virtual_pointer_v1
zwlr_virtual_pointer_manager_v1::create_virtual_pointer_with_output(seat, output)

zwlr_virtual_pointer_v1::motion(time, dx, dy)                    // Relative motion
zwlr_virtual_pointer_v1::motion_absolute(time, x, y, x_extent, y_extent)  // Absolute
zwlr_virtual_pointer_v1::button(time, button, state)             // Mouse button
zwlr_virtual_pointer_v1::axis(time, axis, value)                 // Scroll
zwlr_virtual_pointer_v1::axis_source(source)                     // wheel/finger/etc
zwlr_virtual_pointer_v1::axis_stop(time, axis)
zwlr_virtual_pointer_v1::axis_discrete(axis, steps)              // Discrete scroll
zwlr_virtual_pointer_v1::frame()                                 // End of event group
zwlr_virtual_pointer_v1::destroy()
```

**Important:** Events should be grouped with `frame()` calls. Each logical input action (e.g., click) should end with `frame()`.

### 3.3 Button and Key Codes

Both protocols use **Linux evdev codes**, which lamco-rdp-server already translates to.

**Mouse buttons (evdev):**
- BTN_LEFT = 272 (0x110)
- BTN_RIGHT = 273 (0x111)
- BTN_MIDDLE = 274 (0x112)
- BTN_SIDE = 275 (0x113)
- BTN_EXTRA = 276 (0x114)

**Keyboard:** Uses evdev keycodes (KEY_A = 30, KEY_B = 48, etc.)
The existing `src/input/keyboard/` code already handles RDP scancode → evdev translation.

---

## 4. Rust Crates to Use

### Required Dependencies

```toml
[dependencies]
# Wayland client
wayland-client = "0.31"

# Protocol definitions
wayland-protocols = { version = "0.32", features = ["client", "unstable"] }
wayland-protocols-wlr = { version = "0.3", features = ["client"] }
wayland-protocols-misc = { version = "0.3", features = ["client"] }

# XKB keymap handling
xkbcommon = "0.8"

# Shared memory for keymap
rustix = { version = "0.38", features = ["mm", "fs"] }  # or use nix crate
```

### License Check (CRITICAL)

| Crate | License | Compatible with BUSL-1.1? |
|-------|---------|---------------------------|
| wayland-client | MIT | ✅ Yes |
| wayland-protocols | MIT | ✅ Yes |
| wayland-protocols-wlr | MIT | ✅ Yes |
| wayland-protocols-misc | MIT | ✅ Yes |
| xkbcommon | MIT | ✅ Yes |
| rustix | Apache-2.0/MIT | ✅ Yes |

**All clear - no GPL dependencies needed.**

---

## 5. Reference Implementations (Study Only - GPL)

These are GPL-licensed. Study the architecture and approach, but write your own code.

### 5.1 lan-mouse input-emulation

**URL:** https://github.com/feschber/lan-mouse/tree/main/input-emulation
**License:** GPL-3.0 (cannot copy code)

**What to study:**
- `src/wlroots.rs` - wlroots backend implementation
- `src/xdg_desktop_portal.rs` - Portal backend (for comparison)
- Event loop integration pattern
- Error handling approach

**Key files:**
- Backend selection: `src/lib.rs`
- wlroots impl: `src/wlroots.rs` (~400 lines)

### 5.2 wlrctl

**URL:** https://git.sr.ht/~brocellous/wlrctl
**License:** MIT ✅ (can reference more freely)

**What to study:**
- Simple, focused implementation
- Command-line tool for wlr input injection
- Clean C code, easy to understand

### 5.3 wayvnc

**URL:** https://github.com/any1/wayvnc
**License:** ISC (permissive) ✅

**What to study:**
- `src/keyboard.c` - Keyboard handling with XKB
- `src/pointer.c` - Pointer handling
- Production-quality VNC server using wlr protocols

### 5.4 ydotool

**URL:** https://github.com/ReimuNotMoe/ydotool
**License:** AGPL-3.0 (cannot copy)

**What to study:**
- uinput approach (alternative to Wayland protocols)
- Might be useful for fallback

---

## 6. Implementation Plan

### Phase 1: Research (1-2 days)

**Tasks:**
1. [ ] Read wayland-client Rust documentation
2. [ ] Study wayland-protocols-wlr crate API
3. [ ] Understand XKB keymap format and generation
4. [ ] Read wlrctl source (MIT, can reference closely)
5. [ ] Read wayvnc keyboard.c for XKB patterns
6. [ ] Understand lan-mouse wlroots.rs architecture (GPL, study only)

**Key Questions to Answer:**
- How to create and manage Wayland event loop alongside tokio?
- How to generate a minimal XKB keymap that covers all keys?
- How to share keymap via memfd/shm?
- How to detect if compositor supports wlr-virtual-pointer?

### Phase 2: Scaffold (2-3 days)

**Tasks:**
1. [ ] Create `src/session/strategies/wlr_direct.rs`
2. [ ] Add Wayland connection management
3. [ ] Implement protocol binding (get manager interfaces from registry)
4. [ ] Create `WlrSessionHandle` struct
5. [ ] Add to strategy selector

**File structure:**
```
src/session/strategies/
├── wlr_direct.rs          # NEW - Main implementation
├── wlr_keyboard.rs        # NEW - Keyboard + keymap handling
├── wlr_pointer.rs         # NEW - Pointer handling
├── portal_token.rs        # Existing
├── mutter_direct.rs       # Existing
└── selector.rs            # Modify - Add wlr detection
```

### Phase 3: Virtual Pointer (2-3 days)

**Tasks:**
1. [ ] Implement `zwlr_virtual_pointer_v1` client
2. [ ] Implement `notify_pointer_motion_absolute()`
3. [ ] Implement `notify_pointer_button()`
4. [ ] Implement `notify_pointer_axis()`
5. [ ] Handle `frame()` calls correctly
6. [ ] Test on Sway

**This is the easier part - pointer protocol is straightforward.**

### Phase 4: Virtual Keyboard (5-7 days)

**Tasks:**
1. [ ] Research XKB keymap generation
2. [ ] Implement minimal keymap generator (or use xkbcommon)
3. [ ] Implement shared memory keymap transfer
4. [ ] Implement `zwp_virtual_keyboard_v1` client
5. [ ] Implement `notify_keyboard_keycode()`
6. [ ] Handle modifier state synchronization
7. [ ] Test on Sway with various key combinations

**This is the hard part - keymap handling is tricky.**

### Phase 5: Integration & Testing (3-5 days)

**Tasks:**
1. [ ] Add compositor detection (wlr vs portal)
2. [ ] Add config option: `input_backend = "auto" | "portal" | "wlr-direct"`
3. [ ] Test on Sway
4. [ ] Test on Hyprland
5. [ ] Test on River
6. [ ] Test keyboard edge cases (modifiers, special keys)
7. [ ] Test mouse edge cases (scroll, multi-button)

---

## 7. Technical Deep-Dives Needed

### 7.1 Wayland + Tokio Integration

lamco-rdp-server uses tokio for async. Wayland client has its own event loop.

**Options to research:**
1. `wayland-client` with `calloop` integration
2. Running Wayland dispatch in separate thread
3. Using `wayland_client::EventQueue::poll_dispatch_pending()` in tokio task

**Reference:** Check how lan-mouse handles this (they use tokio too).

### 7.2 XKB Keymap Generation

The virtual keyboard requires an XKB keymap. Options:

**Option A: Minimal hardcoded keymap**
```
xkb_keymap {
    xkb_keycodes { ... };
    xkb_types { ... };
    xkb_compat { ... };
    xkb_symbols { ... };
};
```

**Option B: Use xkbcommon to generate**
```rust
let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
let keymap = xkb::Keymap::new_from_names(
    &context,
    &xkb::RuleNames::default(),  // Use system defaults
    xkb::KEYMAP_COMPILE_NO_FLAGS,
);
let keymap_string = keymap.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1);
```

**Option C: Read system keymap**
- Find current XKB keymap from environment
- This matches what compositor is using

**Recommendation:** Start with Option B, it's most reliable.

### 7.3 Shared Memory for Keymap

Keymap must be passed as a file descriptor to shared memory.

```rust
use rustix::fs::{memfd_create, MemfdFlags};
use rustix::io::write;

fn create_keymap_fd(keymap: &str) -> Result<OwnedFd> {
    let fd = memfd_create("xkb-keymap", MemfdFlags::CLOEXEC)?;
    write(&fd, keymap.as_bytes())?;
    // Seek to beginning
    rustix::fs::seek(&fd, SeekFrom::Start(0))?;
    Ok(fd)
}
```

### 7.4 Coordinate Systems

lamco-rdp-server already handles coordinate transformation in `src/input/coordinates/`.

For wlr-virtual-pointer `motion_absolute()`:
- `x`, `y` are absolute coordinates
- `x_extent`, `y_extent` define the coordinate space (usually screen dimensions)

**Check:** Does existing coordinate transformer output suitable values?

---

## 8. Key Files to Read

### In lamco-rdp-server

| File | Purpose |
|------|---------|
| `src/session/strategy.rs` | SessionHandle trait definition |
| `src/session/strategies/portal_token.rs` | Reference implementation |
| `src/server/input_handler.rs` | How input events flow |
| `src/input/keyboard/mod.rs` | Scancode translation |
| `src/input/keyboard/scancode_map.rs` | RDP → evdev mapping |
| `src/input/mouse/mod.rs` | Mouse state handling |
| `src/input/coordinates/mod.rs` | Coordinate transformation |
| `Cargo.toml` | Current dependencies, license |

### External References

| Resource | URL |
|----------|-----|
| virtual-keyboard protocol | https://wayland.app/protocols/virtual-keyboard-unstable-v1 |
| wlr-virtual-pointer protocol | https://wayland.app/protocols/wlr-virtual-pointer-unstable-v1 |
| wayland-client docs | https://docs.rs/wayland-client/latest/wayland_client/ |
| wayland-protocols-wlr docs | https://docs.rs/wayland-protocols-wlr/latest/wayland_protocols_wlr/ |
| xkbcommon docs | https://docs.rs/xkbcommon/latest/xkbcommon/ |
| wlrctl source (MIT) | https://git.sr.ht/~brocellous/wlrctl |
| wayvnc source (ISC) | https://github.com/any1/wayvnc |
| lan-mouse wlroots (GPL, study) | https://github.com/feschber/lan-mouse/blob/main/input-emulation/src/wlroots.rs |

---

## 9. Success Criteria

### Minimum Viable Implementation

1. [ ] Can inject keyboard events on Sway
2. [ ] Can inject mouse movement on Sway
3. [ ] Can inject mouse clicks on Sway
4. [ ] Can inject scroll events on Sway
5. [ ] Modifier keys work (Ctrl, Alt, Shift, Super)
6. [ ] Special keys work (F1-F12, arrows, Home/End, etc.)

### Full Implementation

1. [ ] All above, plus:
2. [ ] Works on Hyprland
3. [ ] Works on River
4. [ ] Automatic backend detection (portal vs wlr-direct)
5. [ ] Config option to force backend
6. [ ] Graceful fallback if protocols unavailable
7. [ ] Proper cleanup on disconnect

---

## 10. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| XKB keymap complexity | Start with xkbcommon library, don't hand-write |
| Wayland event loop integration | Study lan-mouse's tokio integration |
| Compositor differences | Test on multiple compositors early |
| Protocol version mismatches | Check protocol versions, handle gracefully |
| License contamination | Only read GPL code, write fresh implementation |

---

## 11. Questions for Research Session

When starting the new session, investigate these:

1. **Wayland-client + tokio:** What's the recommended pattern for using wayland-client with tokio async runtime?

2. **XKB keymap:** What's the minimal XKB keymap that supports all standard keys? Can xkbcommon generate one from system defaults?

3. **Protocol availability:** How to detect if compositor supports `zwlr_virtual_pointer_manager_v1`? What if it doesn't?

4. **Absolute coordinates:** What coordinate space does `motion_absolute()` expect? Screen pixels? Normalized 0-1?

5. **Frame timing:** When exactly should `frame()` be called? After every event? After a logical group?

6. **Modifier sync:** How to handle keyboard modifier state synchronization between RDP client and compositor?

7. **Multi-seat:** Do we need to handle multiple seats? Which seat to bind to?

---

## 12. Starting the Implementation

### First Steps in New Session

1. Read this document fully
2. Read `src/session/strategy.rs` and `src/session/strategies/portal_token.rs`
3. Create skeleton `src/session/strategies/wlr_direct.rs`
4. Add dependencies to `Cargo.toml`
5. Implement basic Wayland connection
6. Get `zwlr_virtual_pointer_manager_v1` from registry
7. Create virtual pointer and test basic motion

### Suggested First Test

```rust
// Minimal test: move mouse in a circle
for i in 0..360 {
    let angle = (i as f64) * std::f64::consts::PI / 180.0;
    let x = 500.0 + 100.0 * angle.cos();
    let y = 500.0 + 100.0 * angle.sin();
    virtual_pointer.motion_absolute(timestamp(), x, y, 1920, 1080);
    virtual_pointer.frame();
    tokio::time::sleep(Duration::from_millis(10)).await;
}
```

If the mouse moves in a circle on Sway, the basic infrastructure is working.

---

## Appendix A: Protocol XML Snippets

### virtual-keyboard-unstable-v1 (key parts)

```xml
<interface name="zwp_virtual_keyboard_manager_v1" version="1">
  <request name="create_virtual_keyboard">
    <arg name="seat" type="object" interface="wl_seat"/>
    <arg name="id" type="new_id" interface="zwp_virtual_keyboard_v1"/>
  </request>
</interface>

<interface name="zwp_virtual_keyboard_v1" version="1">
  <request name="keymap">
    <arg name="format" type="uint"/>
    <arg name="fd" type="fd"/>
    <arg name="size" type="uint"/>
  </request>
  <request name="key">
    <arg name="time" type="uint"/>
    <arg name="key" type="uint"/>
    <arg name="state" type="uint"/>
  </request>
  <request name="modifiers">
    <arg name="mods_depressed" type="uint"/>
    <arg name="mods_latched" type="uint"/>
    <arg name="mods_locked" type="uint"/>
    <arg name="group" type="uint"/>
  </request>
  <request name="destroy" type="destructor"/>
</interface>
```

### wlr-virtual-pointer-unstable-v1 (key parts)

```xml
<interface name="zwlr_virtual_pointer_manager_v1" version="2">
  <request name="create_virtual_pointer">
    <arg name="seat" type="object" interface="wl_seat" allow-null="true"/>
    <arg name="id" type="new_id" interface="zwlr_virtual_pointer_v1"/>
  </request>
</interface>

<interface name="zwlr_virtual_pointer_v1" version="2">
  <request name="motion">
    <arg name="time" type="uint"/>
    <arg name="dx" type="fixed"/>
    <arg name="dy" type="fixed"/>
  </request>
  <request name="motion_absolute">
    <arg name="time" type="uint"/>
    <arg name="x" type="uint"/>
    <arg name="y" type="uint"/>
    <arg name="x_extent" type="uint"/>
    <arg name="y_extent" type="uint"/>
  </request>
  <request name="button">
    <arg name="time" type="uint"/>
    <arg name="button" type="uint"/>
    <arg name="state" type="uint"/>
  </request>
  <request name="axis">
    <arg name="time" type="uint"/>
    <arg name="axis" type="uint"/>
    <arg name="value" type="fixed"/>
  </request>
  <request name="frame"/>
  <request name="destroy" type="destructor"/>
</interface>
```

---

## Appendix B: Market Context

### Why This Matters

**wlroots compositor market share (Arch Linux, Dec 2025):**
- Sway: ~12.4%
- Hyprland: ~12.6% (fastest growing WM in 15 years)
- River, labwc, others: ~3-5%
- **Combined: ~25-30% of WM users**

These users currently CANNOT use lamco-rdp-server because xdg-desktop-portal-wlr doesn't implement RemoteDesktop.

### Competition

- **wayvnc:** Works on wlroots (VNC, not RDP)
- **RustDesk:** Working on libei support, limited wlroots
- **GNOME Remote Desktop:** GNOME only

Adding wlr support makes lamco-rdp-server the **first RDP server** with native wlroots support.

---

*End of handover document. Good luck with the implementation!*
