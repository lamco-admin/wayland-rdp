# TASK P1-05: PIPEWIRE INTEGRATION
**Task ID:** TASK-P1-05
**Duration:** 7-10 days
**Dependencies:** TASK-P1-04
**Status:** NOT_STARTED

## OBJECTIVE
Implement PipeWire integration for receiving video frames from compositor via portal-provided file descriptor.

## SUCCESS CRITERIA
- ✅ PipeWire connection established using FD from portal
- ✅ Video format negotiation complete (prefer NV12)
- ✅ Frames received at target FPS
- ✅ Frame metadata correct (dimensions, PTS, format)
- ✅ Multiple streams handled (multi-monitor)
- ✅ DMA-BUF zero-copy working (if available)

## KEY MODULES
- `src/pipewire/stream.rs` - PipeWire stream management
- `src/pipewire/receiver.rs` - Async frame reception
- `src/pipewire/format.rs` - Format negotiation

## CORE IMPLEMENTATION
```rust
pub struct PipeWireStream {
    fd: RawFd,
    context: *mut pw_context,
    core: *mut pw_core,
    stream: *mut pw_stream,
}

impl PipeWireStream {
    pub async fn new(fd: RawFd, stream_info: &StreamInfo) -> Result<Self>;
    pub async fn start(&mut self) -> Result<()>;
    pub fn set_frame_callback<F>(&mut self, callback: F);
}

pub struct FrameReceiver {
    stream: PipeWireStream,
    frame_tx: mpsc::Sender<VideoFrame>,
}
```

## DELIVERABLES
1. PipeWire stream connection
2. Format negotiation
3. Frame receiver with async callbacks
4. DMA-BUF import support
5. Multi-stream handling
6. Integration tests

**Time:** 7-10 days
