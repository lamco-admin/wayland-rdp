# TASK P1-07: HARDWARE H.264 ENCODER (VA-API)
**Task ID:** TASK-P1-07
**Duration:** 10-14 days
**Dependencies:** TASK-P1-06
**Status:** NOT_STARTED

## OBJECTIVE
Implement VA-API hardware-accelerated H.264 encoding for Intel/AMD GPUs.

## SUCCESS CRITERIA
- ✅ VA-API device opens successfully
- ✅ H.264 encoding profile supported
- ✅ Hardware encoding functional
- ✅ DMA-BUF zero-copy from PipeWire
- ✅ Performance significantly better than OpenH264
- ✅ Graceful fallback on failure

## KEY MODULES
- `src/video/encoder/vaapi.rs` - VA-API implementation

## CORE IMPLEMENTATION
```rust
pub struct VaApiEncoder {
    display: VADisplay,
    context: VAContextID,
    config: VAConfigID,
    surfaces: Vec<VASurfaceID>,
    coded_buf: VABufferID,
}

impl VaApiEncoder {
    pub fn new(config: Arc<Config>, width: u32, height: u32) -> Result<Self>;
    pub fn is_available(config: &Config) -> bool;
    
    fn upload_frame(&mut self, frame: &VideoFrame) -> Result<VASurfaceID>;
    fn encode_surface(&mut self, surface: VASurfaceID) -> Result<Vec<u8>>;
}
```

## DELIVERABLES
1. VA-API initialization
2. Surface management
3. DMA-BUF import
4. Hardware encoding
5. Performance testing
6. Fallback mechanism
7. GPU compatibility tests

**Time:** 10-14 days
