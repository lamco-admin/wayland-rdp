# ZGFX Compression Missing - Definitive Root Cause

**Date:** 2025-12-24
**Status:** ROOT CAUSE IDENTIFIED
**Priority:** CRITICAL BLOCKER

---

## The Definitive Answer (From Windows Client Logs)

**Error Event:**
```
Warning: RDPClient_GFX: An error was encountered when transitioning from
GfxStateDecodingRdpGfxPdu to GfxStateError in response to
GfxEventDecodingBulkCompressorFailed (error code 0x80004005).
```

**Translation:**
The Windows RDP client is **failing to decompress EGFX PDUs** because we're sending them **uncompressed** when MS-RDPEGFX specification requires **ZGFX compression**.

---

## What We Were Wrong About

### ❌ H.264 Level Constraints
- **Hypothesis:** Level 3.2 violation (120,000 > 108,000 MB/s) causes client rejection
- **Reality:** Client never gets to H.264 decoding - fails during EGFX PDU decompression

### ❌ RFX_RECT Encoding
- **Hypothesis:** width/height vs bounds format causes crash
- **Reality:** Bounds format was always correct, not the issue

### ❌ SPS Parameters
- **Hypothesis:** Baseline Profile Level 3.2 incompatible with Windows MFT decoder
- **Reality:** Decoder never initializes - fails before that

### ❌ Surface PDU Structure
- **Hypothesis:** ResetGraphics/CreateSurface/MapSurfaceToOutput have invalid parameters
- **Reality:** PDUs are correct, just not compressed

---

## What We Got Right

### ✅ Comprehensive Analysis
Our analysis document **correctly identified ZGFX compression as missing**:

From `EGFX-H264-COMPREHENSIVE-STATUS-2025-12-24.md`:
> **Question 2: ZGFX Compression - Is It Implemented?**
>
> **Answer: NO - Critical Missing Feature!**
> - IronRDP has `Decompressor` (client) but NO `Compressor` (server)
> - **MS-RDPEGFX requires ZGFX compression**
> - Priority: High - **spec compliance issue**

We identified it correctly but thought it was an optimization, not the blocker!

### ✅ All PDU Structures
- CapabilitiesConfirm: Sent correctly ✅
- ResetGraphics: 340 bytes, correct structure ✅
- CreateSurface: 15 bytes, correct structure ✅
- MapSurfaceToOutput: 20 bytes, correct structure ✅
- RFX_RECT bounds: Correct (1279, 799) ✅
- H.264 NAL structure: Valid AVC format ✅

**Everything is correct EXCEPT missing compression!**

---

## The Issue

### MS-RDPEGFX Protocol Flow (Required)

**Server Side:**
```
EGFX PDU → ZGFX Compress → Segmented Data → DVC message → SVC → Wire
```

**Client Side:**
```
Wire → SVC → DVC → ZGFX Decompress → EGFX PDU → Process/Decode
```

### What We're Actually Doing

**Server Side:**
```
EGFX PDU → (no compression!) → DVC message → SVC → Wire
```

**Client Side:**
```
Wire → SVC → DVC → ZGFX Decompress (FAILS!) → Error
                    ↑
                    Tries to decompress uncompressed data!
```

### Why Client Fails

The client receives raw EGFX PDU bytes where it expects ZGFX-compressed data:
- Tries to parse ZGFX segment descriptor (not present)
- Tries to decompress using ZGFX algorithm (data isn't compressed)
- **Decompression fails immediately**
- Transitions to error state and disconnects

---

## Implementation Requirements

### 1. ZGFX Compressor (ironrdp-graphics)

**Current Status:**
- File: `/home/greg/wayland/IronRDP/crates/ironrdp-graphics/src/zgfx/mod.rs`
- Has: `pub struct Decompressor` ✅
- Missing: `pub struct Compressor` ❌

**Required Implementation:**
```rust
pub struct Compressor {
    history: FixedCircularBuffer,
}

impl Compressor {
    pub fn new() -> Self {
        Self {
            history: FixedCircularBuffer::new(HISTORY_SIZE),
        }
    }

    pub fn compress(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<usize, ZgfxError> {
        // Implement ZGFX compression algorithm
        // Mirror of decompress logic
    }
}
```

**Algorithm (from MS-RDPEGFX 2.2.1.1.1):**
- Segment data if > MAX_SIZE
- Apply LZ77-style compression with history buffer
- Encode tokens (literals, matches, copies)
- Add segment descriptor and metadata

### 2. Integration in ironrdp-server

**File:** `/home/greg/wayland/IronRDP/crates/ironrdp-server/src/server.rs`

**Current Code (lines 644-649):**
```rust
ServerEvent::Egfx(EgfxServerMessage::SendMessages { channel_id, messages }) => {
    let data = server_encode_svc_messages(messages, drdynvc_channel_id, user_channel_id)?;
    writer.write_all(&data).await?;
    // ❌ NO COMPRESSION!
}
```

**Required Code:**
```rust
ServerEvent::Egfx(EgfxServerMessage::SendMessages { channel_id, messages }) => {
    // Encode EGFX PDUs to bytes
    let mut uncompressed = Vec::new();
    for msg in messages {
        msg.encode(&mut uncompressed)?;
    }

    // Compress with ZGFX
    let mut compressed = Vec::new();
    self.zgfx_compressor.compress(&uncompressed, &mut compressed)?;

    // Wrap in DVC/SVC and send
    let data = server_encode_svc_messages(vec![compressed], drdynvc_channel_id, user_channel_id)?;
    writer.write_all(&data).await?;
}
```

---

## Timeline

### Why This Took So Long to Find

1. **Error 0x1108** from mstsc is generic - no details
2. **Crash timing** suggested H.264 issue (happened during frame transmission)
3. **Level constraints** were real but not the blocker
4. **Windows operational log** was the missing piece

### Investigation Path

| Step | What We Checked | Result |
|------|-----------------|--------|
| 1 | RFX_RECT bounds format | ✅ Correct (always was) |
| 2 | PDU sequence and structure | ✅ All correct |
| 3 | CapabilitiesConfirm transmission | ✅ Sent to wire |
| 4 | H.264 level constraints | ✅ Identified but not blocker |
| 5 | SPS/PPS parameters | ✅ Valid |
| 6 | **Windows operational log** | **✅ ZGFX decompression failure!** |

---

## Action Plan

### Phase 1: Implement ZGFX Compressor (IronRDP PR)

**Tasks:**
1. Study ZGFX Decompressor code (mirror the logic)
2. Study FreeRDP zgfx_compress_to_stream() implementation
3. Implement Compressor struct and compress() method
4. Add unit tests (compress/decompress round-trip)
5. File PR to IronRDP upstream

**Estimated Effort:** 4-8 hours

### Phase 2: Integrate Compression

**Tasks:**
1. Add ZGFX Compressor instance to Server struct
2. Apply compression in ServerEvent::Egfx handling
3. Handle segmentation for large PDUs
4. Add compression ratio logging

**Estimated Effort:** 2-4 hours

### Phase 3: Test and Validate

**Tasks:**
1. Test with Windows mstsc client
2. Verify connection stays stable
3. Verify frame ACKs received
4. Check compression ratios
5. Test with FreeRDP client (cross-validation)

**Estimated Effort:** 1-2 hours

---

## Expected Outcome

With ZGFX compression implemented:
1. ✅ Windows client successfully decompresses EGFX PDUs
2. ✅ CapabilitiesConfirm processed
3. ✅ Surface PDUs (ResetGraphics, CreateSurface, MapSurfaceToOutput) processed
4. ✅ H.264 frames decoded
5. ✅ FRAME_ACKNOWLEDGE PDUs received
6. ✅ Video streaming works!

All our other analysis (H.264 levels, damage tracking, optimization strategies) becomes relevant AFTER this is fixed.

---

## Lessons Learned

1. **Always get client-side logs** - Error codes alone are insufficient
2. **Spec violations matter** - "Missing compression" seemed like optimization but was critical
3. **Comprehensive analysis pays off** - We identified ZGFX as missing in our docs
4. **Timing can be misleading** - Crash during Frame 1 didn't mean H.264 was the issue

---

## Next Session

1. Implement ZGFX Compressor (research + code)
2. Integrate in server event loop
3. Test with Windows client
4. Verify frame ACKs appear
5. **THEN** build comprehensive roadmap for optimizations

---

## Documents to Update

1. **EGFX-INVESTIGATION-SUMMARY-2025-12-24.md** - Add "SOLVED" status with ZGFX root cause
2. **EGFX-H264-COMPREHENSIVE-STATUS-2025-12-24.md** - Elevate ZGFX from "optimization" to "critical blocker"
3. Create **ZGFX-IMPLEMENTATION-PLAN.md** for the compression work

---

## Conclusion

After comprehensive server-side verification, RFX_RECT fixes, H.264 level analysis, and extensive logging, the Windows operational log gave us the answer in one line:

**"GfxEventDecodingBulkCompressorFailed"**

The entire issue is **missing ZGFX compression**. No H.264 level configuration needed (yet). No SPS parameter fixes needed. Just implement ZGFX Compressor and integrate it.

This unblocks everything.
