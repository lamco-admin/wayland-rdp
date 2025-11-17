# TASK P1-09: RDP GRAPHICS CHANNEL
**Task ID:** TASK-P1-09
**Duration:** 7-10 days
**Dependencies:** TASK-P1-03, P1-08
**Status:** NOT_STARTED

## OBJECTIVE
Implement RDP Graphics Pipeline channel for sending H.264 encoded video to client.

## SUCCESS CRITERIA
- ✅ Video displays on Windows mstsc client
- ✅ Frame rate stable @ 30 FPS
- ✅ Latency acceptable (< 100ms)
- ✅ No visual artifacts
- ✅ Bitrate adapts to network conditions
- ✅ Cursor updates sent separately

## KEY MODULES
- `src/rdp/channels/graphics.rs` - Graphics channel
- `src/rdp/codec/h264.rs` - H.264 codec setup

## CORE IMPLEMENTATION
```rust
pub struct GraphicsChannel {
    channel_id: u16,
    codec: H264Codec,
    encoder_rx: mpsc::Receiver<EncodedFrame>,
}

impl GraphicsChannel {
    pub async fn send_frame(&mut self, frame: EncodedFrame) -> Result<()>;
    pub async fn send_cursor(&mut self, cursor: CursorInfo) -> Result<()>;
    
    fn packetize_frame(&self, frame: &EncodedFrame) -> Vec<GraphicsPdu>;
}
```

## DELIVERABLES
1. Graphics channel implementation
2. H.264 codec configuration (AVC444)
3. Frame packetization
4. Rate control feedback
5. Cursor updates
6. Integration with video pipeline
7. Visual quality tests

**Time:** 7-10 days
