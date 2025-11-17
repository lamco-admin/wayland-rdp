# TASK P1-06: SOFTWARE H.264 ENCODER (OpenH264)
**Task ID:** TASK-P1-06
**Duration:** 5-7 days
**Dependencies:** TASK-P1-01
**Status:** NOT_STARTED

## OBJECTIVE
Implement software H.264 encoding using OpenH264 as fallback encoder.

## SUCCESS CRITERIA
- ✅ OpenH264 encoder initializes
- ✅ Encodes frames to H.264
- ✅ Output decodes correctly (verify with ffmpeg)
- ✅ Bitrate control working
- ✅ Keyframe intervals correct
- ✅ NAL units extracted properly

## KEY MODULES
- `src/video/encoder/mod.rs` - Common encoder trait
- `src/video/encoder/openh264.rs` - OpenH264 implementation

## CORE IMPLEMENTATION
```rust
#[async_trait]
pub trait VideoEncoder: Send + Sync {
    async fn encode(&mut self, frame: &VideoFrame) -> Result<EncodedFrame>;
    async fn flush(&mut self) -> Result<Vec<EncodedFrame>>;
    fn set_bitrate(&mut self, kbps: u32) -> Result<()>;
}

pub struct OpenH264Encoder {
    encoder: openh264::encoder::Encoder,
    config: EncoderConfig,
    sequence: u64,
}

impl OpenH264Encoder {
    pub fn new(config: Arc<Config>, width: u32, height: u32) -> Result<Self>;
}
```

## DELIVERABLES
1. Encoder trait definition
2. OpenH264 implementation
3. Format conversion (BGRA→NV12→I420)
4. NAL unit extraction
5. Bitrate control
6. Benchmarks
7. Tests with ffmpeg validation

**Time:** 5-7 days
