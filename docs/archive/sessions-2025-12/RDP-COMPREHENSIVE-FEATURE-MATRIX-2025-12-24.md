# RDP Protocol - Comprehensive Feature Matrix & Roadmap

**Date:** 2025-12-24
**Purpose:** Complete feature analysis for strategic planning
**Scope:** All MS-RDP protocol extensions and capabilities

---

## Feature Categories

1. **Graphics & Video** - EGFX, RemoteFX, H.264, Damage Tracking
2. **Audio** - Audio Output, Audio Input (Microphone)
3. **Display** - Multimonitor, Resolution Changes, DPI
4. **Input** - Mouse, Keyboard, Touch, Pen
5. **Device Redirection** - Clipboard, Files, Printers, Smartcards, USB
6. **Advanced** - Remote Apps (RAIL), Network Auto-Detect, QoE

---

## 1. Graphics & Video Features

### MS-RDPEGFX (Graphics Pipeline Extension)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **ZGFX Compression** | MS-RDPEGFX 2.2.1.1 | ❌ Missing | **P0** | 4-6h wrapper, 12-20h full | None |
| **H.264 AVC420** | MS-RDPEGFX 2.2.4.4 | ⚠️ Partial | **P0** | 4-8h (level config) | ZGFX |
| **H.264 AVC444** | MS-RDPEGFX 2.2.4.5 | ❌ Not started | P1 | 8-12h | AVC420, ZGFX |
| **RemoteFX Progressive** | MS-RDPEGFX 2.2.4.6 | ❌ Not started | P2 | 16-24h | ZGFX |
| **ClearCodec** | MS-RDPEGFX 2.2.4.7 | ❌ Not started | P3 | 12-16h | ZGFX |
| **Planar Codec** | MS-RDPEGFX 2.2.4.8 | ❌ Not started | P3 | 8-12h | ZGFX |
| **Surface Management** | MS-RDPEGFX 2.2.2.4-2.2.2.6 | ✅ Done | - | - | - |
| **Frame Acknowledgement** | MS-RDPEGFX 2.2.2.7 | ✅ Done | - | - | - |
| **Cache Management** | MS-RDPEGFX 2.2.2.14-2.2.2.15 | ⚠️ Partial | P2 | 4-8h | ZGFX |
| **QoE Metrics** | MS-RDPEGFX 2.2.2.8 | ✅ Done | - | - | - |
| **Damage Rectangles** | Custom/PipeWire | ❌ Not used | **P1** | 8-12h | ZGFX, AVC420 |
| **H.264 Level Management** | ITU-T H.264 | ⚠️ Designed | **P1** | 4-6h | ZGFX |
| **Adaptive Quality (QP)** | Custom | ❌ Not started | P2 | 6-10h | AVC420 working |

**Current Capabilities:**
- ✅ V10.6 capability negotiation
- ✅ Surface creation/deletion
- ✅ AVC420 encoding (OpenH264)
- ✅ Frame management
- ❌ **ZGFX compression (BLOCKER!)**

---

## 2. Audio Features

### MS-RDPSND (Audio Output Virtual Channel)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Server Audio Output** | MS-RDPSND | ❌ Not started | **P1** | 12-16h | PipeWire audio |
| **Audio Formats** | MS-RDPSND 2.2.3 | N/A | P1 | Included | Audio output |
| - PCM (uncompressed) | MS-RDPSND 2.2.3.1 | ❌ | P1 | 2-4h | Audio output |
| - ADPCM | MS-RDPSND 2.2.3.2 | ❌ | P2 | 4-6h | Audio output |
| - AAC | MS-RDPSND 2.2.3.4 | ❌ | P1 | 6-8h | Audio output, codec |
| - Opus | Custom | ❌ | P2 | 6-8h | Audio output, codec |
| **Volume Control** | MS-RDPSND 2.2.2.4 | ❌ | P2 | 2-3h | Audio output |
| **Audio Quality** | MS-RDPSND | ❌ | P2 | 3-4h | Audio output |

### MS-RDPEA (Audio Input / Microphone)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Microphone Redirection** | MS-RDPEA | ❌ Not started | P2 | 12-16h | PipeWire, Portal |
| **Audio Input Formats** | MS-RDPEA 2.2.2 | N/A | P2 | Included | Mic redirection |
| **Dynamic Format Negotiation** | MS-RDPEA | ❌ | P2 | 4-6h | Mic redirection |

**Current Status:** No audio implementation

---

## 3. Display & Monitor Features

### MS-RDPEDISP (Display Control)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Display Update** | MS-RDPEDISP | ⚠️ Partial | **P1** | 6-8h | None |
| **Resolution Change** | MS-RDPEDISP 2.2.2.2 | ⚠️ Basic | **P1** | 4-6h | Display Update |
| **Monitor Add/Remove** | MS-RDPEDISP 2.2.2.2 | ❌ | **P1** | 6-8h | Display Update |
| **Monitor Reposition** | MS-RDPEDISP 2.2.2.2 | ❌ | P1 | 4-6h | Monitor Add/Remove |
| **Orientation Change** | MS-RDPEDISP 2.2.2.2 | ❌ | P2 | 3-4h | Display Update |
| **DPI Scaling** | MS-RDPEDISP 2.2.2.2 | ❌ | P2 | 4-6h | Display Update |

### Multimonitor Support

| Feature | Implementation | Status | Priority | Effort | Dependencies |
|---------|----------------|--------|----------|--------|--------------|
| **Multiple Displays** | Config | ⚠️ Config only | **P1** | 8-12h | RDPEDISP |
| **Per-Monitor Rendering** | PipeWire | ❌ | **P1** | 10-14h | Multimon, PipeWire |
| **Monitor Hotplug** | Portal/RDPEDISP | ❌ | P1 | 6-8h | RDPEDISP |
| **Spanning/Combined** | RDPEDISP | ❌ | P2 | 6-10h | Basic multimon |
| **Per-Monitor DPI** | RDPEDISP | ❌ | P2 | 4-6h | DPI Scaling |

**Current:** `max_monitors: 4` in config, not implemented

---

## 4. Input Features

### MS-RDPEI (Input Virtual Channel)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Mouse Input** | Core RDP | ✅ Done | - | - | libei |
| **Keyboard Input** | Core RDP | ✅ Done | - | - | libei |
| **Touch Input** | MS-RDPEI | ⚠️ Config | P2 | 6-10h | libei, Portal |
| **Multi-touch** | MS-RDPEI 2.2.3.3 | ❌ | P2 | 8-12h | Touch |
| **Pen/Stylus** | MS-RDPEI | ❌ | P3 | 8-12h | libei, Portal |
| **Pen Pressure/Tilt** | MS-RDPEI 2.2.3.4 | ❌ | P3 | 4-6h | Pen |

**Current:** Mouse and keyboard via libei ✅

---

## 5. Device Redirection

### MS-RDPECLIP (Enhanced Clipboard)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Text Clipboard** | MS-RDPECLIP | ✅ Done | - | - | Portal |
| **Image Clipboard** | MS-RDPECLIP | ✅ Done | - | - | Portal |
| **File Transfer** | MS-RDPECLIP 2.2.5 | ✅ Done | - | - | Portal |
| **Large File Streaming** | MS-RDPECLIP | ✅ Done | - | - | File transfer |
| **Clipboard Locking** | MS-RDPECLIP 2.2.3.2 | ✅ Done | - | - | - |

**Current:** Full bidirectional clipboard with file transfer ✅

### MS-RDPEFS (File System Redirection)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Drive Redirection** | MS-RDPEFS | ❌ Not started | P2 | 20-30h | Portal filesystem |
| **File Operations** | MS-RDPEFS 2.2.1 | ❌ | P2 | Included | Drive redirect |
| **Directory Enumeration** | MS-RDPEFS | ❌ | P2 | Included | Drive redirect |
| **File Locking** | MS-RDPEFS | ❌ | P3 | 4-6h | Drive redirect |

### MS-RDPEUSB (USB Redirection)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **USB Device Redirect** | MS-RDPEUSB | ❌ Not started | P3 | 24-40h | Kernel driver |
| **USB Printer** | MS-RDPEUSB | ❌ | P3 | 12-16h | USB redirect |
| **USB Storage** | MS-RDPEUSB | ❌ | P3 | 12-16h | USB redirect |

### MS-RDPESP (Smartcard Redirection)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Smartcard Redirect** | MS-RDPESP | ❌ Not started | P4 | 16-24h | PC/SC |

### MS-RDPEPC (Printer Redirection)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Printer Redirection** | MS-RDPEPC | ❌ Not started | P3 | 16-20h | CUPS |

---

## 6. Advanced Features

### MS-RDPERP (RemoteApp / RAIL)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Remote Applications** | MS-RDPERP | ❌ Not started | P2 | 30-50h | Window management |
| **Window Management** | MS-RDPERP 2.2.1 | ❌ | P2 | Included | RemoteApp |
| **Taskbar Integration** | MS-RDPERP | ❌ | P3 | 8-12h | RemoteApp |
| **System Commands** | MS-RDPERP 2.2.2.3 | ❌ | P3 | 4-6h | RemoteApp |

### MS-RDPBCGR (Network Characteristics Detection)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Bandwidth Detection** | MS-RDPBCGR 3.2.5.4 | ❌ | P2 | 8-12h | Network stack |
| **RTT Measurement** | MS-RDPBCGR | ❌ | P2 | 4-6h | Network stack |
| **Auto-Detect Protocol** | MS-RDPBCGR | ❌ | P2 | 10-14h | Bandwidth detect |
| **Adaptive Quality** | Custom | ❌ | P1 | 8-12h | Network detect, EGFX |

### MS-RDPESC (Multiparty / Collaboration)

| Feature | Specification | Status | Priority | Effort | Dependencies |
|---------|--------------|--------|----------|--------|--------------|
| **Session Sharing** | MS-RDPESC | ❌ | P4 | 40+ hours | Complex |

---

## Implementation Status Summary

### ✅ Completed Features (Production Ready)

1. **Core RDP Server** - Connection, authentication, basic graphics
2. **Clipboard** - Full bidirectional with files and images
3. **Input** - Mouse and keyboard via libei
4. **EGFX Protocol** - Surface management, frame tracking (sans compression)
5. **H.264 Encoding** - OpenH264 integration, AVC420 format

### ⚠️ Partially Complete (Needs Work)

1. **EGFX Graphics** - Missing ZGFX compression ← **BLOCKER**
2. **Multimonitor** - Config exists, not implemented
3. **Display Control** - Basic support, needs enhancement
4. **Touch Input** - Config flag, not implemented

### ❌ Not Started (Future Work)

1. **Audio** (input/output)
2. **Drive Redirection**
3. **RemoteApp/RAIL**
4. **Advanced codecs** (AVC444, RemoteFX, ClearCodec)
5. **USB Redirection**
6. **Printer Redirection**
7. **Network Auto-Detect**

---

## Strategic Priority Matrix

### P0: Critical Blockers (Must Have for MVP)

| Feature | Blocks | Impact | Effort |
|---------|--------|--------|--------|
| **ZGFX Wrapper** | Everything EGFX | Connection fails | 4-6h |
| **H.264 AVC420 Complete** | Video streaming | No video | 4-8h |

**Timeline:** 1-2 days focused work
**Outcome:** Working H.264 video streaming

### P1: Core Features (Essential for Production)

| Feature | Value Proposition | Effort | Dependencies |
|---------|-------------------|--------|--------------|
| **H.264 Level Management** | Multi-resolution support | 4-6h | ZGFX |
| **Damage Tracking** | 90% bandwidth reduction | 8-12h | ZGFX, AVC420 |
| **ZGFX Full Compression** | 2-10x bandwidth savings | 12-20h | ZGFX wrapper |
| **Multimonitor** | Professional/power users | 8-12h | EGFX working |
| **Display Control (Full)** | Resolution changes, DPI | 6-10h | EGFX working |
| **Audio Output** | Complete remote desktop experience | 12-16h | PipeWire audio |
| **Adaptive Quality** | Network resilience | 8-12h | EGFX, network detect |

**Timeline:** 4-6 weeks
**Outcome:** Production-quality remote desktop

### P2: Enhanced Features (Nice to Have)

| Feature | Use Cases | Effort | Dependencies |
|---------|-----------|--------|--------------|
| **Touch Input** | Tablet/touchscreen support | 6-10h | libei, Portal |
| **Audio Input** | Conferencing, voice | 12-16h | PipeWire, Portal |
| **H.264 AVC444** | Better color quality | 8-12h | AVC420 stable |
| **Drive Redirection** | File access | 20-30h | Portal filesystem |
| **RemoteApp/RAIL** | App-level remoting | 30-50h | Window management |
| **Network Auto-Detect** | Automatic adaptation | 10-14h | Network stack |
| **Cache Management** | Startup performance | 4-8h | EGFX stable |

**Timeline:** 8-12 weeks
**Outcome:** Feature-complete product

### P3-P4: Advanced/Niche Features

| Feature | Target Users | Effort | Priority |
|---------|--------------|--------|----------|
| **RemoteFX Progressive** | Legacy compatibility | 16-24h | P3 |
| **ClearCodec** | High-quality lossy | 12-16h | P3 |
| **Planar Codec** | Specific use cases | 8-12h | P3 |
| **USB Redirection** | Specialized hardware | 24-40h | P3 |
| **Printer Redirection** | Print scenarios | 16-20h | P3 |
| **Smartcard** | Security/authentication | 16-24h | P4 |
| **Session Sharing** | Collaboration | 40+ hours | P4 |

**Timeline:** 12-20 weeks
**Outcome:** Enterprise feature set

---

## Effort Summary

### By Priority

| Priority | Features | Total Hours | Timeline |
|----------|----------|-------------|----------|
| **P0** | 2 | 8-14h | 1-2 days |
| **P1** | 8 | 66-98h | 4-6 weeks |
| **P2** | 7 | 98-148h | 8-12 weeks |
| **P3-P4** | 9 | 148-224h | 12-20 weeks |

### By Category

| Category | Features | Total Hours | Business Value |
|----------|----------|-------------|----------------|
| **Graphics/Video** | 13 | 98-154h | ⭐⭐⭐⭐⭐ Critical |
| **Audio** | 7 | 42-64h | ⭐⭐⭐⭐ High |
| **Display** | 6 | 28-44h | ⭐⭐⭐⭐ High |
| **Input** | 6 | 14-28h | ⭐⭐⭐ Medium |
| **Device Redirect** | 9 | 94-150h | ⭐⭐⭐ Medium |
| **Advanced** | 4 | 62-92h | ⭐⭐ Low-Medium |

---

## Recommended Implementation Sequence

### Sprint 1: EGFX Foundation (Week 1-2)

**Goal:** Working H.264 video streaming

1. **ZGFX Uncompressed Wrapper** (4-6h) ← **START HERE**
   - Implement wrapper.rs
   - Integrate in ironrdp-egfx
   - Test with Windows client
   - **Unblocks everything!**

2. **H.264 Level Configuration** (4-6h)
   - Fix encoder_ext.rs compilation
   - Integrate LevelAwareEncoder
   - Test multi-resolution support

3. **Frame ACK Verification** (2-3h)
   - Confirm backpressure clears
   - Monitor stability
   - Log analysis

**Deliverable:** Stable H.264 streaming at 1280×800 @ 30fps

### Sprint 2: Performance & Quality (Week 3-4)

**Goal:** Optimize bandwidth and adapt to network conditions

4. **Damage Tracking** (8-12h)
   - Extract PipeWire damage rectangles
   - Multi-region encoding
   - Test bandwidth savings

5. **ZGFX Full Compression** (12-20h)
   - Implement match finding
   - Token encoding
   - File IronRDP PR

6. **Adaptive Quality** (8-12h)
   - QP adjustment based on backpressure
   - Network condition monitoring
   - Quality/bandwidth tradeoff

**Deliverable:** Optimized streaming (90%+ bandwidth reduction for typical use)

### Sprint 3: Display & Audio (Week 5-6)

**Goal:** Complete remote desktop experience

7. **Multimonitor Implementation** (8-12h)
   - Per-monitor PipeWire streams
   - Monitor add/remove/reposition
   - RDPEDISP integration

8. **Audio Output** (12-16h)
   - PipeWire audio capture
   - PCM/AAC encoding
   - RDPSND channel

9. **Display Control Full** (6-10h)
   - Dynamic resolution
   - DPI awareness
   - Orientation changes

**Deliverable:** Full desktop experience with audio

### Sprint 4: Enhanced Input (Week 7-8)

**Goal:** Touch and advanced input support

10. **Touch Input** (6-10h)
    - Multi-touch via libei
    - RDPEI channel
    - Gesture support

11. **Audio Input** (12-16h)
    - Microphone via PipeWire
    - RDPEA channel

**Deliverable:** Tablet/touch device support

### Sprint 5: Advanced Features (Week 9+)

**Goal:** Enterprise capabilities

12. **RemoteApp/RAIL** (30-50h)
13. **Drive Redirection** (20-30h)
14. **Advanced Codecs** (24-40h) - AVC444, RemoteFX, ClearCodec
15. **USB/Printer Redirection** (32-44h)

**Deliverable:** Enterprise feature parity

---

## Feature Decision Guide

### Must Implement (No Product Without These)

- ✅ ZGFX Wrapper (unblocks everything)
- ✅ H.264 AVC420 complete
- ✅ Basic multimonitor
- ✅ Audio output

### Should Implement (Competitive Features)

- ZGFX full compression
- Damage tracking
- Touch input
- Display control (full)
- Audio input
- Adaptive quality

### Could Implement (Differentiation)

- RemoteApp
- Drive redirection
- Advanced codecs (AVC444, RemoteFX)
- Network auto-detect

### Won't Implement (Initially)

- USB redirection (complex, niche)
- Printer redirection (niche)
- Smartcard (specialized)
- Session sharing (enterprise-only)

---

## Technical Dependencies Graph

```
ZGFX Wrapper (P0)
  ├─→ H.264 AVC420 Complete (P0)
  │     ├─→ Damage Tracking (P1)
  │     ├─→ H.264 Level Mgmt (P1)
  │     ├─→ Adaptive Quality (P1)
  │     └─→ AVC444 (P2)
  │
  ├─→ ZGFX Full Compression (P1)
  │
  └─→ All other EGFX codecs (P2-P3)

EGFX Working
  ├─→ Multimonitor (P1)
  └─→ Display Control (P1)

PipeWire Audio
  ├─→ Audio Output (P1)
  └─→ Audio Input (P2)

libei + Portal
  ├─→ Touch Input (P2)
  └─→ Pen Input (P3)

Portal Filesystem
  └─→ Drive Redirection (P2)
```

---

## Immediate Action: ZGFX Wrapper Implementation

**This is what we implement NOW:**

1. Create `ironrdp-graphics/src/zgfx/wrapper.rs`
2. Modify `ironrdp-graphics/src/zgfx/mod.rs` to export wrapper
3. Modify `ironrdp-egfx/src/server.rs` to wrap GfxPdus
4. Test with Windows client
5. Verify: No "BulkCompressorFailed" error
6. Verify: Frame ACKs received
7. **EGFX unblocked!**

Then we can build out the rest systematically per the roadmap.

---

## Next Document

Creating full feature roadmap with timelines, resource estimates, and decision matrices...
