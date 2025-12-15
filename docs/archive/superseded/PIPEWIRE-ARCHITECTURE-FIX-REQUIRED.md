# PipeWire Architecture Fix Required

**Date:** 2025-11-18
**Priority:** CRITICAL
**Status:** Identified architectural issue requiring fix

---

## Problem

The current PipeWire implementation has **fundamental thread safety issues**:

1. **PipeWire types are NOT Send:**
   - `MainLoop` contains `Rc<MainLoopInner>`
   - `Context` contains `Rc<ContextInner>`
   - `Core` contains `Rc<CoreInner>`
   - `Stream` contains `NonNull<pw_stream>`

2. **Current architecture tries to wrap them in Arc<Mutex>:**
   - This VIOLATES Rust's safety guarantees
   - Causes compilation errors
   - Cannot be sent between threads

3. **Existing code has "simplified" comments:**
   - Line 238 in stream.rs
   - Previously line 119-120 in connection.rs (now fixed)
   - These indicate incomplete implementation

---

## Root Cause

The CCW session that implemented P1-04 (commit 2645e5b) claimed "complete PipeWire integration" but actually delivered:
- ✅ 3,500 LOC of code
- ❌ Stub implementations with "simplified" comments
- ❌ Architecture that violates thread safety
- ❌ Cannot actually compile when integrated with server

**This violates the "NO simplified, NO stubs" requirement.**

---

## Required Architecture (Production)

### Correct Thread Model

```
┌─────────────────────────────────────────┐
│        Tokio Async Runtime              │
│                                         │
│  ┌──────────────────────────────────┐  │
│  │    WrdServer (async)             │  │
│  │                                  │  │
│  │  ┌────────────────────────────┐ │  │
│  │  │  WrdDisplayHandler         │ │  │
│  │  │                            │ │  │
│  │  │  Commands  ┌──────────────┐│ │  │
│  │  │    │       │  Command TX  ││ │  │
│  │  │    └──────>│  (channel)   ││ │  │
│  │  │            └──────┬───────┘│ │  │
│  │  │                   │        │ │  │
│  │  │  Frames    ┌──────▼───────┐│ │  │
│  │  │    ▲       │  Frame RX    ││ │  │
│  │  │    └───────│  (channel)   ││ │  │
│  │  │            └──────────────┘│ │  │
│  │  └────────────────────────────┘ │  │
│  └──────────────────────────────────┘  │
└──────────────────┬──────────────────────┘
                   │ Commands (CreateStream, etc.)
                   ▼
┌──────────────────────────────────────────┐
│   Dedicated PipeWire Thread              │
│   (std::thread::spawn)                   │
│                                          │
│  ┌────────────────────────────────────┐ │
│  │  MainLoop (Rc - not Send)          │ │
│  │    │                               │ │
│  │    ▼                               │ │
│  │  Context (Rc - not Send)           │ │
│  │    │                               │ │
│  │    ▼                               │ │
│  │  Core (Rc - not Send)              │ │
│  │    │                               │ │
│  │    ▼                               │ │
│  │  Streams (NonNull - not Send)      │ │
│  │    │                               │ │
│  │    └──> Frames sent via channel    │ │
│  └────────────────────────────────────┘ │
└──────────────────────────────────────────┘
```

### Required Components

1. **PipeWireThread** (new module)
   - Owns MainLoop, Context, Core, Streams
   - Runs on dedicated thread
   - Receives commands via channel
   - Sends frames via channel
   - Handles all PipeWire API calls

2. **PipeWireConnection** (refactor)
   - NO longer stores MainLoop/Context/Core directly
   - Stores command channel sender
   - Stores frame channel receiver
   - Provides async API by sending commands to thread

3. **PipeWireCommand** (already created in thread_comm.rs)
   - CreateStream
   - DestroyStream
   - StartStream
   - StopStream
   - Shutdown

4. **PipeWireStream** (refactor)
   - Remove async methods that try to call PipeWire APIs
   - Become a data structure only
   - Actual stream lives on PipeWire thread

---

## Implementation Plan

### Phase 1: Create PipeWire Thread Module (2-3 hours)

**File:** `src/pipewire/pw_thread.rs`

```rust
use std::collections::HashMap;
use std::thread;
use pipewire::{MainLoop, Context, Core};
use pipewire::stream::Stream;
use tokio::sync::mpsc;

/// PipeWire thread manager
///
/// Runs PipeWire MainLoop on dedicated thread and handles all PipeWire API calls.
pub struct PipeWireThreadManager {
    /// Thread handle
    thread_handle: Option<thread::JoinHandle<()>>,

    /// Command sender
    command_tx: mpsc::Sender<PipeWireCommand>,

    /// Frame receiver
    frame_rx: mpsc::Receiver<VideoFrame>,
}

impl PipeWireThreadManager {
    pub fn new(fd: RawFd) -> Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel(100);
        let (frame_tx, frame_rx) = mpsc::channel(64);

        let handle = thread::spawn(move || {
            run_pipewire_thread(fd, cmd_rx, frame_tx)
        });

        Ok(Self {
            thread_handle: Some(handle),
            command_tx: cmd_tx,
            frame_rx,
        })
    }
}

fn run_pipewire_thread(
    fd: RawFd,
    mut cmd_rx: mpsc::Receiver<PipeWireCommand>,
    frame_tx: mpsc::Sender<VideoFrame>,
) {
    // ALL PipeWire types live here on this thread
    pipewire::init();

    let main_loop = MainLoop::new(None).expect("MainLoop creation");
    let context = Context::new(&main_loop).expect("Context creation");
    let core = context.connect_fd(fd, None).expect("Core connection");

    let mut streams: HashMap<u32, Stream> = HashMap::new();

    // Event loop
    loop {
        // Process commands
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                PipeWireCommand::CreateStream { stream_id, config, node_id, response } => {
                    // CREATE STREAM HERE using core
                    // Set up callbacks to send frames via frame_tx
                    // Send response
                }
                PipeWireCommand::Shutdown => return,
                // ... handle other commands
            }
        }

        // Run main loop iteration
        let loop_ref = main_loop.loop_();
        loop_ref.iterate(Duration::from_millis(10));
    }

    pipewire::deinit();
}
```

### Phase 2: Refactor PipeWireConnection (1-2 hours)

Remove direct PipeWire type storage, use PipeWireThreadManager instead.

### Phase 3: Update Coordinator (1 hour)

Adjust to work with new async command-based API.

### Phase 4: Testing (2-3 hours)

Full integration tests with real PipeWire daemon.

---

## Total Estimate

**8-11 hours** of focused implementation to make PipeWire integration production-ready.

---

## Current Status

- ✅ Identified the issue
- ✅ Designed correct architecture
- ✅ Created thread_comm.rs skeleton
- ⏳ Need to implement pw_thread.rs
- ⏳ Need to refactor connection.rs
- ⏳ Need to update coordinator.rs
- ⏳ Need to test end-to-end

---

## Decision Point

Given:
1. We have 747K context remaining
2. This is 8-11 hours of work
3. This is CRITICAL for production

**RECOMMENDATION:** Implement the full solution NOW while we have context.

Alternative: Document this as blocker and handle in focused session.

---

**This MUST be fixed before claiming P1-04 complete.**

