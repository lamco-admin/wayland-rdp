# Full Multiplexer Implementation Plan
## All 4 Queues with Priority-Based Routing
## Date: 2025-12-10

---

## CURRENT STATUS

**Phase 1:** Graphics queue ✅ IMPLEMENTED
**Phase 2-4:** Input/Control/Clipboard queues ⏳ IN PROGRESS

---

## IRONRDP EVENT ARCHITECTURE ANALYSIS

### How IronRDP Handles Events

**1. Input Events (Keyboard/Mouse):**
```rust
// IronRDP calls our handler directly (synchronous callbacks):
impl RdpServerInputHandler for WrdInputHandler {
    fn keyboard(&mut self, event: KeyboardEvent) {
        // Called directly by IronRDP's protocol decoder
        // No channel involved
    }

    fn mouse(&mut self, event: MouseEvent) {
        // Called directly by IronRDP's protocol decoder
        // No channel involved
    }
}
```

**2. Clipboard/Sound Events:**
```rust
// IronRDP uses unbounded ServerEvent channel:
pub enum ServerEvent {
    Quit(String),
    Clipboard(ClipboardMessage),
    Rdpsnd(RdpsndServerMessage),
    SetCredentials(Credentials),
    GetLocalAddr(oneshot::Sender<Option<SocketAddr>>),
}

// Sent via: ev_sender.send(ServerEvent::Clipboard(...))
```

**3. Graphics Events:**
```rust
// IronRDP calls display.updates() once to get stream
// We send via: update_sender.send(DisplayUpdate::Bitmap(...))
```

---

## FULL MULTIPLEXER DESIGN

### Architecture

```text
Input Events (from IronRDP callbacks)
        ↓
    Input Queue (32) ──────────┐
                               │
Clipboard Events (from IronRDP ServerEvent)           Priority
        ↓                      │           Multiplexer Drain
    Clipboard Queue (8) ───────┤                ↓
                               ├────────> TCP Socket
Control Events (Quit, etc.)    │
        ↓                      │
    Control Queue (16) ────────┤
                               │
Graphics Events (from our pipeline)
        ↓                      │
    Graphics Queue (4) ────────┘
```

### Integration Points

**1. Modify WrdInputHandler:**
```rust
impl RdpServerInputHandler for WrdInputHandler {
    fn keyboard(&mut self, event: KeyboardEvent) {
        // Route through multiplexer input queue instead of direct Portal call
        let mux_event = MultiplexerInputEvent::Keyboard(event);
        self.input_tx.try_send(mux_event); // Non-blocking
    }

    fn mouse(&mut self, event: MouseEvent) {
        let mux_event = MultiplexerInputEvent::Mouse(event);
        self.input_tx.try_send(mux_event);
    }
}
```

**2. Intercept ServerEvent in our server:**
```rust
// Instead of letting IronRDP handle ServerEvent directly,
// we need to intercept and route through multiplexer

// Current (in IronRDP):
tokio::select! {
    Some(event) = ev_receiver.recv() => {
        match event {
            ServerEvent::Clipboard(msg) => // handle directly
        }
    }
}

// Need to change to:
tokio::select! {
    Some(event) = ev_receiver.recv() => {
        match event {
            ServerEvent::Clipboard(msg) => {
                // Route through multiplexer
                multiplexer.send_clipboard(msg);
            }
        }
    }
}
```

**Problem:** We can't modify IronRDP's run() method without forking!

---

## IMPLEMENTATION APPROACH

### Option A: Fork IronRDP Event Loop (REQUIRED)

Since we can't hook into IronRDP's event processing, we must fork `accept_finalize()` and the client loop.

**Steps:**
1. Copy relevant code from IronRDP to wrd-server
2. Modify to route through our multiplexer
3. Maintain as IronRDP updates

**Files to fork:**
- Parts of `ironrdp-server/src/server.rs` (client loop ~300-500 lines)

**Maintenance:**
- Track IronRDP changes
- Rebase our modifications
- Test thoroughly after updates

### Option B: Wrapper + Intercept Pattern

**For Input:** Modify WrdInputHandler to route through multiplexer
**For Clipboard/Control:** Create wrapper around IronRDP that intercepts ServerEvent

**Hybrid approach:**
- Input: Full multiplexer control
- Clipboard/Control: Best-effort interception
- Graphics: Full multiplexer control (done)

---

## DETAILED IMPLEMENTATION

I'll implement Option A (full fork) to give you complete multiplexer control as requested.

### Step 1: Modify WrdInputHandler

**File:** `src/server/input_handler.rs`

Add input queue sender, route all events through multiplexer.

### Step 2: Create Multiplexer Event Loop

**File:** `src/server/multiplexer_loop.rs` (NEW)

This will be our custom client handling loop that replaces IronRDP's.

### Step 3: Fork IronRDP Connection Handler

Copy and modify `accept_finalize()` and client loop to use our multiplexer.

### Step 4: Wire Everything Together

Update server initialization to use our custom event loop instead of IronRDP's.

---

## STARTING IMPLEMENTATION NOW

I'll build this step by step and show you the progress.
