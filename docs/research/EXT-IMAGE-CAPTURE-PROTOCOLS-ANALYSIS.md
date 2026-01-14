# ext-image-copy-capture & ext-image-capture-source: Deep Analysis

## Document Purpose

This document provides a comprehensive analysis for implementing `ext-image-copy-capture-v1` and `ext-image-capture-source-v1` protocols in the context of:

1. **Your Smithay-based compositor** with modular client plugin architecture
2. **WASM/WGPU/WebGPU clients** as primary plugin consumers
3. **Contributing to Smithay upstream**

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Protocol Architecture](#protocol-architecture)
3. [Why These Protocols Matter for Your Use Case](#why-these-protocols-matter-for-your-use-case)
4. [Smithay Contribution Analysis](#smithay-contribution-analysis)
5. [Implementation Guide](#implementation-guide)
6. [Smithay Code Patterns Reference](#smithay-code-patterns-reference)
7. [Integration with WASM/WebGPU Clients](#integration-with-wasmwebgpu-clients)
8. [Implementation Roadmap](#implementation-roadmap)
9. [Risk Assessment](#risk-assessment)
10. [Appendices](#appendices)

---

## Executive Summary

### Key Findings

| Aspect | Status |
|--------|--------|
| Protocol definitions in wayland-protocols | Available since 0.32.4 (Sep 2024) |
| Smithay's wayland-protocols version | 0.32.9 (protocols available) |
| Smithay implementation status | Not implemented (blank in issue #781) |
| Existing infrastructure for capture | ExportMem trait fully implemented |
| Estimated implementation effort | 950-1500 LOC, 32-49 hours |
| Priority for your compositor | **HIGH** - enables client screen capture |
| Value as Smithay contribution | **HIGH** - commonly requested feature |

### Recommendation

**Implement these protocols** because:

1. **For your compositor**: Your WASM/WebGPU clients will need a standard way to capture screen content. These protocols provide exactly that.

2. **For Smithay contribution**: This is one of the most-requested missing features. Your implementation would benefit the entire ecosystem (Niri, Cosmic, MagmaWM, etc.).

3. **Strategic alignment**: Implementing upstream gives you maintenance support and keeps your compositor compatible with the broader ecosystem.

---

## Protocol Architecture

### Protocol Dependency Graph

```
┌─────────────────────────────────────────────────────────────────┐
│                    CLIENT APPLICATION                            │
│            (e.g., your WASM/WebGPU plugin client)               │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│              ext_image_copy_capture_manager_v1                   │
│  • create_session(source, options) → session                    │
│  • create_pointer_cursor_session(source, device) → cursor_sess  │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
        ┌─────────────────────────┼─────────────────────────┐
        │                         │                         │
        ▼                         ▼                         ▼
┌───────────────────┐  ┌────────────────────┐  ┌───────────────────────┐
│ ext_image_capture │  │ ext_image_copy_    │  │ ext_image_copy_       │
│ _source_v1        │  │ capture_session_v1 │  │ capture_cursor_       │
│                   │  │                    │  │ session_v1            │
│ (opaque handle)   │  │ Events:            │  │                       │
│                   │  │ • buffer_size      │  │ Events:               │
└───────────────────┘  │ • shm_format       │  │ • enter/leave         │
        ▲              │ • dmabuf_device    │  │ • position            │
        │              │ • dmabuf_format    │  │ • hotspot             │
        │              │ • done             │  └───────────────────────┘
        │              │ • stopped          │
        │              │                    │
        │              │ Requests:          │
        │              │ • create_frame     │
        │              └─────────┬──────────┘
        │                        │
        │                        ▼
        │              ┌────────────────────┐
        │              │ ext_image_copy_    │
        │              │ capture_frame_v1   │
        │              │                    │
        │              │ Requests:          │
        │              │ • attach_buffer    │
        │              │ • damage_buffer    │
        │              │ • capture          │
        │              │                    │
        │              │ Events:            │
        │              │ • transform        │
        │              │ • damage           │
        │              │ • presentation_time│
        │              │ • ready            │
        │              │ • failed           │
        │              └────────────────────┘
        │
┌───────┴───────────────────────────────────────────────────────┐
│                    SOURCE PROVIDERS                            │
├───────────────────────────────┬───────────────────────────────┤
│ ext_output_image_capture_     │ ext_foreign_toplevel_image_   │
│ source_manager_v1             │ capture_source_manager_v1     │
│                               │                               │
│ create_source(wl_output)      │ create_source(toplevel_handle)│
│ → ext_image_capture_source_v1 │ → ext_image_capture_source_v1 │
└───────────────────────────────┴───────────────────────────────┘
```

### Capture Workflow Sequence

```
Client                        Compositor                    GPU/Buffer
  │                               │                            │
  │ bind manager                  │                            │
  │──────────────────────────────>│                            │
  │                               │                            │
  │ create_source(output)         │                            │
  │──────────────────────────────>│                            │
  │                               │                            │
  │ create_session(source, opts)  │                            │
  │──────────────────────────────>│                            │
  │                               │                            │
  │          buffer_size(w, h)    │                            │
  │<──────────────────────────────│                            │
  │          shm_format(fmt)      │                            │
  │<──────────────────────────────│ (repeated for each format) │
  │          dmabuf_device(dev)   │                            │
  │<──────────────────────────────│                            │
  │          dmabuf_format(fmt, mods)                          │
  │<──────────────────────────────│ (repeated)                 │
  │          done()               │                            │
  │<──────────────────────────────│                            │
  │                               │                            │
  │ [Client allocates buffer matching constraints]             │
  │                               │                            │
  │ create_frame()                │                            │
  │──────────────────────────────>│                            │
  │                               │                            │
  │ attach_buffer(wl_buffer)      │                            │
  │──────────────────────────────>│                            │
  │                               │                            │
  │ damage_buffer(x, y, w, h)     │                            │
  │──────────────────────────────>│                            │
  │                               │                            │
  │ capture()                     │                            │
  │──────────────────────────────>│                            │
  │                               │ copy_framebuffer()         │
  │                               │───────────────────────────>│
  │                               │                            │
  │                               │      pixels                │
  │                               │<───────────────────────────│
  │                               │                            │
  │          transform(...)       │                            │
  │<──────────────────────────────│                            │
  │          damage(regions)      │                            │
  │<──────────────────────────────│                            │
  │          presentation_time(t) │                            │
  │<──────────────────────────────│                            │
  │          ready()              │                            │
  │<──────────────────────────────│                            │
  │                               │                            │
  │ [Client can now read buffer]  │                            │
```

---

## Why These Protocols Matter for Your Use Case

### Your Architecture: Smithay Compositor + Plugin Clients

```
┌────────────────────────────────────────────────────────────────────┐
│                    YOUR SMITHAY COMPOSITOR                          │
│                                                                    │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                     Plugin Architecture                       │  │
│  │                                                              │  │
│  │   ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │  │
│  │   │ WASM/WGPU   │  │ Native      │  │ Future              │ │  │
│  │   │ Client      │  │ Client      │  │ Clients             │ │  │
│  │   │             │  │             │  │                     │ │  │
│  │   │ WebGPU      │  │ Vulkan      │  │ ...                 │ │  │
│  │   │ rendering   │  │ rendering   │  │                     │ │  │
│  │   └─────────────┘  └─────────────┘  └─────────────────────┘ │  │
│  │          │                │                    │             │  │
│  │          └────────────────┼────────────────────┘             │  │
│  │                           │                                  │  │
│  │                           ▼                                  │  │
│  │            ┌─────────────────────────────┐                   │  │
│  │            │  ext-image-copy-capture     │                   │  │
│  │            │  (screen capture protocol)  │                   │  │
│  │            └─────────────────────────────┘                   │  │
│  │                           │                                  │  │
│  └───────────────────────────┼──────────────────────────────────┘  │
│                              │                                     │
│                              ▼                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │              Smithay Core (your compositor)                   │  │
│  │                                                              │  │
│  │  • CompositorState          • ExportMem (frame readback)     │  │
│  │  • DmabufState              • Output management              │  │
│  │  • ForeignToplevelList      • Renderer abstraction           │  │
│  │                                                              │  │
│  └──────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────┘
```

### Use Cases for Your Plugin Clients

| Client Type | Capture Use Case |
|-------------|-----------------|
| **WASM/WebGPU Client** | Screenshot features, picture-in-picture, screen recording |
| **Collaborative Plugin** | Screen sharing, remote assistance |
| **Debug/Dev Tools** | Visual debugging, frame analysis |
| **Accessibility Plugin** | Screen magnification, visual assistance |
| **Recording Plugin** | Game capture, tutorial creation |

### Why Not Just Use ExportMem Directly?

You could have your compositor grab frames directly via `ExportMem::copy_framebuffer()`. However:

1. **Client Autonomy**: Clients should be able to request capture without compositor cooperation
2. **Standard Protocol**: Using the standard protocol means any Wayland capture client works
3. **Permission Model**: Protocol-based capture allows permission controls
4. **Buffer Ownership**: Client allocates and owns the buffer (important for WASM/WebGPU)
5. **Ecosystem Compatibility**: OBS, grim, slurp, etc. will work with your compositor

---

## Smithay Contribution Analysis

### Current Ecosystem Status

| Compositor | ext-image-copy-capture Support |
|------------|-------------------------------|
| **Smithay** | Not implemented |
| **wlroots** | Implemented (wlr-screencopy, transitioning) |
| **Mutter** | Not yet (uses Portal internally) |
| **KWin** | Not yet |

### Smithay Maintainer Expectations

From analyzing CONTRIBUTING.md, issue discussions, and recent PRs:

**1. Scope Alignment** (from CONTRIBUTING.md):
> "Smithay attempts to be as generic and un-opinionated as possible"

`ext-image-copy-capture` fits because:
- Generic feature many compositors need
- Standard protocol (not compositor-specific)
- Builds on existing infrastructure (ExportMem, DmaBuf)

**2. Modularity Requirements**:
> "Functionalities should be split into independent modules"

The implementation should:
- Be optional via feature flag
- Not require other unrelated protocols
- Work with any renderer implementing `ExportMem`

**3. Communication Channels**:
- Primary: GitHub issues/PRs
- Real-time: Matrix #smithay:matrix.org (IRC bridged)
- Maintainers: @Drakulix (Victoria Brekenfeld), @PolyMeilex, @ids1024

### Contribution Process

```
1. Open Issue (Discussion)
   │
   │  "I'd like to implement ext-image-copy-capture-v1"
   │  Reference: Issue #781
   │
   ▼
2. Get Feedback
   │
   │  Maintainers may suggest:
   │  - Design considerations
   │  - Integration points
   │  - API preferences
   │
   ▼
3. Draft Implementation
   │
   │  Follow patterns from:
   │  - drm_syncobj (clean, modern)
   │  - foreign_toplevel_list (similar scope)
   │  - dmabuf (buffer handling reference)
   │
   ▼
4. Open PR (Draft first)
   │
   │  - Reference the issue
   │  - Include documentation
   │  - Add anvil integration
   │
   ▼
5. Review Iterations
   │
   │  Expect feedback on:
   │  - API design
   │  - Error handling
   │  - Documentation
   │
   ▼
6. Merge
```

---

## Implementation Guide

### File Structure

```
src/wayland/
├── image_capture_source/
│   └── mod.rs              (~250-350 lines)
│       • ImageCaptureSourceState
│       • OutputImageCaptureSourceManager
│       • ForeignToplevelImageCaptureSourceManager
│       • ImageCaptureSource handle
│       • delegate_image_capture_source! macro
│
├── image_copy_capture/
│   ├── mod.rs              (~600-800 lines)
│   │   • ImageCopyCaptureState
│   │   • ImageCopyCaptureHandler trait
│   │   • Session management
│   │   • Frame lifecycle
│   │   • delegate_image_copy_capture! macro
│   │
│   └── dispatch.rs         (~200-300 lines)
│       • Dispatch implementations
│       • Request handlers
│       • Buffer validation
│
└── mod.rs                  (add module exports)
```

### Core Data Structures

```rust
// ============================================================================
// image_capture_source/mod.rs
// ============================================================================

use std::sync::{Arc, Mutex, Weak};
use wayland_protocols::ext::image_capture_source::v1::server::{
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
    ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
};
use wayland_server::{backend::GlobalId, Client, DisplayHandle, Resource, Weak as WlWeak};

use crate::output::Output;
use crate::wayland::foreign_toplevel_list::ForeignToplevelHandle;

/// The type of capture source
#[derive(Debug, Clone)]
pub enum CaptureSourceType {
    /// Capture from a wl_output
    Output(Output),
    /// Capture from a foreign toplevel
    Toplevel(ForeignToplevelHandle),
}

/// Inner state for a capture source
#[derive(Debug)]
struct ImageCaptureSourceInner {
    source_type: CaptureSourceType,
    /// Protocol resources representing this source
    instances: Vec<WlWeak<ExtImageCaptureSourceV1>>,
    /// Whether the source is still valid
    valid: bool,
}

/// Handle to a capture source
#[derive(Debug, Clone)]
pub struct ImageCaptureSource {
    inner: Arc<Mutex<ImageCaptureSourceInner>>,
}

impl ImageCaptureSource {
    /// Create a new output capture source
    pub fn from_output(output: Output) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ImageCaptureSourceInner {
                source_type: CaptureSourceType::Output(output),
                instances: Vec::new(),
                valid: true,
            })),
        }
    }

    /// Create a new toplevel capture source
    pub fn from_toplevel(toplevel: ForeignToplevelHandle) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ImageCaptureSourceInner {
                source_type: CaptureSourceType::Toplevel(toplevel),
                instances: Vec::new(),
                valid: true,
            })),
        }
    }

    /// Get the source type
    pub fn source_type(&self) -> CaptureSourceType {
        self.inner.lock().unwrap().source_type.clone()
    }

    /// Check if the source is still valid
    pub fn is_valid(&self) -> bool {
        self.inner.lock().unwrap().valid
    }

    /// Invalidate this source (e.g., output disconnected)
    pub fn invalidate(&self) {
        self.inner.lock().unwrap().valid = false;
    }
}

/// State for image capture source protocol
#[derive(Debug)]
pub struct ImageCaptureSourceState {
    output_manager_global: GlobalId,
    toplevel_manager_global: Option<GlobalId>,
}

impl ImageCaptureSourceState {
    /// Create a new image capture source state
    ///
    /// The `toplevel_manager` parameter controls whether toplevel capture
    /// is supported. If `true`, requires `ForeignToplevelListState` to be
    /// initialized.
    pub fn new<D>(display: &DisplayHandle, enable_toplevel: bool) -> Self
    where
        D: ImageCaptureSourceHandler,
    {
        let output_manager_global = display
            .create_global::<D, ExtOutputImageCaptureSourceManagerV1, _>(1, ());

        let toplevel_manager_global = if enable_toplevel {
            Some(display
                .create_global::<D, ExtForeignToplevelImageCaptureSourceManagerV1, _>(1, ()))
        } else {
            None
        };

        Self {
            output_manager_global,
            toplevel_manager_global,
        }
    }
}

/// Handler trait for image capture source
pub trait ImageCaptureSourceHandler:
    GlobalDispatch<ExtOutputImageCaptureSourceManagerV1, ()>
    + Dispatch<ExtOutputImageCaptureSourceManagerV1, ()>
    + Dispatch<ExtImageCaptureSourceV1, ImageCaptureSource>
    + 'static
{
    /// Get the image capture source state
    fn image_capture_source_state(&mut self) -> &mut ImageCaptureSourceState;

    /// Called when an output capture source is created
    ///
    /// Return `None` to deny the capture request
    fn output_capture_source_created(
        &mut self,
        output: &Output,
        client: &Client,
    ) -> Option<ImageCaptureSource> {
        Some(ImageCaptureSource::from_output(output.clone()))
    }

    /// Called when a toplevel capture source is created
    ///
    /// Return `None` to deny the capture request
    fn toplevel_capture_source_created(
        &mut self,
        toplevel: &ForeignToplevelHandle,
        client: &Client,
    ) -> Option<ImageCaptureSource> {
        Some(ImageCaptureSource::from_toplevel(toplevel.clone()))
    }
}
```

```rust
// ============================================================================
// image_copy_capture/mod.rs
// ============================================================================

use std::sync::{Arc, Mutex};
use wayland_protocols::ext::image_copy_capture::v1::server::{
    ext_image_copy_capture_manager_v1::{self, ExtImageCopyCaptureManagerV1},
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
    ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
};
use wayland_server::{
    backend::GlobalId,
    protocol::wl_buffer::WlBuffer,
    Client, DisplayHandle, Resource,
};

use crate::backend::allocator::{Fourcc, Modifier};
use crate::backend::renderer::ExportMem;
use crate::utils::{Physical, Rectangle, Transform};
use crate::wayland::image_capture_source::ImageCaptureSource;

/// Options for capture session creation
#[derive(Debug, Clone, Default)]
pub struct CaptureSessionOptions {
    /// Whether to paint cursors into the captured image
    pub paint_cursors: bool,
}

/// Buffer constraints for capture
#[derive(Debug, Clone)]
pub struct CaptureBufferConstraints {
    /// Required buffer dimensions
    pub size: (u32, u32),
    /// Supported SHM formats
    pub shm_formats: Vec<Fourcc>,
    /// DRM device for dmabuf allocation (if supported)
    pub dmabuf_device: Option<std::os::fd::RawFd>,
    /// Supported dmabuf formats with modifiers
    pub dmabuf_formats: Vec<(Fourcc, Vec<Modifier>)>,
}

/// State of a capture session
#[derive(Debug)]
pub enum SessionState {
    /// Session is active and accepting frames
    Active,
    /// Session was stopped (source destroyed or error)
    Stopped,
}

/// Inner state for a capture session
#[derive(Debug)]
struct CaptureSessionInner {
    source: ImageCaptureSource,
    options: CaptureSessionOptions,
    constraints: CaptureBufferConstraints,
    state: SessionState,
    /// Currently active frame (only one at a time)
    active_frame: Option<ExtImageCopyCaptureFrameV1>,
}

/// Handle to a capture session
#[derive(Debug, Clone)]
pub struct CaptureSession {
    inner: Arc<Mutex<CaptureSessionInner>>,
}

/// State for a capture frame
#[derive(Debug)]
pub struct CaptureFrameState {
    session: CaptureSession,
    buffer: Option<WlBuffer>,
    damage: Vec<Rectangle<i32, Physical>>,
    captured: bool,
}

/// Capture frame result
#[derive(Debug)]
pub struct CaptureFrameResult {
    /// Applied buffer transform
    pub transform: Transform,
    /// Damaged regions
    pub damage: Vec<Rectangle<i32, Physical>>,
    /// Presentation timestamp (seconds high, seconds low, nanoseconds)
    pub presentation_time: Option<(u32, u32, u32)>,
}

/// Why a capture failed
#[derive(Debug, Clone, Copy)]
pub enum CaptureFailureReason {
    /// Unknown runtime error (retry permitted)
    Unknown,
    /// Buffer doesn't match constraints
    BufferConstraints,
    /// Session is no longer available
    Stopped,
}

/// State for image copy capture protocol
#[derive(Debug)]
pub struct ImageCopyCaptureState {
    global: GlobalId,
}

impl ImageCopyCaptureState {
    /// Create a new image copy capture state
    pub fn new<D>(display: &DisplayHandle) -> Self
    where
        D: ImageCopyCaptureHandler,
    {
        let global = display
            .create_global::<D, ExtImageCopyCaptureManagerV1, _>(1, ());

        Self { global }
    }

    /// Get the global id
    pub fn global(&self) -> GlobalId {
        self.global.clone()
    }
}

/// Handler trait for image copy capture
pub trait ImageCopyCaptureHandler:
    ImageCaptureSourceHandler
    + GlobalDispatch<ExtImageCopyCaptureManagerV1, ()>
    + Dispatch<ExtImageCopyCaptureManagerV1, ()>
    + Dispatch<ExtImageCopyCaptureSessionV1, CaptureSession>
    + Dispatch<ExtImageCopyCaptureFrameV1, CaptureFrameState>
    + 'static
{
    /// Get the image copy capture state
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState;

    /// Get buffer constraints for a capture source
    ///
    /// Called when a new session is created to determine what buffer
    /// formats and sizes are acceptable.
    fn get_capture_constraints(
        &mut self,
        source: &ImageCaptureSource,
    ) -> CaptureBufferConstraints;

    /// Perform a capture operation
    ///
    /// Called when a client requests capture. The compositor should:
    /// 1. Render the source to the provided buffer
    /// 2. Return the result metadata
    ///
    /// Return `Err(reason)` if capture failed.
    fn capture_frame(
        &mut self,
        session: &CaptureSession,
        buffer: &WlBuffer,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<CaptureFrameResult, CaptureFailureReason>;
}
```

### Handler Implementation Pattern

```rust
// ============================================================================
// Example: Implementing in your compositor
// ============================================================================

use smithay::{
    delegate_image_capture_source, delegate_image_copy_capture,
    wayland::{
        image_capture_source::{
            ImageCaptureSource, ImageCaptureSourceHandler, ImageCaptureSourceState,
        },
        image_copy_capture::{
            CaptureBufferConstraints, CaptureFailureReason, CaptureFrameResult,
            CaptureSession, ImageCopyCaptureHandler, ImageCopyCaptureState,
        },
    },
};

pub struct MyCompositorState {
    // ... other state ...
    image_capture_source_state: ImageCaptureSourceState,
    image_copy_capture_state: ImageCopyCaptureState,
}

impl ImageCaptureSourceHandler for MyCompositorState {
    fn image_capture_source_state(&mut self) -> &mut ImageCaptureSourceState {
        &mut self.image_capture_source_state
    }

    fn output_capture_source_created(
        &mut self,
        output: &Output,
        client: &Client,
    ) -> Option<ImageCaptureSource> {
        // Optional: Check permissions
        // if !self.can_client_capture(client) {
        //     return None;
        // }

        Some(ImageCaptureSource::from_output(output.clone()))
    }
}

impl ImageCopyCaptureHandler for MyCompositorState {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState {
        &mut self.image_copy_capture_state
    }

    fn get_capture_constraints(
        &mut self,
        source: &ImageCaptureSource,
    ) -> CaptureBufferConstraints {
        match source.source_type() {
            CaptureSourceType::Output(output) => {
                let mode = output.current_mode().unwrap();
                let renderer = self.get_renderer_for_output(&output);

                CaptureBufferConstraints {
                    size: (mode.size.w as u32, mode.size.h as u32),
                    shm_formats: vec![Fourcc::Argb8888, Fourcc::Xrgb8888],
                    dmabuf_device: self.drm_device_fd(),
                    dmabuf_formats: renderer.dmabuf_formats().collect(),
                }
            }
            CaptureSourceType::Toplevel(toplevel) => {
                // Get toplevel geometry
                let geometry = self.get_toplevel_geometry(&toplevel);

                CaptureBufferConstraints {
                    size: (geometry.size.w as u32, geometry.size.h as u32),
                    shm_formats: vec![Fourcc::Argb8888],
                    dmabuf_device: None,
                    dmabuf_formats: vec![],
                }
            }
        }
    }

    fn capture_frame(
        &mut self,
        session: &CaptureSession,
        buffer: &WlBuffer,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<CaptureFrameResult, CaptureFailureReason> {
        let source = session.source();

        match source.source_type() {
            CaptureSourceType::Output(output) => {
                self.capture_output_to_buffer(&output, buffer, damage)
            }
            CaptureSourceType::Toplevel(toplevel) => {
                self.capture_toplevel_to_buffer(&toplevel, buffer, damage)
            }
        }
    }
}

delegate_image_capture_source!(MyCompositorState);
delegate_image_copy_capture!(MyCompositorState);

// Helper method using ExportMem
impl MyCompositorState {
    fn capture_output_to_buffer(
        &mut self,
        output: &Output,
        buffer: &WlBuffer,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<CaptureFrameResult, CaptureFailureReason> {
        let renderer = self.get_renderer_for_output(output);

        // Get the framebuffer for this output
        let framebuffer = self.get_output_framebuffer(output);

        // Use ExportMem to copy framebuffer contents
        let format = Fourcc::Argb8888; // Or determine from buffer
        let region = Rectangle::from_size(output.current_mode().unwrap().size);

        let mapping = renderer
            .copy_framebuffer(&framebuffer, region, format)
            .map_err(|_| CaptureFailureReason::Unknown)?;

        let pixels = renderer
            .map_texture(&mapping)
            .map_err(|_| CaptureFailureReason::Unknown)?;

        // Write to the client's buffer
        self.write_to_wl_buffer(buffer, pixels)?;

        Ok(CaptureFrameResult {
            transform: output.current_transform(),
            damage: damage.to_vec(),
            presentation_time: Some(self.get_presentation_time()),
        })
    }
}
```

---

## Smithay Code Patterns Reference

### Pattern 1: Handler Trait with State Accessor

Every protocol follows this pattern:

```rust
pub trait ProtocolHandler: /* required trait bounds */ {
    fn protocol_state(&mut self) -> &mut ProtocolState;

    // Optional callback methods with default implementations
    fn on_event(&mut self, /* params */) {
        // Default: do nothing
    }
}
```

### Pattern 2: Delegate Macro

```rust
#[macro_export]
macro_rules! delegate_protocol {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        const _: () = {
            use $crate::reexports::{
                wayland_protocols::...,
                wayland_server::{delegate_dispatch, delegate_global_dispatch},
            };
            use $crate::wayland::protocol::{...};

            delegate_global_dispatch!(
                $(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
                $ty: [ProtocolGlobal: GlobalData] => ProtocolState
            );
            delegate_dispatch!(
                $(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)?
                $ty: [ProtocolResource: ResourceData] => ProtocolState
            );
        };
    };
}
```

### Pattern 3: GlobalDispatch + Dispatch

```rust
impl<D: ProtocolHandler> GlobalDispatch<Global, GlobalData, D> for ProtocolState {
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<Global>,
        _global_data: &GlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ()); // or ResourceData
    }
}

impl<D: ProtocolHandler> Dispatch<Resource, ResourceData, D> for ProtocolState {
    fn request(
        state: &mut D,
        client: &Client,
        resource: &Resource,
        request: protocol::Request,
        data: &ResourceData,
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            Request::DoSomething { param } => {
                // Handle request
            }
            Request::Destroy => {
                // Cleanup
            }
            _ => unreachable!(),
        }
    }
}
```

### Pattern 4: Handle Pattern (Arc<Mutex<Inner>>)

Used when multiple protocol resources share state:

```rust
#[derive(Debug)]
struct HandleInner {
    data: SomeData,
    instances: Vec<Weak<ProtocolResource>>,
}

#[derive(Debug, Clone)]
pub struct Handle {
    inner: Arc<Mutex<HandleInner>>,
}

impl Handle {
    pub fn downgrade(&self) -> WeakHandle {
        WeakHandle { inner: Arc::downgrade(&self.inner) }
    }
}
```

### Pattern 5: Cacheable State (Double-Buffered)

For surface-attached protocol state:

```rust
#[derive(Debug, Default)]
pub struct CachedProtocolState {
    pub pending_value: Option<Value>,
}

impl Cacheable for CachedProtocolState {
    fn commit(&mut self, _dh: &DisplayHandle) -> Self {
        Self {
            pending_value: self.pending_value.take(),
        }
    }

    fn merge_into(self, into: &mut Self, _dh: &DisplayHandle) {
        if self.pending_value.is_some() {
            into.pending_value = self.pending_value;
        }
    }
}
```

---

## Integration with WASM/WebGPU Clients

### Architecture for WASM Clients

Your WASM/WebGPU clients need capture capabilities. Here's how they'd use the protocol:

```
┌─────────────────────────────────────────────────────────────────┐
│                      WASM/WGPU Client                           │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   WebGPU/WGPU Layer                        │  │
│  │                                                           │  │
│  │  • GPU textures for rendering                             │  │
│  │  • Can receive captured frames as textures                │  │
│  │                                                           │  │
│  └────────────────────────────┬──────────────────────────────┘  │
│                               │                                  │
│  ┌────────────────────────────▼──────────────────────────────┐  │
│  │                   Wayland Client Layer                     │  │
│  │                                                           │  │
│  │  • ext-image-copy-capture client implementation           │  │
│  │  • Allocates wl_buffer (shm or dmabuf)                   │  │
│  │  • Receives captured frames                               │  │
│  │                                                           │  │
│  └────────────────────────────┬──────────────────────────────┘  │
│                               │                                  │
└───────────────────────────────┼─────────────────────────────────┘
                                │ Wayland protocol
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Your Smithay Compositor                        │
│                                                                 │
│  • ext-image-capture-source (source management)                 │
│  • ext-image-copy-capture (frame capture)                       │
│  • ExportMem (GPU readback)                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### WASM Client Capture Flow

```rust
// Pseudocode for WASM/WebGPU client using ext-image-copy-capture

// 1. Get capture source for output
let source_manager = registry.bind::<ExtOutputImageCaptureSourceManagerV1>();
let source = source_manager.create_source(output);

// 2. Create capture session
let capture_manager = registry.bind::<ExtImageCopyCaptureManagerV1>();
let session = capture_manager.create_session(source, CaptureOptions::default());

// 3. Wait for buffer constraints
session.on_buffer_size(|w, h| {
    buffer_size = (w, h);
});
session.on_dmabuf_format(|format, modifiers| {
    supported_formats.push((format, modifiers));
});
session.on_done(|| {
    // Constraints received, allocate buffer
});

// 4. Allocate buffer matching constraints
let buffer = allocate_dmabuf_buffer(buffer_size, best_format);

// 5. Create frame and capture
let frame = session.create_frame();
frame.attach_buffer(buffer);
frame.damage_buffer(0, 0, buffer_size.0, buffer_size.1);
frame.capture();

// 6. Wait for result
frame.on_ready(|| {
    // Buffer now contains captured pixels
    // Import into WebGPU texture
    let texture = wgpu_device.create_texture_from_dmabuf(buffer);
});
```

### Zero-Copy Path (DMA-BUF)

For optimal performance, WASM clients using WebGPU should:

1. Allocate DMA-BUF backed wl_buffer
2. Import the same DMA-BUF into WebGPU
3. Compositor writes directly to the DMA-BUF
4. Client uses the texture without CPU copy

```
                    Shared DMA-BUF
                         │
         ┌───────────────┼───────────────┐
         │               │               │
         ▼               │               ▼
    Compositor           │          WASM Client
    (GPU write)          │          (GPU read)
         │               │               │
         ▼               │               ▼
    ExportMem::          │          wgpu::Texture
    copy_framebuffer()   │          (imported)
```

---

## Implementation Roadmap

### Phase 1: ext-image-capture-source-v1 (Week 1)

**Goal**: Implement source providers

| Task | Effort | Priority |
|------|--------|----------|
| Define ImageCaptureSource handle | 2h | P0 |
| Implement OutputImageCaptureSourceManager | 4h | P0 |
| Implement ForeignToplevelImageCaptureSourceManager | 4h | P1 |
| Add delegate macro | 2h | P0 |
| Documentation | 2h | P0 |
| Unit tests | 2h | P1 |

**Deliverable**: Working source creation, ready for copy-capture

### Phase 2: ext-image-copy-capture-v1 Core (Week 2)

**Goal**: Implement session and frame management

| Task | Effort | Priority |
|------|--------|----------|
| Define state structures | 3h | P0 |
| Implement Manager dispatch | 3h | P0 |
| Implement Session dispatch | 6h | P0 |
| Implement Frame dispatch | 6h | P0 |
| Buffer constraint negotiation | 4h | P0 |
| Add delegate macro | 2h | P0 |

**Deliverable**: Protocol skeleton, constraints work, capture stubbed

### Phase 3: Capture Implementation (Week 3)

**Goal**: Integrate with ExportMem for actual capture

| Task | Effort | Priority |
|------|--------|----------|
| Output capture via ExportMem | 6h | P0 |
| Toplevel capture | 6h | P1 |
| SHM buffer writing | 3h | P0 |
| DMA-BUF buffer writing | 4h | P0 |
| Damage region handling | 3h | P1 |
| Transform handling | 2h | P1 |

**Deliverable**: Working capture for outputs

### Phase 4: Cursor Session (Week 4)

**Goal**: Implement cursor capture extension

| Task | Effort | Priority |
|------|--------|----------|
| CursorSession state | 3h | P2 |
| Cursor position tracking | 4h | P2 |
| Hotspot handling | 2h | P2 |
| Enter/leave events | 2h | P2 |

**Deliverable**: Complete cursor capture support

### Phase 5: Integration & Polish (Week 5)

**Goal**: Anvil integration, testing, documentation

| Task | Effort | Priority |
|------|--------|----------|
| Anvil integration | 4h | P0 |
| Error handling review | 3h | P0 |
| Documentation polish | 4h | P0 |
| Test with grim/slurp | 4h | P1 |
| Performance testing | 3h | P1 |
| PR preparation | 2h | P0 |

**Deliverable**: PR-ready implementation

### Timeline Summary

```
Week 1: Source protocol        ████████░░░░░░░░  ~16h
Week 2: Copy-capture core      ████████████████  ~24h
Week 3: Capture implementation ████████████████  ~24h
Week 4: Cursor session         ████████░░░░░░░░  ~11h (optional)
Week 5: Integration & polish   ████████████████  ~20h
                               ─────────────────
                               Total: ~95h (with cursor)
                               Total: ~84h (without cursor)
```

---

## Risk Assessment

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| ExportMem performance issues | Medium | High | Profile early, consider GPU-to-GPU path |
| Buffer format incompatibility | Medium | Medium | Support common formats first (ARGB8888) |
| Multi-GPU complexity | High | Medium | Test with MultiRenderer, document limitations |
| Toplevel capture complexity | Medium | Medium | Implement output-only first |

### Process Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| API design disagreements | Medium | Medium | Open issue first, discuss before coding |
| Review delays | Medium | Low | Draft PR early, iterate |
| Protocol changes upstream | Low | High | Monitor wayland-protocols |

### Dependency Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| wayland-protocols version | Low | Low | Already 0.32.9, protocols available |
| Smithay breaking changes | Medium | Medium | Target specific Smithay version |

---

## Appendices

### A. Protocol XML Location

The protocol definitions are in wayland-protocols:
- `staging/ext-image-capture-source/ext-image-capture-source-v1.xml`
- `staging/ext-image-copy-capture/ext-image-copy-capture-v1.xml`

Available in wayland-protocols crate 0.32.4+

### B. Related Smithay Code References

| File | Purpose | Relevance |
|------|---------|-----------|
| `src/wayland/dmabuf/mod.rs` | Buffer import/feedback | Buffer handling patterns |
| `src/wayland/drm_syncobj/mod.rs` | Modern protocol example | Clean pattern to follow |
| `src/wayland/foreign_toplevel_list/mod.rs` | Handle pattern | Source handle reference |
| `src/backend/renderer/mod.rs` | ExportMem trait | Capture implementation |
| `src/backend/renderer/gles/mod.rs` | GLES ExportMem impl | Reference for capture |

### C. External References

| Resource | URL |
|----------|-----|
| Smithay GitHub | https://github.com/Smithay/smithay |
| Smithay Matrix | https://matrix.to/#/#smithay:matrix.org |
| Protocol Spec (source) | https://gitlab.freedesktop.org/wayland/wayland-protocols/-/tree/main/staging/ext-image-capture-source |
| Protocol Spec (copy-capture) | https://gitlab.freedesktop.org/wayland/wayland-protocols/-/tree/main/staging/ext-image-copy-capture |
| Issue #781 (protocol tracking) | https://github.com/Smithay/smithay/issues/781 |

### D. Testing Clients

For testing your implementation:

| Client | Purpose |
|--------|---------|
| `grim` | Screenshot tool (if updated for new protocol) |
| `slurp` | Region selection |
| `wf-recorder` | Screen recording |
| OBS | Streaming/recording |

### E. Glossary

| Term | Definition |
|------|------------|
| **ExportMem** | Smithay trait for GPU framebuffer readback |
| **DMA-BUF** | Linux buffer sharing mechanism |
| **SHM** | Shared memory buffers (CPU accessible) |
| **Cacheable** | Smithay trait for double-buffered surface state |
| **GlobalDispatch** | wayland-server trait for handling global binds |
| **Dispatch** | wayland-server trait for handling resource requests |

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-01-08 | Claude | Initial comprehensive analysis |

---

## Next Steps

1. **Read this document thoroughly** - understand the protocol and patterns
2. **Open a Smithay issue** - signal intent, get early feedback
3. **Start with Phase 1** - ext-image-capture-source is simpler, good warmup
4. **Draft PR early** - even skeleton code gets feedback faster
5. **Test with your compositor** - validate design before upstream PR
