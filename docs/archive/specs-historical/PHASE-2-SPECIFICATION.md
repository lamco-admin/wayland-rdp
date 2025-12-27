# PHASE 2 CONSOLIDATED SPECIFICATION
**Document:** PHASE-2-SPECIFICATION.md
**Version:** 2.0
**Date:** 2025-01-18
**Status:** AUTHORITATIVE - Production Grade
**Parent:** 00-MASTER-SPECIFICATION.md
**Dependencies:** PHASE-1-SPECIFICATION.md (must be 100% complete first)

---

## PHASE 2 OVERVIEW

### Objective
Extend the production-ready Wayland Remote Desktop Server with **bidirectional audio streaming** and **advanced performance optimizations** to achieve v1.0 production release quality.

### Prerequisites
**Phase 1 MUST be 100% complete** before starting Phase 2:
- All Phase 1 acceptance criteria met
- All Phase 1 tests passing
- Production deployment validated
- Performance targets achieved

### Timeline  
**Duration:** 4 weeks
**Start:** Week 11 (after Phase 1 complete)
**Completion:** Week 14

### Features Added

#### Audio Features
- ✅ Audio streaming (server → client)
- ✅ Audio input (client → server, microphone)
- ✅ Audio encoding using Opus codec
- ✅ Audio/video synchronization
- ✅ Volume control
- ✅ Multiple audio devices support
- ✅ Low-latency audio (< 50ms)

#### Performance Optimizations
- ✅ Profile-guided optimizations
- ✅ Memory usage reduction
- ✅ CPU usage optimization
- ✅ Network bandwidth optimization
- ✅ Frame pacing improvements

#### Advanced Features
- ✅ Session persistence and reconnection
- ✅ Dynamic resolution adjustment
- ✅ Bandwidth adaptation
- ✅ Quality presets (Low/Medium/High/Auto)
- ✅ Prometheus metrics export
- ✅ Real-time performance monitoring

---

## PHASE 2 TASKS

### P2-01: Audio Capture (Week 11-12, 7-10 days)

**Objective:** Capture audio from Wayland compositor via PipeWire

**Key Components:**
- PipeWire audio stream connection
- Audio format negotiation (sample rate, channels, format)
- Audio buffer management
- Multiple audio source support
- Low-latency audio path (< 50ms)

**Deliverables:**
- src/audio/capture.rs - Audio capture module
- PipeWire audio stream implementation
- Audio buffer queue (ring buffer)
- Audio format conversion (all formats → S16LE/S24LE/F32)
- Integration test
- Example program

**Codec:** Raw PCM initially, Opus encoding in P2-02

**Technical Requirements:**
- Sample rates: 44.1kHz, 48kHz, 96kHz support
- Channels: Mono, stereo, 5.1, 7.1 support
- Formats: S16LE, S24LE, F32LE support
- Buffer size: Configurable (default 20ms)
- Latency: < 20ms capture to encode

### P2-02: Audio Encoding and RDP Channel (Week 12-13, 7-10 days)

**Objective:** Encode audio and send via IronRDP

**Key Components:**
- Opus audio encoder integration
- IronRDP RDPSND (audio output) channel
- RDPAI (audio input) channel for microphone
- Audio packet packetization
- Jitter buffer for smooth playback

**Deliverables:**
- src/audio/encoder.rs - Opus encoder
- src/audio/decoder.rs - Opus decoder (for microphone)
- IronRDP RdpsndServer integration
- Audio/video timestamp synchronization
- Lip-sync implementation
- Integration test

**Codec:** Opus
- Bitrate: 32-128 kbps (configurable)
- Complexity: 5-10 (configurable)
- Frame size: 20ms (960 samples @ 48kHz)

**Technical Requirements:**
- A/V sync tolerance: ± 40ms
- Audio latency: < 50ms total
- Packet loss handling: FEC and PLC
- Adaptive bitrate: Based on network conditions

### P2-03: Performance Optimization (Week 13-14, 7-10 days)

**Objective:** Optimize all performance aspects for production

**Key Components:**
- CPU profiling and optimization
- Memory profiling and reduction
- Network optimization (TCP tuning)
- Frame pacing improvements
- Cache optimization

**Deliverables:**
- Performance profiling reports
- Optimized bitmap conversion (SIMD tuning)
- Memory pool optimizations
- Network buffer tuning
- Frame rate smoothing algorithm
- Benchmark improvements

**Target Improvements:**
- CPU usage: -15% (from 40% to 25% at 1080p30)
- Memory usage: -20% (from 400MB to 320MB)
- Latency: -20% (from 80ms to 64ms)
- Bandwidth: -10% (from 20Mbps to 18Mbps)

### P2-04: Advanced Features & Polish (Week 14, 5-7 days)

**Objective:** Production polish and advanced features

**Key Components:**
- Prometheus metrics export
- Real-time performance dashboard
- Session reconnection
- Dynamic quality adjustment
- Comprehensive logging

**Deliverables:**
- Prometheus metrics endpoint
- Quality presets (Low/Medium/High/Auto)
- Reconnection logic
- Bandwidth adaptation algorithm
- Performance dashboard (optional web UI)
- Production documentation

---

## AUDIO TECHNICAL SPECIFICATIONS

### PipeWire Audio Integration

**Audio Stream Connection:**
```rust
// Similar to video, but for audio
let audio_props = pw_properties_new(
    "media.type", "Audio",
    "media.category", "Capture",
    "media.role", "Communication",
);

let audio_stream = pw_stream_new_simple(
    loop,
    "wrd-server-audio",
    audio_props,
    audio_process_callback,
    userdata,
);

pw_stream_connect(
    audio_stream,
    PW_DIRECTION_INPUT,
    PW_ID_ANY,
    (PW_STREAM_FLAG_AUTOCONNECT | PW_STREAM_FLAG_MAP_BUFFERS),
    params,
    n_params,
);
```

**Format Negotiation:**
- Prefer: 48kHz, stereo, F32LE
- Fallback: 44.1kHz, stereo, S16LE
- Support: All standard PCM formats

### Opus Encoder Configuration

**Encoder Settings:**
```rust
opus_encoder_create(48000, 2, OPUS_APPLICATION_RESTRICTED_LOWDELAY)?;
opus_encoder_ctl(enc, OPUS_SET_BITRATE(64000))?;
opus_encoder_ctl(enc, OPUS_SET_COMPLEXITY(8))?;
opus_encoder_ctl(enc, OPUS_SET_SIGNAL(OPUS_SIGNAL_MUSIC))?;
opus_encoder_ctl(enc, OPUS_SET_DTX(0))?; // No DTX for low latency
opus_encoder_ctl(enc, OPUS_SET_FEC(1))?; // Enable FEC
```

**Frame Configuration:**
- Frame size: 20ms (960 samples @ 48kHz)
- Lookahead: 0ms (low latency)
- Packet loss: 10% FEC
- VBR: Enabled

### Audio/Video Synchronization

**Synchronization Strategy:**
```
Video PTS (microseconds)
Audio PTS (microseconds)
  ↓
Calculate drift:
  drift = video_pts - audio_pts

If drift > 40ms:
  If video ahead: Skip video frame
  If audio ahead: Insert silence

Maintain sync buffer:
  ±3 frames tolerance
  Adjust playout timing
```

**A/V Sync Algorithm:**
- Measure PTS difference continuously
- Maintain running average over 10 seconds
- Adjust playback rate ±2% if needed
- Drop/duplicate frames if drift > 100ms

---

## PERFORMANCE OPTIMIZATION STRATEGY

### CPU Optimization

**Profiling:**
- Use perf, flamegraph
- Identify hot paths
- Focus on bitmap conversion loop

**Optimizations:**
- SIMD: Ensure AVX2 usage in bitmap conversion
- Parallelization: Multi-core bitmap processing
- Caching: Bitmap diff caching
- Algorithmic: Reduce redundant calculations

### Memory Optimization

**Profiling:**
- Use valgrind massif
- Identify allocation hotspots
- Check for leaks

**Optimizations:**
- Buffer pooling: Reuse frame buffers
- Arena allocation: Group allocations
- Zero-copy: DMA-BUF paths
- Reduce copies: In-place conversions

### Network Optimization

**TCP Tuning:**
- Increase send/receive buffers
- Enable TCP_NODELAY for low latency
- Consider TCP_CORK for batching

**Bandwidth Optimization:**
- Adaptive bitrate based on RTT
- Frame skipping under congestion
- RemoteFX quality adjustment

---

## DEPENDENCIES (Phase 2 Additions)

```toml
[dependencies]
# Audio encoding
opus = "0.3.0"
libopus-sys = "0.1.10"

# Audio processing
cpal = "0.15.2"  # Cross-platform audio (for microphone)
dasp = "0.11.0"  # Digital audio signal processing

# Metrics
prometheus = "0.13.3"
```

---

## PHASE 2 ACCEPTANCE CRITERIA

### Audio Functionality
- [ ] Audio plays on client
- [ ] Microphone works (client → server)
- [ ] A/V sync within ±40ms
- [ ] Audio latency < 50ms
- [ ] Volume control functional
- [ ] No audio dropouts

### Performance Improvements
- [ ] CPU usage reduced by 15%
- [ ] Memory usage reduced by 20%
- [ ] Latency reduced by 20%
- [ ] Bandwidth optimized
- [ ] Frame pacing smooth

### Advanced Features
- [ ] Reconnection works
- [ ] Quality presets functional
- [ ] Metrics exportable
- [ ] Monitoring dashboard operational

### Quality
- [ ] All audio tests pass
- [ ] Performance benchmarks meet targets
- [ ] Code coverage maintained > 80%
- [ ] Documentation updated

---

## DELIVERABLES

### Code
- [ ] src/audio/ module complete
- [ ] Audio integration with IronRDP
- [ ] Performance optimizations applied
- [ ] Metrics and monitoring

### Tests
- [ ] Audio capture tests
- [ ] Audio encode/decode tests
- [ ] A/V sync tests
- [ ] Performance regression tests

### Documentation
- [ ] Audio configuration guide
- [ ] Performance tuning guide
- [ ] Metrics documentation
- [ ] v1.0 release notes

---

## PHASE 2 COMPLETION = V1.0 RELEASE

When Phase 2 is complete, the project reaches **v1.0 production release**:

- ✅ Full-featured remote desktop server
- ✅ Video + Audio streaming
- ✅ Complete input control
- ✅ Clipboard synchronization
- ✅ Multi-monitor support
- ✅ Production-grade performance
- ✅ Enterprise-ready security
- ✅ Comprehensive documentation
- ✅ Complete test coverage

**Release Criteria:**
- All Phase 1 + Phase 2 acceptance criteria met
- No critical or high-severity bugs
- Documentation complete
- Deployment validated
- Performance targets exceeded

---

**END OF PHASE 2 SPECIFICATION**

**Status:** PRODUCTION-GRADE PRD/SRS QUALITY  
**Completeness:** 100%
**Timeline:** 4 weeks to v1.0
