# Multi-Language AVC444 Implementation Research - 2025-12-29

**Date**: 2025-12-29 15:50 UTC
**Goal**: Find working AVC444 server implementations across all languages
**Method**: Systematic search and analysis

---

## IMPLEMENTATIONS FOUND

### C/C++ Implementations

#### 1. FreeRDP (Primary Reference)

**Repository**: https://github.com/FreeRDP/FreeRDP
**File**: `libfreerdp/codec/h264.c`
**Function**: `avc444_compress()`

**Key Implementation Details**:

```c
// ONE encoder instance in H264_CONTEXT
h264->subsystem->Compress(...)

// Change detection for luma and chroma SEPARATELY
detect_changes(..., pYUV444Data, pOldYUV444Data, ..., meta);
detect_changes(..., pYUVData, pOldYUVData, ..., auxMeta);

// LC field logic (CRITICAL - this is the bandwidth win!)
if ((meta->numRegionRects > 0) && (auxMeta->numRegionRects > 0))
    *op = 0;  // Both luma and chroma changed
else if (meta->numRegionRects > 0)
    *op = 1;  // Luma only (OMIT AUX!)
else if (auxMeta->numRegionRects > 0)
    *op = 2;  // Chroma only

// ENCODE LUMA (if needed)
if ((*op == 0) || (*op == 1)) {
    h264->subsystem->Compress(h264, pcYUV444Data, ..., &coded, &codedSize);
    h264->firstLumaFrameDone = TRUE;
    *ppDstData = coded;  // Main stream
}

// ENCODE CHROMA (if needed) - SAME ENCODER!
if ((*op == 0) || (*op == 2)) {
    h264->subsystem->Compress(h264, pcYUVData, ..., &coded, &codedSize);
    h264->firstChromaFrameDone = TRUE;
    *ppAuxDstData = coded;  // Aux stream
}
```

**CRITICAL OBSERVATIONS**:

1. **ONE encoder instance** (`h264->subsystem->Compress`) - confirms spec requirement
2. **Change detection determines which streams to encode**:
   - If luma unchanged → skip luma encode (LC=2)
   - If chroma unchanged → skip chroma encode (LC=1)
   - If both changed → encode both (LC=0)
3. **Sequential encoding**: Calls Compress() for luma, then for chroma
4. **firstLumaFrameDone / firstChromaFrameDone**: Track state separately

**BANDWIDTH OPTIMIZATION**: Comes from **NOT encoding unchanged streams**, not from P-frames!

**Question**: Does their encoder produce P-frames for aux? Not visible in this code.

---

#### 2. GNOME Remote Desktop

**Repository**: https://github.com/GNOME/gnome-remote-desktop (mirror)
**Primary**: https://gitlab.gnome.org/GNOME/gnome-remote-desktop
**Language**: C
**Encoder**: VA-API (hardware), not OpenH264

**Key Files**:
- `src/grd-rdp-dvc-graphics-pipeline.c` - AVC444 packet preparation
- `src/grd-encode-session-vaapi.c` - VA-API hardware encoding
- `src/grd-rdp-frame.c` - Frame management

**Implementation Notes**:

```c
// LC field handling (same as FreeRDP)
switch (view_type) {
    case GRD_RDP_FRAME_VIEW_TYPE_DUAL:
        avc444->LC = 0;  // Both streams
        break;
    case GRD_RDP_FRAME_VIEW_TYPE_MAIN:
        avc444->LC = 1;  // Luma only
        break;
    case GRD_RDP_FRAME_VIEW_TYPE_AUX:
        avc444->LC = 2;  // Chroma only
        break;
}
```

**Frame types referenced**: `GRD_AVC_FRAME_TYPE_P` exists in code

**CRITICAL**: Uses **hardware encoder (VA-API)**, not software (OpenH264)
- May have different frame type behavior
- May handle P-frames differently

**Status**: Actively maintained (GNOME 48 added AVC444 hardware encoding)

---

#### 3. xrdp

**Repository**: https://github.com/neutrinolabs/xrdp
**Language**: C
**Encoder**: OpenH264

**Findings**:
- Has AVC444 codec ID constants defined (`XR_RDPGFX_CODECID_AVC444 = 0x000E`)
- Has OpenH264 encoder implementation (`xrdp_encoder_openh264.c`)
- **NO avc444_compress or AVC444 encoding implementation found**
- Likely only supports AVC420, not AVC444 server-side

**Conclusion**: Not useful as reference for AVC444

---

### Go Implementations

#### 1. grdp (tomatome)

**Repository**: https://github.com/tomatome/grdp
**Stars**: 215
**Description**: "pure golang rdp protocol"
**Status**: Client implementation, not server

**Checked**: No AVC444 encoding (client only)

#### 2. rdpgo (mojocn)

**Repository**: https://github.com/mojocn/rdpgo
**Stars**: 265
**Description**: "Websocket-H5-RDP/VNC remote desktop client"
**Status**: Client implementation

**Conclusion**: Go implementations are primarily clients, not servers

---

### Python Implementations

#### 1. rdpy (citronneur)

**Repository**: https://github.com/citronneur/rdpy
**Description**: "Remote Desktop Protocol in Twisted Python"
**Status**: Checking if has server-side encoding...

**Need to analyze**: Whether it has AVC444 encoding implementation

---

### Rust Implementations

#### 1. IronRDP (Devolutions)

**Repository**: https://github.com/Devolutions/IronRDP
**Stars**: 2,797
**Status**: **This is what we're USING!**

**Note**: Protocol library, not full server. We're building the server on top.

#### 2. rdp-rs (citronneur)

**Repository**: https://github.com/citronneur/rdp-rs
**Stars**: 250
**Description**: "Remote Desktop Protocol in RUST"
**Status**: Checking...

#### 3. rustdesk

**Repository**: https://github.com/rustdesk/rustdesk
**Stars**: 104,891
**Description**: "Alternative to TeamViewer"
**Status**: **Massive project - might have encoding insights**

**Action**: Check if they use H.264 encoding and how

---

## INITIAL FINDINGS SUMMARY

### Pattern from FreeRDP (The Only Working Server Reference So Far)

**Architecture**:
- ✅ ONE encoder instance (confirmed)
- ✅ Sequential calls for luma then chroma
- ✅ Change detection for EACH stream independently
- ✅ LC field properly implemented (0=both, 1=luma, 2=chroma)

**Bandwidth Optimization Strategy**:
- **Primary**: Aux omission when unchanged (LC=1)
- **Not visible**: Whether aux uses P-frames or IDR

**What FreeRDP Does NOT show us**:
- Actual encoder configuration (hidden in subsystem)
- Frame type decisions (P vs IDR) for aux
- Whether they even care about aux P-frames

---

## CRITICAL REALIZATION from FreeRDP Code

**The bandwidth win** in their implementation comes from:

```c
// If chroma hasn't changed:
*op = 1;  // LC=1 (luma only)
// Don't even call Compress() for chroma!
// Skip encoding entirely
```

**This is EXACTLY what the other session told me!**

"Don't encode what you don't send" - FreeRDP literally doesn't encode aux when it hasn't changed!

**This means**:
1. Most frames: Encode only Main (luma) → ~75KB
2. Occasional frames: Encode both Main + Aux → ~145KB
3. Average: Probably ~1-2 MB/s depending on change rate

**Whether aux uses P-frames when it IS encoded: STILL UNKNOWN from this code**

---

## NEXT RESEARCH STEPS

### Immediate (Next 30 minutes)

1. **Check rustdesk** (massive Rust project, might have H.264 insights)
2. **Analyze GNOME's VA-API** encoder to see frame type behavior
3. **Check rdpy** (Python) for any encoding implementation

### Deep Dive (Next 2 hours)

4. **OpenH264 source analysis** - Find IDR insertion logic
5. **Find FreeRDP's actual encoder subsystem** - What does Compress() do?

### Documentation (Ongoing)

Writing findings as I discover them (this document)

---

**Progress**: 30% through comprehensive search
**Key Discovery**: FreeRDP confirms "don't encode what you don't send" pattern
**Still Unknown**: Whether aux P-frames exist in ANY implementation
**Status**: Continuing research...
