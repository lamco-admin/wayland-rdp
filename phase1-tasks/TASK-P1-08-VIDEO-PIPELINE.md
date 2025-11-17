# TASK P1-08: VIDEO PROCESSING PIPELINE
**Task ID:** TASK-P1-08
**Duration:** 7-10 days
**Dependencies:** TASK-P1-05, P1-06, P1-07
**Status:** NOT_STARTED

## OBJECTIVE
Implement complete video processing pipeline orchestrating frame reception, processing, and encoding.

## SUCCESS CRITERIA
- ✅ Frames processed at 30+ FPS
- ✅ Encoding latency < 33ms @ 30 FPS
- ✅ Damage tracking reduces encoding work
- ✅ Cursor metadata extracted
- ✅ Memory usage stable (no leaks)
- ✅ Pipeline survives errors gracefully

## KEY MODULES
- `src/video/pipeline.rs` - Pipeline orchestrator
- `src/video/damage.rs` - Damage tracking
- `src/video/cursor.rs` - Cursor handling
- `src/video/scaler.rs` - Resolution scaling

## CORE IMPLEMENTATION
```rust
pub struct VideoPipeline {
    encoder: Box<dyn VideoEncoder>,
    damage_tracker: DamageTracker,
    cursor_manager: CursorManager,
    frame_rx: mpsc::Receiver<VideoFrame>,
    encoded_tx: mpsc::Sender<EncodedFrame>,
    metrics: Arc<RwLock<PipelineMetrics>>,
}

impl VideoPipeline {
    pub async fn new(config: Arc<Config>, encoder_type: EncoderType) -> Result<Self>;
    pub async fn run(&mut self) -> Result<()>;
}
```

## DELIVERABLES
1. Pipeline orchestrator
2. Damage tracking
3. Cursor extraction
4. Format conversion
5. Thread pool for encoding
6. Buffer pool
7. Performance metrics
8. Benchmarks

**Time:** 7-10 days
