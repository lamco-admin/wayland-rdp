# WRD-Server Protocol Specifications

## Document Information
- **Version**: 1.0.0
- **Last Updated**: 2024-11-18
- **Status**: Production-Ready
- **Classification**: Technical Protocol Specification

## Table of Contents
1. [Overview](#1-overview)
2. [Portal D-Bus Protocol](#2-portal-d-bus-protocol)
3. [PipeWire Protocol](#3-pipewire-protocol)
4. [IronRDP Server Traits](#4-ironrdp-server-traits)
5. [RDP Protocol Integration](#5-rdp-protocol-integration)
6. [Protocol Interactions](#6-protocol-interactions)
7. [Error Handling](#7-error-handling)
8. [Timing Requirements](#8-timing-requirements)

## 1. Overview

This document provides complete protocol specifications for all inter-process communication and network protocols used in the wrd-server implementation. Each protocol is specified with exact method signatures, parameter types, return values, error codes, and interaction sequences.

### 1.1 Protocol Stack

```
┌─────────────────────────────────────┐
│        RDP Client (mstsc)           │
└──────────────┬──────────────────────┘
               │ RDP Protocol
┌──────────────▼──────────────────────┐
│     IronRDP Server Framework        │
│  (Traits: InputHandler, Display)    │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│        WRD-Server Core              │
└──────┬────────────────┬─────────────┘
       │                │
┌──────▼────┐    ┌──────▼─────────────┐
│  D-Bus    │    │    PipeWire        │
│  Portal   │    │    Protocol        │
└──────┬────┘    └──────┬─────────────┘
       │                │
┌──────▼────────────────▼─────────────┐
│    Wayland Compositor (GNOME/KDE)   │
└──────────────────────────────────────┘
```

## 2. Portal D-Bus Protocol

### 2.1 org.freedesktop.portal.ScreenCast

#### 2.1.1 Interface Definition

```xml
<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"
"http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
<node>
  <interface name="org.freedesktop.portal.ScreenCast">
    <property name="version" type="u" access="read"/>
    <property name="AvailableSourceTypes" type="u" access="read"/>
    <property name="AvailableCursorModes" type="u" access="read"/>

    <method name="CreateSession">
      <arg type="a{sv}" name="options" direction="in"/>
      <arg type="o" name="handle" direction="out"/>
    </method>

    <method name="SelectSources">
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="a{sv}" name="options" direction="in"/>
      <arg type="o" name="request_handle" direction="out"/>
    </method>

    <method name="Start">
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="s" name="parent_window" direction="in"/>
      <arg type="a{sv}" name="options" direction="in"/>
      <arg type="o" name="request_handle" direction="out"/>
    </method>

    <method name="OpenPipeWireRemote">
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="a{sv}" name="options" direction="in"/>
      <arg type="h" name="fd" direction="out"/>
    </method>
  </interface>
</node>
```

#### 2.1.2 Method Specifications

##### CreateSession

**Signature**: `CreateSession(options: a{sv}) -> handle: o`

**Parameters**:
- `options`: Dictionary of session options
  - `handle_token` (s): Unique token for request tracking
  - `session_handle_token` (s): Unique token for session identification

**Returns**:
- `handle`: Object path to session handle

**Error Codes**:
- `org.freedesktop.portal.Error.Failed`: Generic failure
- `org.freedesktop.portal.Error.InvalidArgument`: Invalid options
- `org.freedesktop.portal.Error.NotAllowed`: Permission denied

**Sequence Diagram**:
```
Client              Portal              Compositor
  │                   │                     │
  ├──CreateSession──►│                     │
  │                   ├──Validate──────────►│
  │                   │                     │
  │◄──/o/f/p/s/123───┤◄──Session OK────────┤
  │                   │                     │
```

##### SelectSources

**Signature**: `SelectSources(session_handle: o, options: a{sv}) -> request_handle: o`

**Parameters**:
- `session_handle`: Object path from CreateSession
- `options`: Source selection options
  - `multiple` (b): Allow multiple source selection (default: false)
  - `types` (u): Bitmask of source types (1=Monitor, 2=Window, 4=Virtual)
  - `cursor_mode` (u): Cursor capture mode (0=Hidden, 1=Embedded, 2=Metadata)
  - `restore_token` (s): Token to restore previous selection
  - `persist_mode` (u): Persistence mode (0=DoNot, 1=Application, 2=ExplicitGrant)

**Returns**:
- `request_handle`: Object path to track request

**Error Codes**:
- `org.freedesktop.portal.Error.Failed`: Selection failed
- `org.freedesktop.portal.Error.InvalidArgument`: Invalid source types
- `org.freedesktop.portal.Error.Cancelled`: User cancelled selection

**Timing Requirements**:
- User dialog timeout: 60 seconds
- Response timeout: 5 seconds after user action

##### Start

**Signature**: `Start(session_handle: o, parent_window: s, options: a{sv}) -> request_handle: o`

**Parameters**:
- `session_handle`: Active session handle
- `parent_window`: Parent window identifier (empty for headless)
- `options`: Start options (reserved for future use)

**Returns**:
- `request_handle`: Request tracking handle

**Response Signal**:
```
signal Response (
  response: u,  // 0=Success, 1=UserCancelled, 2=Other
  results: a{sv} {
    streams: aa{sv} [
      {
        node_id: u,        // PipeWire node ID
        position: (ii),    // x,y position
        size: (ii),        // width,height
        source_type: u     // 1=Monitor, 2=Window
      }
    ],
    restore_token: s       // Token for future restore
  }
)
```

##### OpenPipeWireRemote

**Signature**: `OpenPipeWireRemote(session_handle: o, options: a{sv}) -> fd: h`

**Parameters**:
- `session_handle`: Active session with started streams
- `options`: Reserved for future use

**Returns**:
- `fd`: Unix file descriptor to PipeWire remote

**Error Codes**:
- `org.freedesktop.portal.Error.Failed`: PipeWire connection failed
- `org.freedesktop.portal.Error.NotAllowed`: Session not started

### 2.2 org.freedesktop.portal.RemoteDesktop

#### 2.2.1 Interface Definition

```xml
<interface name="org.freedesktop.portal.RemoteDesktop">
  <property name="version" type="u" access="read"/>
  <property name="AvailableDeviceTypes" type="u" access="read"/>

  <method name="CreateSession">
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="o" name="handle" direction="out"/>
  </method>

  <method name="SelectDevices">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="o" name="request_handle" direction="out"/>
  </method>

  <method name="Start">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="s" name="parent_window" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
    <arg type="o" name="request_handle" direction="out"/>
  </method>

  <method name="NotifyPointerMotion">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="d" name="dx" direction="in"/>
    <arg type="d" name="dy" direction="in"/>
  </method>

  <method name="NotifyPointerMotionAbsolute">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="u" name="stream" direction="in"/>
    <arg type="d" name="x" direction="in"/>
    <arg type="d" name="y" direction="in"/>
  </method>

  <method name="NotifyPointerButton">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="i" name="button" direction="in"/>
    <arg type="u" name="state" direction="in"/>
  </method>

  <method name="NotifyPointerAxis">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="d" name="dx" direction="in"/>
    <arg type="d" name="dy" direction="in"/>
    <arg type="b" name="finish" direction="in"/>
  </method>

  <method name="NotifyPointerAxisDiscrete">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="u" name="axis" direction="in"/>
    <arg type="i" name="steps" direction="in"/>
  </method>

  <method name="NotifyKeyboardKeycode">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="i" name="keycode" direction="in"/>
    <arg type="u" name="state" direction="in"/>
  </method>

  <method name="NotifyKeyboardKeysym">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="i" name="keysym" direction="in"/>
    <arg type="u" name="state" direction="in"/>
  </method>

  <method name="NotifyTouchDown">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="u" name="stream" direction="in"/>
    <arg type="u" name="slot" direction="in"/>
    <arg type="d" name="x" direction="in"/>
    <arg type="d" name="y" direction="in"/>
  </method>

  <method name="NotifyTouchMotion">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="u" name="stream" direction="in"/>
    <arg type="u" name="slot" direction="in"/>
    <arg type="d" name="x" direction="in"/>
    <arg type="d" name="y" direction="in"/>
  </method>

  <method name="NotifyTouchUp">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="u" name="slot" direction="in"/>
  </method>
</interface>
```

#### 2.2.2 Method Specifications

##### SelectDevices

**Signature**: `SelectDevices(session_handle: o, options: a{sv}) -> request_handle: o`

**Parameters**:
- `session_handle`: Session from CreateSession
- `options`: Device selection options
  - `types` (u): Device type bitmask
    - 0x01: Keyboard
    - 0x02: Pointer
    - 0x04: TouchScreen
  - `restore_token` (s): Previous selection token
  - `persist_mode` (u): Persistence mode

**Device Type Flags**:
```rust
const DEVICE_KEYBOARD: u32 = 0x01;
const DEVICE_POINTER: u32 = 0x02;
const DEVICE_TOUCHSCREEN: u32 = 0x04;
```

##### Input Injection Methods

**NotifyPointerMotion**
- `dx` (d): Relative X movement in pixels
- `dy` (d): Relative Y movement in pixels
- Range: -32768.0 to 32767.0
- Timing: < 16ms for 60fps responsiveness

**NotifyPointerMotionAbsolute**
- `stream` (u): PipeWire node ID
- `x` (d): Absolute X position (0.0 to stream_width)
- `y` (d): Absolute Y position (0.0 to stream_height)
- Coordinate space: Stream-relative

**NotifyPointerButton**
- `button` (i): Button code
  - 0x110 (272): BTN_LEFT
  - 0x111 (273): BTN_RIGHT
  - 0x112 (274): BTN_MIDDLE
  - 0x113 (275): BTN_SIDE
  - 0x114 (276): BTN_EXTRA
- `state` (u): Button state
  - 0: Released
  - 1: Pressed

**NotifyPointerAxis**
- `dx` (d): Horizontal scroll (-1.0 to 1.0 per notch)
- `dy` (d): Vertical scroll (-1.0 to 1.0 per notch)
- `finish` (b): Axis event complete

**NotifyKeyboardKeycode**
- `keycode` (i): Linux input keycode (8-255)
- `state` (u): Key state (0=Released, 1=Pressed)

### 2.3 org.freedesktop.portal.Clipboard

#### 2.3.1 Interface Definition

```xml
<interface name="org.freedesktop.portal.Clipboard">
  <method name="RequestClipboard">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
  </method>

  <method name="SetSelection">
    <arg type="o" name="session_handle" direction="in"/>
    <arg type="a{sv}" name="options" direction="in"/>
  </method>

  <signal name="SelectionOwnerChanged">
    <arg type="o" name="session_handle"/>
    <arg type="a{sv}" name="options"/>
  </signal>

  <signal name="SelectionTransfer">
    <arg type="o" name="session_handle"/>
    <arg type="s" name="mime_type"/>
    <arg type="h" name="fd"/>
  </signal>
</interface>
```

#### 2.3.2 Clipboard Data Transfer

**MIME Types Supported**:
- `text/plain`: Plain text
- `text/plain;charset=utf-8`: UTF-8 text
- `text/html`: HTML content
- `image/png`: PNG images
- `image/jpeg`: JPEG images
- `application/octet-stream`: Binary data

**Transfer Protocol**:
```
Client              Portal              Compositor
  │                   │                     │
  ├─RequestClipboard─►│                     │
  │                   ├──Get Selection─────►│
  │                   │◄──MIME Types────────┤
  │◄──Available Types─┤                     │
  │                   │                     │
  ├──Request Type────►│                     │
  │                   ├──Request Data──────►│
  │                   │◄──Data FD───────────┤
  │◄──Transfer FD─────┤                     │
  │                   │                     │
```

### 2.4 Session Lifecycle Protocol

#### 2.4.1 Session States

```
         ┌─────────┐
         │ Created │
         └────┬────┘
              │ SelectSources/Devices
         ┌────▼────┐
         │Selected │
         └────┬────┘
              │ Start
         ┌────▼────┐
         │ Active  │
         └────┬────┘
              │ Close/Timeout
         ┌────▼────┐
         │ Closed  │
         └─────────┘
```

#### 2.4.2 Session Handle Format

```
/org/freedesktop/portal/desktop/session/{sender_id}/{token}

Example:
/org/freedesktop/portal/desktop/session/1_42/wrd_abc123def456
```

#### 2.4.3 Permission Grant Flow

```
Client              Portal              PolicyKit         User
  │                   │                     │              │
  ├──CreateSession───►│                     │              │
  │                   ├──Check Policy──────►│              │
  │                   │◄──Need User─────────┤              │
  │                   ├──Show Dialog────────────────────►│
  │                   │                     │              │
  │                   │◄──Grant/Deny──────────────────────┤
  │◄──Response────────┤                     │              │
  │                   │                     │              │
```

**Timing**:
- Dialog timeout: 60 seconds
- Session timeout: 3600 seconds (1 hour) idle
- Keep-alive interval: 30 seconds

## 3. PipeWire Protocol

### 3.1 Stream Connection Protocol

#### 3.1.1 Connection Sequence

```
Client            PipeWire           Node
  │                  │                 │
  ├─pw_init─────────►│                 │
  ├─pw_context_new──►│                 │
  ├─pw_core_connect─►│                 │
  │                  ├──Registry──────►│
  │◄─────────────────┤                 │
  ├─create_stream───►│                 │
  │                  ├──Node Create───►│
  │                  │◄──Node ID───────┤
  ├─stream_connect──►│                 │
  │                  ├──Format Nego───►│
  │                  │◄──Format OK─────┤
  │◄──STATE_STREAMING┤                 │
  │                  │                 │
```

#### 3.1.2 PipeWire API Methods

```c
// Core connection
struct pw_core* pw_context_connect(
    struct pw_context *context,
    struct pw_properties *properties,
    size_t user_data_size
);

// Stream creation
struct pw_stream* pw_stream_new(
    struct pw_core *core,
    const char *name,
    struct pw_properties *props
);

// Stream connection
int pw_stream_connect(
    struct pw_stream *stream,
    enum pw_direction direction,
    uint32_t target_id,
    enum pw_stream_flags flags,
    const struct spa_pod **params,
    uint32_t n_params
);

// Buffer dequeue
struct pw_buffer* pw_stream_dequeue_buffer(
    struct pw_stream *stream
);

// Buffer queue
int pw_stream_queue_buffer(
    struct pw_stream *stream,
    struct pw_buffer *buffer
);
```

### 3.2 Format Negotiation

#### 3.2.1 SPA Pod Format Structure

```c
struct spa_video_info_raw {
    enum spa_video_format format;  // SPA_VIDEO_FORMAT_*
    int32_t width;
    int32_t height;
    struct spa_fraction framerate;
    struct spa_fraction max_framerate;
    uint64_t modifier;  // DRM format modifier
};
```

#### 3.2.2 Supported Formats

```
Format             FourCC    Bits    Layout
─────────────────────────────────────────
SPA_VIDEO_FORMAT_RGB    RGB   24     RGB888
SPA_VIDEO_FORMAT_RGBA   RGBA  32     RGBA8888
SPA_VIDEO_FORMAT_BGRA   BGRA  32     BGRA8888
SPA_VIDEO_FORMAT_BGRx   BGRx  32     BGRx8888
SPA_VIDEO_FORMAT_YUY2   YUY2  16     YUYV422
SPA_VIDEO_FORMAT_I420   I420  12     YUV420P
```

#### 3.2.3 Format Negotiation Sequence

```
Stream              PipeWire            Source
  │                    │                  │
  ├──EnumFormat(0)────►│                  │
  │                    ├──Query Format───►│
  │                    │◄──Format List────┤
  │◄──Format Choice────┤                  │
  │                    │                  │
  ├──SetFormat────────►│                  │
  │                    ├──Set Format─────►│
  │                    │◄──Accept─────────┤
  │◄──Format Set───────┤                  │
  │                    │                  │
```

### 3.3 Buffer Exchange Protocol

#### 3.3.1 Buffer Structure

```c
struct pw_buffer {
    struct spa_buffer *buffer;
    void *user_data;
    uint64_t size;
    uint64_t requested_size;
};

struct spa_buffer {
    uint32_t n_datas;
    struct spa_data *datas;
    uint32_t n_metas;
    struct spa_meta *metas;
};

struct spa_data {
    uint32_t type;      // SPA_DATA_*
    uint32_t flags;
    int64_t fd;         // DMA-BUF fd
    uint32_t mapoffset;
    uint32_t maxsize;
    void *data;
    struct spa_chunk *chunk;
};

struct spa_chunk {
    uint32_t offset;
    uint32_t size;
    int32_t stride;
    int32_t flags;      // SPA_CHUNK_FLAG_*
};
```

#### 3.3.2 Buffer Flow Sequence

```
Consumer           PipeWire           Producer
  │                   │                  │
  ├─Dequeue──────────►│                  │
  │◄──Buffer──────────┤                  │
  │                   │◄──Process────────┤
  │                   ├──Buffer Ready───►│
  ├─Process Buffer    │                  │
  ├─Queue─────────────►│                  │
  │                   ├──Recycle────────►│
  │                   │                  │
```

**Timing Requirements**:
- Dequeue timeout: 100ms
- Process time: < 16.67ms (60fps)
- Queue deadline: Next frame time

### 3.4 DMA-BUF Protocol

#### 3.4.1 DMA-BUF Import

```c
// Import DMA-BUF
int import_dmabuf(int fd, uint32_t width, uint32_t height,
                  uint32_t format, uint64_t modifier) {
    struct dma_buf_sync sync = {0};
    sync.flags = DMA_BUF_SYNC_START | DMA_BUF_SYNC_READ;

    // Begin CPU access
    ioctl(fd, DMA_BUF_IOCTL_SYNC, &sync);

    // Map buffer
    void *data = mmap(NULL, size, PROT_READ, MAP_SHARED, fd, 0);

    // Process data...

    // End CPU access
    sync.flags = DMA_BUF_SYNC_END | DMA_BUF_SYNC_READ;
    ioctl(fd, DMA_BUF_IOCTL_SYNC, &sync);

    munmap(data, size);
}
```

#### 3.4.2 Modifier Negotiation

```
Common Modifiers:
0x0000000000000000  LINEAR         Linear layout
0x0100000000000001  I915_X_TILED   Intel X-tiling
0x0100000000000002  I915_Y_TILED   Intel Y-tiling
0x0100000000000004  I915_Yf_TILED  Intel Yf-tiling
```

### 3.5 Stream State Transitions

```
    ┌──────────┐
    │  ERROR   │◄─────────────┐
    └──────────┘              │
                              │
    ┌──────────┐              │
┌──►│UNCONNECTED◄──┐          │
│   └─────┬────┘   │          │
│         │Connect │          │Error
│   ┌─────▼────┐   │          │
│   │CONNECTING├───┼──────────┤
│   └─────┬────┘   │          │
│         │Ready   │          │
│   ┌─────▼────┐   │          │
│   │  PAUSED  ├───┼──────────┤
│   └─────┬────┘   │          │
│         │Start   │Disconnect│
│   ┌─────▼────┐   │          │
│   │STREAMING ├───┼──────────┤
│   └─────┬────┘   │          │
│         │Stop    │          │
└─────────┴────────┘          │
```

**State Event Handlers**:
```c
static void on_stream_state_changed(
    void *data,
    enum pw_stream_state old,
    enum pw_stream_state state,
    const char *error
);
```

## 4. IronRDP Server Traits

### 4.1 RdpServerInputHandler Trait

#### 4.1.1 Complete Trait Definition

```rust
pub trait RdpServerInputHandler: Send {
    /// Handle keyboard input event
    /// Called on: RDP input thread
    /// Must complete within: 1ms
    fn keyboard(&mut self, event: KeyboardEvent);

    /// Handle mouse input event
    /// Called on: RDP input thread
    /// Must complete within: 1ms
    fn mouse(&mut self, event: MouseEvent);

    /// Handle extended mouse input
    /// Called on: RDP input thread
    /// Must complete within: 1ms
    fn mouse_extended(&mut self, event: ExtendedMouseEvent) {
        // Default: Convert to standard mouse event
    }

    /// Handle unicode keyboard input
    /// Called on: RDP input thread
    /// Must complete within: 1ms
    fn unicode(&mut self, event: UnicodeEvent) {
        // Default: Convert to keyboard event
    }
}
```

#### 4.1.2 KeyboardEvent Specification

```rust
#[derive(Debug, Clone, Copy)]
pub enum KeyboardEvent {
    /// Key pressed event
    Pressed {
        /// Scan code (0-255)
        code: u8,
        /// Extended key flag (E0 prefix)
        extended: bool
    },

    /// Key released event
    Released {
        /// Scan code (0-255)
        code: u8,
        /// Extended key flag (E0 prefix)
        extended: bool
    },

    /// Unicode character pressed
    UnicodePressed(u16),

    /// Unicode character released
    UnicodeReleased(u16),

    /// Keyboard LED synchronization
    Synchronize(SynchronizeFlags),
}

#[derive(Debug, Clone, Copy)]
pub struct SynchronizeFlags {
    pub scroll_lock: bool,
    pub num_lock: bool,
    pub caps_lock: bool,
    pub kana_lock: bool,
}
```

**Scan Code Mapping**:
```
RDP Code    Linux Code    Key
────────────────────────────
0x01        1            ESC
0x02        2            1
0x03        3            2
0x0E        14           BACKSPACE
0x0F        15           TAB
0x1C        28           ENTER
0x1D        29           LEFT_CTRL
0x2A        42           LEFT_SHIFT
0x36        54           RIGHT_SHIFT
0x38        56           LEFT_ALT
0x39        57           SPACE
0x3A        58           CAPS_LOCK
0x3B-0x44   59-68        F1-F10
0x45        69           NUM_LOCK
0x46        70           SCROLL_LOCK
0x47        71           HOME (KP_7)
0x48        72           UP (KP_8)
0x49        73           PAGE_UP (KP_9)
0x4B        75           LEFT (KP_4)
0x4D        77           RIGHT (KP_6)
0x4F        79           END (KP_1)
0x50        80           DOWN (KP_2)
0x51        81           PAGE_DOWN (KP_3)
0x52        82           INSERT (KP_0)
0x53        83           DELETE (KP_DOT)
0x57        87           F11
0x58        88           F12
0xE0,0x1D   97           RIGHT_CTRL
0xE0,0x38   100          RIGHT_ALT
0xE0,0x47   102          HOME
0xE0,0x48   103          UP
0xE0,0x49   104          PAGE_UP
0xE0,0x4B   105          LEFT
0xE0,0x4D   106          RIGHT
0xE0,0x4F   107          END
0xE0,0x50   108          DOWN
0xE0,0x51   109          PAGE_DOWN
0xE0,0x52   110          INSERT
0xE0,0x53   111          DELETE
0xE0,0x5B   125          LEFT_WIN
0xE0,0x5C   126          RIGHT_WIN
0xE0,0x5D   127          MENU
```

#### 4.1.3 MouseEvent Specification

```rust
#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    /// Absolute mouse movement
    Move {
        /// X coordinate (0-65535, scaled to desktop)
        x: u16,
        /// Y coordinate (0-65535, scaled to desktop)
        y: u16
    },

    /// Right button pressed
    RightPressed,

    /// Right button released
    RightReleased,

    /// Left button pressed
    LeftPressed,

    /// Left button released
    LeftReleased,

    /// Middle button pressed
    MiddlePressed,

    /// Middle button released
    MiddleReleased,

    /// Extended button 4 pressed (back)
    Button4Pressed,

    /// Extended button 4 released
    Button4Released,

    /// Extended button 5 pressed (forward)
    Button5Pressed,

    /// Extended button 5 released
    Button5Released,

    /// Vertical scroll wheel
    VerticalScroll {
        /// Scroll amount (-120 = up, +120 = down)
        value: i16
    },

    /// Horizontal scroll wheel
    HorizontalScroll {
        /// Scroll amount (-120 = left, +120 = right)
        value: i16
    },

    /// High-precision scroll
    Scroll {
        /// Horizontal scroll delta
        x: i32,
        /// Vertical scroll delta
        y: i32
    },

    /// Relative mouse movement
    RelMove {
        /// Relative X movement
        x: i32,
        /// Relative Y movement
        y: i32
    },
}
```

**Coordinate Scaling**:
```rust
fn scale_mouse_position(x: u16, y: u16, desktop_width: u32, desktop_height: u32) -> (f64, f64) {
    let scaled_x = (x as f64 / 65535.0) * desktop_width as f64;
    let scaled_y = (y as f64 / 65535.0) * desktop_height as f64;
    (scaled_x, scaled_y)
}
```

### 4.2 RdpServerDisplay Trait

#### 4.2.1 Complete Trait Definition

```rust
#[async_trait]
pub trait RdpServerDisplay: Send {
    /// Return current desktop size
    /// Called on: RDP main thread
    /// Must return within: 100ms
    async fn size(&mut self) -> DesktopSize;

    /// Return display updates receiver
    /// Called on: RDP main thread once at connection
    /// Must return within: 500ms
    async fn updates(&mut self) -> Result<Box<dyn RdpServerDisplayUpdates>>;

    /// Handle display layout request from client
    /// Called on: RDP main thread
    /// Must return within: 100ms
    fn request_layout(&mut self, layout: DisplayControlMonitorLayout) {
        // Default: ignore layout requests
    }

    /// Handle refresh request
    /// Called on: RDP main thread
    /// Must return within: 10ms
    fn request_refresh(&mut self, rect: Option<Rectangle>) {
        // Default: ignore refresh requests
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DesktopSize {
    pub width: u16,   // 1-7680 (8K max)
    pub height: u16,  // 1-4320 (8K max)
}

#[derive(Debug, Clone)]
pub struct Rectangle {
    pub left: u16,
    pub top: u16,
    pub right: u16,    // Exclusive
    pub bottom: u16,   // Exclusive
}
```

#### 4.2.2 DisplayControlMonitorLayout

```rust
#[derive(Debug, Clone)]
pub struct DisplayControlMonitorLayout {
    pub flags: u32,
    pub monitors: Vec<Monitor>,
}

#[derive(Debug, Clone)]
pub struct Monitor {
    pub flags: u32,           // MONITOR_PRIMARY = 0x01
    pub left: i32,            // Virtual desktop coordinates
    pub top: i32,
    pub width: u32,           // Monitor dimensions
    pub height: u32,
    pub physical_width: u32,  // Physical size in mm
    pub physical_height: u32,
    pub orientation: u32,     // 0, 90, 180, 270
    pub desktop_scale_factor: u32,  // 100 = 100%
    pub device_scale_factor: u32,   // 100 = 100%
}
```

### 4.3 RdpServerDisplayUpdates Trait

#### 4.3.1 Complete Trait Definition

```rust
#[async_trait]
pub trait RdpServerDisplayUpdates: Send {
    /// Get next display update
    /// MUST be cancellation-safe for use with tokio::select!
    /// Called on: RDP update thread
    /// Timing: Should return within 16ms (60fps)
    async fn next_update(&mut self) -> Result<Option<DisplayUpdate>>;

    /// Check if updates are available without blocking
    fn has_update(&self) -> bool {
        false  // Default: always poll
    }

    /// Reset update stream (after connection loss)
    fn reset(&mut self) {
        // Default: no-op
    }
}
```

#### 4.3.2 DisplayUpdate Variants

```rust
#[derive(Debug, Clone)]
pub enum DisplayUpdate {
    /// Desktop resize event
    Resize(DesktopSize),

    /// Bitmap update (primary update mechanism)
    Bitmap(BitmapUpdate),

    /// Cursor position update
    PointerPosition(PointerPositionAttribute),

    /// Color cursor update (legacy)
    ColorPointer(ColorPointer),

    /// RGBA cursor update (modern)
    RGBAPointer(RGBAPointer),

    /// Hide cursor
    HidePointer,

    /// Use default system cursor
    DefaultPointer,

    /// Surface commands (optional)
    Surface(SurfaceCommand),
}
```

#### 4.3.3 BitmapUpdate Structure

```rust
#[derive(Debug, Clone)]
pub struct BitmapUpdate {
    /// Update rectangles
    pub rectangles: Vec<BitmapData>,

    /// Compression hint
    pub compression_hint: CompressionHint,
}

#[derive(Debug, Clone)]
pub struct BitmapData {
    /// Destination rectangle
    pub dest_left: u16,
    pub dest_top: u16,
    pub dest_right: u16,   // Exclusive
    pub dest_bottom: u16,  // Exclusive

    /// Source dimensions
    pub width: u16,
    pub height: u16,

    /// Bits per pixel (15, 16, 24, 32)
    pub bits_per_pixel: u16,

    /// Compression flags
    pub flags: BitmapFlags,

    /// Compressed data
    pub bitmap_data: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct BitmapFlags {
    pub compression: bool,      // RDP 6.0 bitmap compression
    pub no_bitmap_header: bool,  // Skip bitmap header
}

#[derive(Debug, Clone, Copy)]
pub enum CompressionHint {
    None,
    Default,
    RDP6,     // RDP 6.0 bitmap compression
    RemoteFX,  // RemoteFX codec
}
```

**Bitmap Data Format**:
```
// 32-bit BGRA format (most common)
struct Pixel32 {
    blue: u8,
    green: u8,
    red: u8,
    alpha: u8,  // Usually 0xFF
}

// Data layout: Bottom-up, left-to-right
// Row 0 = Bottom row of image
// Stride = ALIGN(width * bytes_per_pixel, 4)
```

#### 4.3.4 Pointer Updates

```rust
#[derive(Debug, Clone)]
pub struct PointerPositionAttribute {
    pub position: Point,
}

#[derive(Debug, Clone)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone)]
pub struct RGBAPointer {
    pub hot_spot: Point,
    pub width: u16,       // 1-384
    pub height: u16,      // 1-384
    pub xor_mask: Vec<u8>,  // BGRA data
    pub and_mask: Vec<u8>,  // 1-bit mask (optional)
}

#[derive(Debug, Clone)]
pub struct ColorPointer {
    pub cache_index: u16,
    pub hot_spot: Point,
    pub width: u16,       // 1-32
    pub height: u16,      // 1-32
    pub and_mask: Vec<u8>,  // 1-bit AND mask
    pub xor_mask: Vec<u8>,  // Color XOR mask
}
```

### 4.4 Threading Requirements

#### 4.4.1 Thread Model

```
┌─────────────────────────────────────┐
│         RDP Main Thread             │
│  - Connection management             │
│  - Protocol negotiation              │
│  - size() calls                     │
│  - request_layout() calls           │
└──────────────┬──────────────────────┘
               │
      ┌────────┴────────┬──────────────┐
      │                 │              │
┌─────▼──────┐   ┌──────▼──────┐ ┌────▼─────┐
│Input Thread│   │Update Thread│ │I/O Thread│
│ keyboard() │   │next_update()│ │Send/Recv │
│  mouse()   │   │   60 Hz     │ │TCP/TLS   │
└────────────┘   └─────────────┘ └──────────┘
```

#### 4.4.2 Synchronization Requirements

```rust
// Input handler must be thread-safe
impl RdpServerInputHandler for MyHandler {
    fn keyboard(&mut self, event: KeyboardEvent) {
        // Use channels or Arc<Mutex<>> for state sharing
        // DO NOT block > 1ms
        self.input_tx.try_send(Input::Keyboard(event)).ok();
    }
}

// Display updates must be cancellation-safe
impl RdpServerDisplayUpdates for MyUpdates {
    async fn next_update(&mut self) -> Result<Option<DisplayUpdate>> {
        // MUST work correctly with tokio::select!
        tokio::select! {
            frame = self.frame_rx.recv() => {
                Ok(Some(DisplayUpdate::Bitmap(frame?)))
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                Ok(None)  // Timeout, no update
            }
        }
    }
}
```

### 4.5 Error Handling Requirements

#### 4.5.1 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum RdpServerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Display error: {0}")]
    Display(String),

    #[error("Input handler error: {0}")]
    InputHandler(String),

    #[error("Connection closed by client")]
    ConnectionClosed,

    #[error("Timeout")]
    Timeout,
}
```

#### 4.5.2 Error Recovery

```rust
// Display updates should handle errors gracefully
async fn next_update(&mut self) -> Result<Option<DisplayUpdate>> {
    match self.get_frame().await {
        Ok(frame) => Ok(Some(DisplayUpdate::Bitmap(frame))),
        Err(e) if e.is_temporary() => {
            // Log and continue
            warn!("Temporary frame error: {}", e);
            Ok(None)
        }
        Err(e) => {
            // Fatal error, propagate
            Err(e.into())
        }
    }
}
```

## 5. RDP Protocol Integration

### 5.1 Connection Sequence

```
Client              IronRDP            WRD-Server
  │                    │                    │
  ├──TCP Connect──────►│                    │
  │                    ├──Accept────────────►│
  │                    │                    │
  ├──X.224 CR──────────►│                    │
  │                    ├──Parse─────────────►│
  │◄──X.224 CC──────────┤◄──Response────────┤
  │                    │                    │
  ├──MCS Connect───────►│                    │
  │                    ├──Parse─────────────►│
  │◄──MCS Response──────┤◄──Build───────────┤
  │                    │                    │
  ├──Security Exchange─►│                    │
  │                    ├──TLS Handshake────►│
  │◄──Security OK───────┤◄──Certificate─────┤
  │                    │                    │
  ├──Client Info───────►│                    │
  │                    ├──Parse─────────────►│
  │                    │         ┌──────────▼──────────┐
  │                    │         │Create Input Handler│
  │                    │         │Create Display Handler
  │                    │         └──────────┬──────────┘
  │◄──Capabilities─────┤◄──Build────────────┤
  │                    │                    │
  ├──Confirm Active───►│                    │
  │                    ├──Parse─────────────►│
  │                    │         ┌──────────▼──────────┐
  │                    │         │Start Update Thread  │
  │                    │         │Start Input Thread   │
  │                    │         └──────────┬──────────┘
  │◄──Updates Begin────┤◄──DisplayUpdate────┤
  │                    │                    │
```

### 5.2 Capability Sets

#### 5.2.1 General Capability Set

```rust
pub struct GeneralCapabilitySet {
    pub os_major_type: u16,      // OSMAJORTYPE_UNIX = 0x04
    pub os_minor_type: u16,      // OSMINORTYPE_NATIVE_WAYLAND = 0x09
    pub protocol_version: u16,   // 0x0200 (RDP 5.0+)
    pub compression_types: u16,  // 0 (no compression)
    pub extra_flags: u16,        // FASTPATH_OUTPUT_SUPPORTED
    pub update_capability: u16,  // 0
    pub remote_unshare: u16,     // 0
    pub compression_level: u16,  // 0
    pub refresh_rect_support: u8, // TRUE
    pub suppress_output_support: u8, // TRUE
}
```

#### 5.2.2 Bitmap Capability Set

```rust
pub struct BitmapCapabilitySet {
    pub preferred_bits_per_pixel: u16,  // 32
    pub receive_1_bit_per_pixel: u16,   // FALSE
    pub receive_4_bits_per_pixel: u16,  // FALSE
    pub receive_8_bits_per_pixel: u16,  // FALSE
    pub desktop_width: u16,              // Actual width
    pub desktop_height: u16,             // Actual height
    pub desktop_resize_flag: u16,        // TRUE
    pub bitmap_compression_flag: u16,    // TRUE
    pub drawing_flags: u8,               // 0
}
```

#### 5.2.3 Input Capability Set

```rust
pub struct InputCapabilitySet {
    pub input_flags: u16,        // INPUT_FLAG_*
    pub keyboard_layout: u32,    // Keyboard layout ID
    pub keyboard_type: u32,      // IBM_101_102_KEYS
    pub keyboard_subtype: u32,   // 0
    pub keyboard_function_key: u32,  // 12 function keys
    pub ime_file_name: [u16; 32],   // Empty
}

const INPUT_FLAG_SCANCODES: u16 = 0x0001;
const INPUT_FLAG_MOUSEX: u16 = 0x0004;
const INPUT_FLAG_FASTPATH_INPUT: u16 = 0x0008;
const INPUT_FLAG_UNICODE: u16 = 0x0010;
const INPUT_FLAG_FASTPATH_INPUT2: u16 = 0x0020;
const INPUT_FLAG_MOUSE_HWHEEL: u16 = 0x0100;
const INPUT_FLAG_QOEI_TIMESTAMP: u16 = 0x0200;
```

### 5.3 FastPath Protocol

#### 5.3.1 FastPath Input Format

```
┌──────────┬──────────┬──────────────┬──────────┐
│  Header  │NumEvents │  Event Data  │   MAC    │
│  1 byte  │ 1 byte   │   Variable   │ Optional │
└──────────┴──────────┴──────────────┴──────────┘

Header Format:
┌───┬───┬───────────────┐
│ E │ S │   Action      │
│ 2 │ 1 │    5 bits     │
└───┴───┴───────────────┘

E: Encryption flags (00 = none)
S: Secure checksum (0 = none)
Action: 0x0 (input events)
```

#### 5.3.2 FastPath Input Events

```rust
pub enum FastPathInputEvent {
    // Type: 0x01 - Keyboard
    KeyboardEvent {
        flags: u8,    // KEYUP = 0x01
        code: u8,     // Scan code
    },

    // Type: 0x03 - Mouse
    MouseEvent {
        pointer_flags: u16,  // Button and action flags
        x_pos: u16,
        y_pos: u16,
    },

    // Type: 0x04 - Extended mouse
    ExtendedMouseEvent {
        pointer_flags: u16,
        x_pos: u16,
        y_pos: u16,
    },

    // Type: 0x05 - Unicode
    UnicodeKeyboardEvent {
        flags: u8,    // KEYUP = 0x01
        unicode: u16,
    },
}
```

#### 5.3.3 FastPath Output Format

```
┌──────────┬──────────────┬──────────────┬──────────┐
│  Header  │UpdateCode(s) │ Update Data  │   MAC    │
│  1 byte  │   Variable   │   Variable   │ Optional │
└──────────┴──────────────┴──────────────┴──────────┘

Update Codes:
0x00 - ORDERS
0x01 - BITMAP
0x02 - PALETTE
0x03 - SYNCHRONIZE
0x04 - SURFACE_COMMANDS
0x05 - POINTER_HIDDEN
0x06 - POINTER_DEFAULT
0x08 - POINTER_POSITION
0x09 - COLOR_POINTER
0x0A - CACHED_POINTER
0x0B - NEW_POINTER
```

### 5.4 RemoteFX Codec

#### 5.4.1 RemoteFX Tile Structure

```rust
pub struct RfxTile {
    pub block_type: u16,     // 0xCAC3 (CBT_TILE)
    pub block_len: u32,
    pub quantization_idx: u8,  // 0-15
    pub y_data: Vec<u8>,       // Y component
    pub cb_data: Vec<u8>,      // Cb component
    pub cr_data: Vec<u8>,      // Cr component
}

pub struct RfxRect {
    pub x: u16,       // Multiple of 64
    pub y: u16,       // Multiple of 64
    pub width: u16,   // Multiple of 64
    pub height: u16,  // Multiple of 64
}
```

#### 5.4.2 RemoteFX Encoding Process

```
RGB Input → YCbCr420 → DWT → Quantization → RLE → Output

1. Color conversion (RGB to YCbCr420)
2. Discrete Wavelet Transform (3 levels)
3. Quantization (reduce precision)
4. Run-Length Encoding
5. Package into tiles
```

### 5.5 Channel Protocol

#### 5.5.1 Static Virtual Channels

```rust
pub struct StaticChannel {
    pub name: [u8; 8],        // NULL-terminated, 7 chars max
    pub flags: u32,
    pub channel_id: u16,
}

// Common channels
const CLIPRDR_CHANNEL: &[u8; 8] = b"cliprdr\0";
const RDPSND_CHANNEL: &[u8; 8] = b"rdpsnd\0\0";
const RDPDR_CHANNEL: &[u8; 8] = b"rdpdr\0\0\0";
const DRDYNVC_CHANNEL: &[u8; 8] = b"drdynvc\0";
```

#### 5.5.2 Dynamic Virtual Channels

```rust
pub struct DvcCreateRequest {
    pub channel_id: u32,
    pub channel_name: String,
}

pub struct DvcDataPdu {
    pub channel_id: u32,
    pub data: Vec<u8>,
}

// DVC PDU types
const DVC_CREATE_REQUEST: u8 = 0x01;
const DVC_CREATE_RESPONSE: u8 = 0x02;
const DVC_CLOSE: u8 = 0x03;
const DVC_DATA_FIRST: u8 = 0x04;
const DVC_DATA: u8 = 0x05;
```

## 6. Protocol Interactions

### 6.1 Complete Session Flow

```
User                Client            IronRDP          WRD-Server         Portal          PipeWire
 │                    │                 │                  │                 │                │
 ├─Connect───────────►│                 │                  │                 │                │
 │                    ├─TCP/TLS────────►│                  │                 │                │
 │                    │                 ├─Accept──────────►│                 │                │
 │                    │                 │                  ├─CreateSession──►│                │
 │                    │                 │                  │◄─SessionHandle──┤                │
 │                    │                 │                  ├─SelectDevices──►│                │
 │                    │                 │                  │◄─DevicesOK──────┤                │
 │                    │                 │                  ├─Start───────────►│                │
 │◄─Permission Dialog─┼─────────────────┼──────────────────┼─────────────────┤                │
 ├─Grant──────────────►                 │                  │                 │                │
 │                    │                 │                  │◄─PW FD──────────┤                │
 │                    │                 │                  ├─Connect─────────────────────────►│
 │                    │                 │                  │◄─Stream Ready────────────────────┤
 │                    │◄─Connected──────┤◄─Ready───────────┤                 │                │
 │◄─Desktop───────────┤                 │                  │                 │                │
 │                    │                 │                  │◄─Frame──────────────────────────┤
 │                    │◄─Update─────────┤◄─Bitmap──────────┤                 │                │
 ├─Mouse──────────────►│                 │                  │                 │                │
 │                    ├─Input──────────►│                  │                 │                │
 │                    │                 ├─mouse()─────────►│                 │                │
 │                    │                 │                  ├─NotifyPointer──►│                │
 │                    │                 │                  │                 ├─Inject────────►│
 │                    │                 │                  │                 │                │
```

### 6.2 Frame Processing Pipeline

```
PipeWire          WRD-Server         Encoder          IronRDP          Client
   │                 │                  │                │               │
   ├─Buffer Ready───►│                  │                │               │
   │                 ├─Dequeue Buffer   │                │               │
   │◄────────────────┤                  │                │               │
   │                 ├─Map DMA-BUF      │                │               │
   │                 ├─Copy/Convert────►│                │               │
   │                 │                  ├─Encode         │               │
   │                 │◄─Bitmap Data─────┤                │               │
   │                 ├─Queue Buffer     │                │               │
   │◄────────────────┤                  │                │               │
   │                 ├─next_update()◄───┤                │               │
   │                 ├─DisplayUpdate────────────────────►│               │
   │                 │                  │                ├─Serialize     │
   │                 │                  │                ├─Send─────────►│
   │                 │                  │                │               │
```

**Timing**:
- Buffer dequeue: < 1ms
- DMA-BUF map: < 2ms
- Format conversion: < 5ms
- Encode (1080p): < 10ms
- Network send: < 1ms
- Total: < 20ms per frame

### 6.3 Input Injection Pipeline

```
Client            IronRDP          InputHandler        Portal           Compositor
   │                 │                  │                │                  │
   ├─Mouse Move─────►│                  │                │                  │
   │                 ├─Decode           │                │                  │
   │                 ├─mouse()─────────►│                │                  │
   │                 │                  ├─Scale coords   │                  │
   │                 │                  ├─NotifyMotion──►│                  │
   │                 │                  │                ├─D-Bus Call       │
   │                 │                  │                ├─InjectEvent─────►│
   │                 │                  │◄─Success───────┤                  │
   │                 │◄─────────────────┤                │                  │
   │                 │                  │                │                  │
```

**Latency Budget**:
- Network receive: 1-50ms (depends on connection)
- Decode: < 0.1ms
- Handler processing: < 1ms
- D-Bus call: < 2ms
- Compositor injection: < 1ms
- Total added latency: < 5ms

## 7. Error Handling

### 7.1 D-Bus Error Codes

```
org.freedesktop.portal.Error.Failed
  Generic failure, check logs for details

org.freedesktop.portal.Error.InvalidArgument
  Invalid parameter passed to method

org.freedesktop.portal.Error.NotAllowed
  Permission denied by policy or user

org.freedesktop.portal.Error.Cancelled
  Operation cancelled by user

org.freedesktop.portal.Error.WindowDestroyed
  Parent window no longer exists

org.freedesktop.DBus.Error.NoReply
  Method call timed out (default: 25s)

org.freedesktop.DBus.Error.ServiceUnknown
  Portal service not available

org.freedesktop.DBus.Error.AccessDenied
  D-Bus policy denies access
```

### 7.2 PipeWire Error Codes

```c
enum pw_error {
    PW_ERROR_INVALID = -EINVAL,      // Invalid argument
    PW_ERROR_NO_MEMORY = -ENOMEM,    // Out of memory
    PW_ERROR_NOT_IMPLEMENTED = -ENOSYS, // Not implemented
    PW_ERROR_NO_PERMISSION = -EPERM,    // Permission denied
    PW_ERROR_PROTOCOL = -EPROTO,        // Protocol error
    PW_ERROR_BUSY = -EBUSY,             // Resource busy
    PW_ERROR_TIMEOUT = -ETIMEDOUT,      // Operation timeout
    PW_ERROR_DISCONNECTED = -EPIPE,     // Connection lost
};
```

### 7.3 RDP Disconnect Reasons

```rust
pub enum DisconnectReason {
    // User-initiated (0x0000-0x0FFF)
    UserRequested = 0x0001,
    UserLogoff = 0x0002,

    // Protocol errors (0x1000-0x1FFF)
    ProtocolError = 0x1004,
    NotFound = 0x1005,
    InvalidRequest = 0x1006,

    // Licensing (0x2000-0x2FFF)
    LicenseError = 0x2001,
    LicenseTimeout = 0x2002,

    // Connection (0x3000-0x3FFF)
    NetworkError = 0x3001,
    ConnectionTimeout = 0x3002,
    SocketClosed = 0x3003,

    // Server errors (0x4000-0x4FFF)
    InternalError = 0x4001,
    OutOfMemory = 0x4002,
    ServerDenied = 0x4003,
}
```

### 7.4 Error Recovery Strategies

#### 7.4.1 Portal Session Recovery

```rust
async fn recover_portal_session(&mut self) -> Result<()> {
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 3;

    while retry_count < MAX_RETRIES {
        match self.create_session().await {
            Ok(session) => {
                self.session = Some(session);
                return Ok(());
            }
            Err(e) if e.is_temporary() => {
                retry_count += 1;
                tokio::time::sleep(Duration::from_secs(1 << retry_count)).await;
            }
            Err(e) => return Err(e),
        }
    }

    Err(anyhow!("Failed to recover portal session after {} retries", MAX_RETRIES))
}
```

#### 7.4.2 PipeWire Stream Recovery

```rust
async fn recover_stream(&mut self) -> Result<()> {
    // Disconnect existing stream
    if let Some(stream) = self.stream.take() {
        stream.disconnect()?;
    }

    // Wait for cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Reconnect with same node ID
    let stream = self.create_stream()?;
    stream.connect(
        Direction::Input,
        self.node_id,
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
        &params,
    )?;

    self.stream = Some(stream);
    Ok(())
}
```

## 8. Timing Requirements

### 8.1 Protocol Timeouts

| Operation | Timeout | Notes |
|-----------|---------|-------|
| D-Bus method call | 25s | Default D-Bus timeout |
| Portal user dialog | 60s | User interaction timeout |
| Portal session idle | 3600s | 1 hour idle timeout |
| PipeWire buffer dequeue | 100ms | Frame wait timeout |
| RDP connection | 30s | Initial connection |
| RDP keep-alive | 60s | No activity disconnect |
| RDP input processing | 1ms | Per event processing |
| RDP frame generation | 16.67ms | 60fps target |
| TLS handshake | 10s | Certificate validation |

### 8.2 Performance Requirements

#### 8.2.1 Latency Targets

```
Input Latency (mouse/keyboard):
  Network RTT:        1-100ms (variable)
  Processing:         < 5ms
  Total added:        < 10ms

Frame Latency (capture to display):
  Capture:            < 5ms
  Encode:             < 15ms
  Network:            1-100ms (variable)
  Decode (client):    < 10ms
  Total added:        < 30ms
```

#### 8.2.2 Throughput Targets

```
Resolution    FPS    Bitrate       Bandwidth
──────────────────────────────────────────────
1920x1080     60     20 Mbps       2.5 MB/s
1920x1080     30     10 Mbps       1.25 MB/s
1280x720      60     10 Mbps       1.25 MB/s
1280x720      30     5 Mbps        0.625 MB/s
```

### 8.3 Synchronization Points

```rust
// Critical sections that require synchronization
pub struct SyncPoints {
    // Portal session creation (mutex required)
    session_creation: Mutex<()>,

    // PipeWire buffer pool (atomic access)
    buffer_pool: AtomicPtr<BufferPool>,

    // Frame sequence number (atomic increment)
    frame_sequence: AtomicU64,

    // Input event queue (channel capacity: 1000)
    input_queue: mpsc::Sender<InputEvent>,

    // Display update queue (channel capacity: 10)
    update_queue: mpsc::Sender<DisplayUpdate>,
}
```

### 8.4 Quality of Service

#### 8.4.1 Adaptive Quality

```rust
pub struct QualityController {
    target_fps: u32,           // 15-60
    target_bitrate: u32,       // 1-50 Mbps
    compression_level: u32,    // 0-9
    color_depth: u32,          // 15, 16, 24, 32

    // Metrics
    actual_fps: f32,
    frame_drops: u32,
    network_rtt: Duration,
    bandwidth_estimate: u32,
}

impl QualityController {
    pub fn adapt(&mut self) {
        if self.frame_drops > 10 {
            self.target_fps = (self.target_fps - 5).max(15);
        }

        if self.network_rtt > Duration::from_millis(100) {
            self.compression_level = 9;
            self.color_depth = 16;
        }

        if self.bandwidth_estimate < 5_000_000 {
            self.target_bitrate = self.bandwidth_estimate * 80 / 100;
        }
    }
}
```

#### 8.4.2 Frame Skip Strategy

```rust
pub struct FrameSkipper {
    last_frame_time: Instant,
    min_frame_interval: Duration,
    skip_count: u32,
}

impl FrameSkipper {
    pub fn should_send_frame(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now - self.last_frame_time;

        if elapsed < self.min_frame_interval {
            self.skip_count += 1;
            false
        } else {
            self.last_frame_time = now;
            true
        }
    }
}
```

## 9. Protocol Validation

### 9.1 Input Validation

```rust
fn validate_mouse_input(x: u16, y: u16, desktop: &DesktopSize) -> Result<()> {
    // RDP uses 0-65535 coordinate space
    if x > 65535 || y > 65535 {
        return Err(anyhow!("Mouse coordinates out of range"));
    }

    // Scaled coordinates must be within desktop
    let scaled_x = (x as u32 * desktop.width as u32) / 65535;
    let scaled_y = (y as u32 * desktop.height as u32) / 65535;

    if scaled_x > desktop.width as u32 || scaled_y > desktop.height as u32 {
        warn!("Mouse coordinates outside desktop bounds");
    }

    Ok(())
}

fn validate_keyboard_input(code: u8, extended: bool) -> Result<()> {
    // Valid scan codes: 1-127 (0 reserved, 128-255 extended)
    if code == 0 || code > 127 {
        return Err(anyhow!("Invalid scan code: {}", code));
    }

    // Extended codes have E0 prefix
    if extended && code < 0x1C {
        return Err(anyhow!("Invalid extended scan code: {}", code));
    }

    Ok(())
}
```

### 9.2 Protocol Compliance

```rust
pub struct ProtocolValidator {
    strict_mode: bool,
    max_packet_size: usize,
    allowed_channels: Vec<String>,
}

impl ProtocolValidator {
    pub fn validate_packet(&self, packet: &[u8]) -> Result<()> {
        if packet.len() > self.max_packet_size {
            return Err(anyhow!("Packet exceeds maximum size"));
        }

        // Check packet header
        let header = packet.get(0).ok_or(anyhow!("Empty packet"))?;

        if self.strict_mode {
            // Validate against RDP specification
            self.validate_rdp_compliance(packet)?;
        }

        Ok(())
    }
}
```

## 10. Security Considerations

### 10.1 Portal Security

- Sessions require explicit user permission via dialog
- Sessions are tied to process PID
- Restore tokens are cryptographically signed
- D-Bus policies enforce access control

### 10.2 RDP Security

- TLS 1.2+ required for all connections
- NLA (Network Level Authentication) supported
- Certificate validation required
- Channel data encryption available

### 10.3 Input Injection Security

```rust
pub struct InputSanitizer {
    max_events_per_second: u32,
    last_event_time: Instant,
    event_count: u32,
}

impl InputSanitizer {
    pub fn sanitize(&mut self, event: InputEvent) -> Result<InputEvent> {
        // Rate limiting
        let now = Instant::now();
        if now.duration_since(self.last_event_time) >= Duration::from_secs(1) {
            self.event_count = 0;
            self.last_event_time = now;
        }

        self.event_count += 1;
        if self.event_count > self.max_events_per_second {
            return Err(anyhow!("Input rate limit exceeded"));
        }

        // Validate event
        match event {
            InputEvent::Keyboard(k) => validate_keyboard_input(k.code, k.extended)?,
            InputEvent::Mouse(m) => validate_mouse_input(m.x, m.y, &self.desktop_size)?,
        }

        Ok(event)
    }
}
```

---

## Appendix A: Protocol Constants

```rust
// D-Bus paths
pub const PORTAL_BUS: &str = "org.freedesktop.portal.Desktop";
pub const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
pub const SCREENCAST_INTERFACE: &str = "org.freedesktop.portal.ScreenCast";
pub const REMOTEDESKTOP_INTERFACE: &str = "org.freedesktop.portal.RemoteDesktop";

// PipeWire
pub const PW_VERSION: u32 = 3;
pub const PW_DEFAULT_REMOTE: &str = "pipewire-0";

// RDP
pub const RDP_DEFAULT_PORT: u16 = 3389;
pub const RDP_PROTOCOL_VERSION: u32 = 0x00080004;  // RDP 8.0

// Limits
pub const MAX_DESKTOP_WIDTH: u16 = 7680;   // 8K
pub const MAX_DESKTOP_HEIGHT: u16 = 4320;  // 8K
pub const MAX_MONITORS: u8 = 16;
pub const MAX_CHANNELS: u8 = 31;
```

## Appendix B: Example Implementation

```rust
// Complete minimal implementation example
use async_trait::async_trait;
use ironrdp_server::*;

struct MinimalInputHandler {
    portal: RemoteDesktopManager,
    session: PortalSessionHandle,
}

impl RdpServerInputHandler for MinimalInputHandler {
    fn mouse(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Move { x, y } => {
                let scaled_x = (x as f64 / 65535.0) * self.desktop_width as f64;
                let scaled_y = (y as f64 / 65535.0) * self.desktop_height as f64;

                tokio::spawn(async move {
                    self.portal.notify_pointer_motion_absolute(
                        &self.session,
                        self.stream_id,
                        scaled_x,
                        scaled_y
                    ).await.ok();
                });
            }
            _ => {}
        }
    }

    fn keyboard(&mut self, event: KeyboardEvent) {
        // Convert and inject
    }
}

struct MinimalDisplay {
    updates_rx: mpsc::Receiver<DisplayUpdate>,
}

#[async_trait]
impl RdpServerDisplay for MinimalDisplay {
    async fn size(&mut self) -> DesktopSize {
        DesktopSize { width: 1920, height: 1080 }
    }

    async fn updates(&mut self) -> Result<Box<dyn RdpServerDisplayUpdates>> {
        Ok(Box::new(MinimalUpdates {
            rx: self.updates_rx.clone()
        }))
    }
}

struct MinimalUpdates {
    rx: mpsc::Receiver<DisplayUpdate>,
}

#[async_trait]
impl RdpServerDisplayUpdates for MinimalUpdates {
    async fn next_update(&mut self) -> Result<Option<DisplayUpdate>> {
        Ok(self.rx.recv().await)
    }
}
```

---

*End of Protocol Specifications Document*

*This document represents the complete and authoritative protocol specifications for the wrd-server project. All implementations must conform to these specifications for correct operation.*