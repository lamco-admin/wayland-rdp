# ZGFX Compression - Comprehensive Implementation Analysis

**Date:** 2025-12-24
**Priority:** P0 - CRITICAL BLOCKER
**Status:** Design Phase
**Estimated Effort:** 8-16 hours implementation + 4-8 hours testing

---

## Executive Summary

Windows operational log confirmed: **`GfxEventDecodingBulkCompressorFailed`** - The client cannot decompress EGFX PDUs because we're not sending ZGFX-wrapped data. This is THE root cause of all connection failures.

---

## MS-RDPEGFX Specification Requirements

### Section 2.2.1.1 - RDP Graphics Pipeline Packet Structure

**Required Structure:**
```
ZGFX Compressed Packet:
  Descriptor (1 byte): 0xE0 (single) or 0xE1 (multipart)

  If Single (0xE0):
    CompressionFlags (1 byte): 0x04 (RDP8 type) | 0x02 (if compressed)
    Data: ZGFX-compressed or raw EGFX PDU bytes

  If Multipart (0xE1):
    SegmentCount (2 bytes LE)
    UncompressedSize (4 bytes LE)
    For each segment:
      Size (4 bytes LE)
      CompressionFlags (1 byte)
      Data: ZGFX-compressed or raw segment
```

**Compression Flag Bits:**
- `0x04` - RDP8 compression type (required)
- `0x02` - COMPRESSED flag (set if data is actually compressed)

**If COMPRESSED flag is SET:**
- Data is ZGFX-compressed using token-based encoding
- Client must decompress using ZGFX algorithm

**If COMPRESSED flag is NOT set:**
- Data is raw/uncompressed
- Client uses data directly (no decompression)

### Section 2.2.1.1.1 - ZGFX Compression Algorithm

**Algorithm Type:** LZ77-variant with Huffman-style token encoding

**Key Features:**
- 2.5MB history buffer (circular)
- Variable-length prefix codes for tokens
- Three token types:
  1. **Literals** - Direct byte values (commonly used bytes have short codes)
  2. **Matches** - Back-references (distance + length from history)
  3. **Null Literal** - Arbitrary byte with prefix "0" + 8 bits

**Token Table:** 40 predefined tokens with varying prefix lengths (1-9 bits)

**Limits:**
- Max uncompressed bytes per segment: 65,535
- Max history: 2,500,000 bytes
- Max segments: 65,535
- Min match length: 3 bytes

---

## IronRDP Current Implementation Analysis

### What Exists: Decompressor ✅

**File:** `/home/greg/wayland/IronRDP/crates/ironrdp-graphics/src/zgfx/mod.rs`

**Components:**
1. **`FixedCircularBuffer`** - 2.5MB history buffer with read_with_offset()
2. **`SegmentedDataPdu`** - Parses descriptor and segments
3. **`BulkEncodedData`** - Handles flags and compression type
4. **`Decompressor`** - Main decompression logic
5. **`TOKEN_TABLE`** - All 40 tokens with prefixes
6. **Comprehensive tests** - 10 test cases with binary test assets

**Algorithm Implementation:**
- ✅ Token prefix matching
- ✅ Literal byte output
- ✅ Match handling (distance + length)
- ✅ Unencoded block handling (distance=0 special case)
- ✅ Bit-level parsing with padding
- ✅ History buffer management

**Quality:** Well-implemented, thoroughly tested, production-ready

### What's Missing: Compressor ❌

**Required:**
1. **`Compressor` struct** with history buffer
2. **`compress()` method** - Mirror of decompress()
3. **Encoding logic:**
   - Find matches in history buffer
   - Select optimal tokens (literals vs matches)
   - Encode with prefix codes
   - Handle bit-level packing
   - Add padding byte
4. **Segmentation** for large data (>65KB)
5. **Tests** - Round-trip compress/decompress validation

---

## FreeRDP Implementation Analysis

**File:** `/tmp/freerdp-clone/libfreerdp/codec/zgfx.c`

**Key Discovery:** FreeRDP's `zgfx_compress_segment()` is **NOT IMPLEMENTED!**

```c
static BOOL zgfx_compress_segment(...) {
    /* FIXME: Currently compression not implemented. Just copy the raw source */
    (*pFlags) |= ZGFX_PACKET_COMPR_TYPE_RDP8;
    Stream_Write_UINT8(s, *pFlags); /* header (1 byte) */
    Stream_Write(s, pSrcData, SrcSize);
    return TRUE;
}
```

**What FreeRDP Does:**
1. Writes descriptor (0xE0 or 0xE1)
2. Writes flags byte (0x04 = RDP8 type, NO 0x02 COMPRESSED flag)
3. Writes raw uncompressed data
4. **This is spec-compliant!** (uncompressed data is allowed)

**Why FreeRDP Works with Windows:**
- Properly wraps data in ZGFX segment structure
- Sets correct flags (0x04 without 0x02 = uncompressed)
- Windows client sees "not compressed" flag and uses data directly

**Why Our Implementation Fails:**
- We send raw SVC/DVC bytes with NO ZGFX wrapper at all
- Windows client expects ZGFX segment structure
- Tries to parse descriptor (0xE0/0xE1) but gets raw data
- Decompression fails immediately

---

## Root Cause: Missing ZGFX Wrapper

**What We're Sending:**
```
DVC Data PDU {
    channel_id: 2
    payload: [raw EGFX PDU bytes]  ← NO ZGFX WRAPPER!
}
```

**What Windows Expects:**
```
DVC Data PDU {
    channel_id: 2
    payload: [
        descriptor: 0xE0,
        flags: 0x04,
        data: [EGFX PDU bytes]  ← Wrapped in ZGFX segment!
    ]
}
```

---

## Implementation Options

### Option A: Uncompressed ZGFX Wrapper (Quick Fix - 2-4 hours)

**Approach:** Match FreeRDP's current implementation

**Implementation:**
```rust
pub struct ZgfxWrapper;

impl ZgfxWrapper {
    /// Wrap data in ZGFX segment structure (uncompressed)
    pub fn wrap_uncompressed(data: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(data.len() + 2);

        if data.len() <= 65535 {
            // Single segment
            output.push(0xE0);  // ZGFX_SEGMENTED_SINGLE
            output.push(0x04);  // RDP8 type, not compressed
            output.extend_from_slice(data);
        } else {
            // Multipart (segment into 65KB chunks)
            output.push(0xE1);  // ZGFX_SEGMENTED_MULTIPART

            let segment_count = (data.len() + 65534) / 65535;
            output.extend_from_slice(&(segment_count as u16).to_le_bytes());
            output.extend_from_slice(&(data.len() as u32).to_le_bytes());

            for chunk in data.chunks(65535) {
                output.extend_from_slice(&((chunk.len() + 1) as u32).to_le_bytes());
                output.push(0x04);  // RDP8 type, not compressed
                output.extend_from_slice(chunk);
            }
        }

        output
    }
}
```

**Pros:**
- Simple, quick to implement
- Matches FreeRDP behavior
- Spec-compliant
- **Unblocks development immediately**

**Cons:**
- No actual compression (bandwidth not optimized)
- Still need full compressor eventually

**Testing:**
- Windows client should accept wrapped data
- Connection should stay stable
- Frame ACKs should flow

### Option B: Full ZGFX Compressor (Proper Solution - 8-16 hours)

**Approach:** Implement actual ZGFX compression algorithm

**Implementation Strategy:**

#### 1. Match Finding (LZ77-style)

```rust
struct MatchFinder {
    history: FixedCircularBuffer,
}

impl MatchFinder {
    fn find_best_match(&self, input: &[u8], pos: usize) -> Option<Match> {
        // Search history buffer for longest match
        // Min match length: 3 bytes
        // Max distance: current history size
        // Return: (distance, length) or None if no good match
    }

    fn should_encode_as_match(&self, match_len: usize) -> bool {
        // Match is worth encoding if compressed size < literal size
        // Depends on token prefix length for distance
        match_len >= 3  // Minimum benefit threshold
    }
}

struct Match {
    distance: usize,
    length: usize,
}
```

#### 2. Token Selection

```rust
enum EncodingChoice {
    Literal(u8),
    Match { distance: usize, length: usize },
    UnencodeBlock(Vec<u8>),  // For uncompressible data
}

fn select_encoding(byte: u8, best_match: Option<Match>) -> EncodingChoice {
    if let Some(m) = best_match {
        if m.length >= 3 {
            return EncodingChoice::Match {
                distance: m.distance,
                length: m.length,
            };
        }
    }

    // Check if byte has a short literal token
    if has_literal_token(byte) {
        EncodingChoice::Literal(byte)
    } else {
        EncodingChoice::Literal(byte)  // Use null literal (0 + 8 bits)
    }
}
```

#### 3. Bit-Level Encoding

```rust
struct BitWriter {
    bytes: Vec<u8>,
    current_byte: u8,
    bits_in_current: usize,
}

impl BitWriter {
    fn write_bits(&mut self, value: u32, num_bits: usize) {
        // Pack bits MSB-first (Msb0 order)
        // Handle byte boundaries
        // Track unused bits in last byte
    }

    fn finish(mut self) -> Vec<u8> {
        // Add padding bits indicator as last byte
        let unused_bits = if self.bits_in_current == 0 {
            0
        } else {
            8 - self.bits_in_current
        };

        if self.bits_in_current > 0 {
            self.bytes.push(self.current_byte);
        }
        self.bytes.push(unused_bits as u8);

        self.bytes
    }
}
```

#### 4. Main Compressor

```rust
pub struct Compressor {
    history: FixedCircularBuffer,
    match_finder: MatchFinder,
}

impl Compressor {
    pub fn new() -> Self {
        Self {
            history: FixedCircularBuffer::new(HISTORY_SIZE),
            match_finder: MatchFinder::new(),
        }
    }

    pub fn compress(&mut self, input: &[u8]) -> Result<Vec<u8>, ZgfxError> {
        let mut bit_writer = BitWriter::new();
        let mut pos = 0;

        while pos < input.len() {
            let best_match = self.match_finder.find_best_match(input, pos);

            match self.select_encoding(input[pos], best_match) {
                EncodingChoice::Literal(byte) => {
                    self.encode_literal(&mut bit_writer, byte);
                    self.history.write_u8(byte)?;
                    pos += 1;
                }
                EncodingChoice::Match { distance, length } => {
                    self.encode_match(&mut bit_writer, distance, length);
                    self.history.write_all(&input[pos..pos + length])?;
                    pos += length;
                }
                EncodingChoice::UnencodeBlock(data) => {
                    self.encode_unencoded_block(&mut bit_writer, &data);
                    self.history.write_all(&data)?;
                    pos += data.len();
                }
            }
        }

        Ok(bit_writer.finish())
    }

    fn encode_literal(&self, writer: &mut BitWriter, byte: u8) {
        if let Some(token) = find_literal_token(byte) {
            writer.write_bits(token.prefix_code, token.prefix_length);
        } else {
            // Null literal: prefix "0" + 8 bits
            writer.write_bits(0, 1);
            writer.write_bits(byte as u32, 8);
        }
    }

    fn encode_match(&self, writer: &mut BitWriter, distance: usize, length: usize) {
        // Find token for this distance
        let token = find_match_token(distance);
        writer.write_bits(token.prefix_code, token.prefix_length);
        writer.write_bits((distance - token.distance_base) as u32, token.distance_value_size);

        // Encode length
        self.encode_match_length(writer, length);
    }

    fn encode_match_length(&self, writer: &mut BitWriter, length: usize) {
        if length == 3 {
            // Special case: "0" bit
            writer.write_bits(0, 1);
        } else {
            // length = base + value, where base = 2^(token_size+1)
            // Determine token_size from length
            let token_size = ((length as f32).log2().floor() as usize).saturating_sub(1);
            let base = 2usize.pow((token_size + 1) as u32);
            let value = length - base;

            // Write token_size "1" bits + "0" bit
            writer.write_bits((1 << (token_size + 1)) - 1, token_size + 1);
            // Write value
            writer.write_bits(value as u32, token_size + 1);
        }
    }
}
```

**Pros:**
- Full compression (2-10x bandwidth savings)
- Optimal for production
- Reusable for all EGFX traffic

**Cons:**
- Complex implementation
- Need thorough testing
- Compression ratio depends on data type (H.264 ~1.2-2x, bitmaps ~3-10x)

### Option C: Hybrid Approach (RECOMMENDED - 4-8 hours)

**Start with Option A (uncompressed wrapper), upgrade to Option B later**

**Phase 1:** Implement uncompressed wrapper (immediate unblocking)
**Phase 2:** Implement full compressor (optimization)

**Benefits:**
- Fast path to working EGFX
- Can test H.264 streaming immediately
- Compression becomes optimization not blocker
- Can prioritize other features (damage tracking, levels)

---

## Detailed Design: Option A (Immediate Implementation)

### File Structure

```
ironrdp-graphics/src/zgfx/
  mod.rs           - Public API (Decompressor + Compressor)
  wrapper.rs       - NEW: Uncompressed wrapper
  compressor.rs    - NEW: (Future) Full compression
  circular_buffer.rs
  control_messages.rs
```

### wrapper.rs Implementation

```rust
//! ZGFX Uncompressed Wrapper
//!
//! Wraps data in ZGFX segment structure without actual compression.
//! This is spec-compliant and allows clients to process EGFX PDUs.
//!
//! For actual compression, use the full Compressor implementation.

use byteorder::{LittleEndian, WriteBytesExt};

const ZGFX_SEGMENTED_SINGLE: u8 = 0xE0;
const ZGFX_SEGMENTED_MULTIPART: u8 = 0xE1;
const ZGFX_PACKET_COMPR_TYPE_RDP8: u8 = 0x04;
const ZGFX_PACKET_COMPRESSED: u8 = 0x02;
const ZGFX_SEGMENTED_MAXSIZE: usize = 65535;

/// Wrap data in ZGFX segment structure (uncompressed)
///
/// This creates a spec-compliant ZGFX packet that clients can process,
/// but doesn't actually compress the data. The COMPRESSED flag (0x02)
/// is NOT set, indicating to the client to use the data directly.
///
/// # Arguments
///
/// * `data` - Raw data to wrap (typically EGFX PDU bytes)
///
/// # Returns
///
/// ZGFX-wrapped data ready for transmission
pub fn wrap_uncompressed(data: &[u8]) -> Vec<u8> {
    if data.len() <= ZGFX_SEGMENTED_MAXSIZE {
        wrap_single_segment(data)
    } else {
        wrap_multipart_segments(data)
    }
}

fn wrap_single_segment(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() + 2);

    // Descriptor
    output.push(ZGFX_SEGMENTED_SINGLE);

    // Flags: RDP8 type, not compressed
    output.push(ZGFX_PACKET_COMPR_TYPE_RDP8);

    // Raw data
    output.extend_from_slice(data);

    output
}

fn wrap_multipart_segments(data: &[u8]) -> Vec<u8> {
    let segments: Vec<&[u8]> = data.chunks(ZGFX_SEGMENTED_MAXSIZE).collect();
    let segment_count = segments.len();

    // Estimate size: descriptor(1) + count(2) + uncompressed_size(4) +
    //                segments * (size(4) + flags(1)) + data
    let mut output = Vec::with_capacity(data.len() + 7 + segment_count * 5);

    // Descriptor
    output.push(ZGFX_SEGMENTED_MULTIPART);

    // Segment count (LE u16)
    output.write_u16::<LittleEndian>(segment_count as u16).unwrap();

    // Total uncompressed size (LE u32)
    output.write_u32::<LittleEndian>(data.len() as u32).unwrap();

    // Each segment
    for segment in segments {
        // Segment size (includes flags byte)
        output.write_u32::<LittleEndian>((segment.len() + 1) as u32).unwrap();

        // Flags: RDP8 type, not compressed
        output.push(ZGFX_PACKET_COMPR_TYPE_RDP8);

        // Raw segment data
        output.extend_from_slice(segment);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_small_data() {
        let data = b"Hello, ZGFX!";
        let wrapped = wrap_uncompressed(data);

        // Should be: descriptor(1) + flags(1) + data
        assert_eq!(wrapped.len(), data.len() + 2);
        assert_eq!(wrapped[0], 0xE0);  // Single segment
        assert_eq!(wrapped[1], 0x04);  // RDP8, not compressed
        assert_eq!(&wrapped[2..], data);
    }

    #[test]
    fn test_wrap_large_data() {
        let data = vec![0xAB; 100000];  // 100KB > 65KB limit
        let wrapped = wrap_uncompressed(&data);

        assert_eq!(wrapped[0], 0xE1);  // Multipart

        // Verify structure
        let segment_count = u16::from_le_bytes([wrapped[1], wrapped[2]]) as usize;
        assert_eq!(segment_count, 2);  // 100KB / 65KB = 2 segments

        let uncompressed_size = u32::from_le_bytes([
            wrapped[3], wrapped[4], wrapped[5], wrapped[6]
        ]) as usize;
        assert_eq!(uncompressed_size, 100000);
    }

    #[test]
    fn test_round_trip_unwrap() {
        let data = b"Test data for ZGFX round-trip";
        let wrapped = wrap_uncompressed(data);

        // Verify decompressor can handle it
        use super::super::Decompressor;
        let mut decompressor = Decompressor::new();
        let mut output = Vec::new();
        decompressor.decompress(&wrapped, &mut output).unwrap();

        assert_eq!(&output, data);
    }
}
```

### Integration in ironrdp-server

**File:** `ironrdp-server/src/server.rs`

**Current Code:**
```rust
ServerEvent::Egfx(EgfxServerMessage::SendMessages { channel_id, messages }) => {
    let data = server_encode_svc_messages(messages, drdynvc_channel_id, user_channel_id)?;
    writer.write_all(&data).await?;
}
```

**Problem:** `messages` are already encoded as SvcMessage types with DVC wrapping. We need to wrap BEFORE DVC encoding.

**Solution:** Need to access raw EGFX PDU bytes before DVC encoding.

**Architecture Question:** Where in the pipeline should ZGFX wrapping happen?

**Option 1:** In EGFX crate before creating DvcMessage
**Option 2:** In server.rs by unwrapping SvcMessage, wrapping, re-encoding
**Option 3:** New message type that carries unwrapped EGFX PDUs

Let me examine how GfxPdu becomes DvcMessage...

---

## Architecture Deep Dive

### Current Flow

```
wrd-server-specs:
  display_handler.rs:
    encode_dvc_messages(channel_id, dvc_messages, flags) → Vec<SvcMessage>
    Send via ServerEvent::Egfx

IronRDP:
  ironrdp-egfx/src/server.rs:
    drain_output() → Vec<DvcMessage>
      // DvcMessage is Box<dyn DvcEncode>
      // GfxPdu implements DvcEncode

  ironrdp-dvc/src/lib.rs:
    encode_dvc_messages(channel_id, messages, flags) → Vec<SvcMessage>
      // For each DvcMessage:
      //   - Encode to bytes
      //   - Wrap in DVC Data PDU
      //   - Encode to SvcMessage
```

**Key Issue:** GfxPdu is encoded directly to bytes and wrapped in DVC Data PDU. There's NO ZGFX wrapping step!

### Required Architecture

```
GfxPdu → [encode to bytes] → ZGFX wrap → DVC Data PDU → SVC → Wire
            ^                    ^
            |                    |
        ironrdp-pdu          ironrdp-graphics
```

**Problem:** DvcMessage trait abstracts the encoding - we can't intercept the bytes.

**Solutions:**

**A. Add ZGFX wrapper in GfxPdu encoding itself**
- Modify ironrdp-egfx to wrap GfxPdu bytes in ZGFX before returning from DvcEncode
- Pro: Clean separation
- Con: Couples EGFX to ZGFX

**B. Special handling in encode_dvc_messages for EGFX**
- Detect DvcMessage is GfxPdu
- Encode, wrap with ZGFX, re-wrap in DVC
- Pro: Keeps ZGFX in graphics crate
- Con: Special-case logic

**C. New EgfxMessage type that wraps GfxPdu with ZGFX**
- Create wrapper type that handles ZGFX wrapping
- Implements DvcEncode
- Pro: Clean, reusable
- Con: Extra abstraction layer

---

## Immediate Implementation Plan (Option A Wrapper)

### Phase 1: Implement ZGFX Wrapper (2-4 hours)

**Tasks:**
1. Create `ironrdp-graphics/src/zgfx/wrapper.rs`
2. Implement `wrap_uncompressed()` function
3. Add comprehensive tests (single/multipart, round-trip)
4. Export from zgfx module

**Deliverable:** `zgfx::wrap_uncompressed()` function ready to use

### Phase 2: Integrate in ironrdp-egfx (2-3 hours)

**Approach C: EgfxMessageWrapper**

**File:** `ironrdp-egfx/src/server.rs`

```rust
use ironrdp_graphics::zgfx;

/// Wrapper that applies ZGFX encoding to GfxPdu
struct ZgfxWrappedGfxPdu {
    inner: GfxPdu,
}

impl DvcEncode for ZgfxWrappedGfxPdu {
    fn encode(&self) -> EncodeResult<Vec<u8>> {
        // 1. Encode inner GfxPdu to bytes
        let mut gfx_bytes = Vec::new();
        let mut cursor = WriteCursor::new(&mut gfx_bytes);
        self.inner.encode(&mut cursor)?;

        // 2. Wrap with ZGFX (uncompressed for now)
        let zgfx_wrapped = zgfx::wrap_uncompressed(&gfx_bytes);

        Ok(zgfx_wrapped)
    }
}

// Modify drain_output to return wrapped messages
pub fn drain_output(&mut self) -> Vec<DvcMessage> {
    self.output_queue
        .drain(..)
        .map(|pdu| {
            Box::new(ZgfxWrappedGfxPdu { inner: pdu }) as DvcMessage
        })
        .collect()
}
```

**Testing:** Verify Windows client accepts wrapped PDUs

### Phase 3: Test and Validate (1-2 hours)

1. Build and deploy
2. Test with Windows mstsc
3. Verify operational log shows no bulk decompression errors
4. Verify frame ACKs received
5. Verify stable connection

---

## Future: Full ZGFX Compression (Phase 2)

### Implementation Timeline

**Week 1: Core Algorithm**
- Implement match finding
- Implement token encoding
- Implement bit writer
- Basic compress() method

**Week 2: Optimization**
- Tune match finding performance
- Optimize token selection
- Handle edge cases
- Benchmark compression ratios

**Week 3: Testing**
- Unit tests for all components
- Round-trip compress/decompress tests
- Performance benchmarks
- Integration testing

**Week 4: PR and Integration**
- File IronRDP PR
- Code review
- Merge upstream
- Update wrd-server-specs

### Compression Effectiveness

**Data Type Compression Ratios:**

| Content Type | Uncompressed | ZGFX Compressed | Ratio | Bandwidth Saved |
|--------------|--------------|-----------------|-------|-----------------|
| H.264 frames | 85 KB | 50-70 KB | 1.2-1.7x | 18-29% |
| Bitmap data | 4096 KB | 400-1000 KB | 4-10x | 75-90% |
| Text/UI | 100 KB | 10-30 KB | 3-10x | 70-90% |
| RemoteFX | 200 KB | 30-60 KB | 3-7x | 70-85% |

**H.264 Compression Characteristics:**
- Already compressed by video codec
- ZGFX compression modest (headers, metadata, some entropy)
- Still worthwhile for large frames (85KB → 50KB = 35KB saved)

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_wrap_uncompressed_small() {
    let data = b"Test";
    let wrapped = wrap_uncompressed(data);
    // Verify structure
}

#[test]
fn test_wrap_uncompressed_large() {
    let data = vec![0; 100000];
    let wrapped = wrap_uncompressed(&data);
    // Verify multipart segmentation
}

#[test]
fn test_round_trip() {
    let data = b"Round trip test data";
    let wrapped = wrap_uncompressed(data);

    let mut decompressor = Decompressor::new();
    let mut output = Vec::new();
    decompressor.decompress(&wrapped, &mut output).unwrap();

    assert_eq!(&output, data);
}
```

### Integration Tests

```rust
#[test]
fn test_egfx_with_zgfx_wrapper() {
    // Create GfxPdu (e.g., CapabilitiesConfirm)
    let pdu = GfxPdu::CapabilitiesConfirm(...);

    // Wrap and encode
    let wrapped = ZgfxWrappedGfxPdu { inner: pdu };
    let bytes = wrapped.encode().unwrap();

    // Verify can be decoded
    let mut decompressor = Decompressor::new();
    let mut output = Vec::new();
    decompressor.decompress(&bytes, &mut output).unwrap();

    // Decode should get back original GfxPdu
    let decoded_pdu = GfxPdu::decode(&output).unwrap();
    assert_eq!(decoded_pdu, pdu);
}
```

### End-to-End Tests

1. **Test with Windows mstsc:** Should connect without bulk decompression errors
2. **Test with FreeRDP client:** Cross-validation
3. **Monitor operational log:** No error event 226
4. **Verify frame ACKs:** Backpressure should clear
5. **Long-running test:** 5+ minute stable connection

---

## Performance Considerations

### Uncompressed Wrapper Overhead

**Overhead:** 2 bytes (single segment) or 7+ bytes (multipart)
- Single segment: +2 bytes (descriptor + flags)
- Multipart (100KB): +7 bytes header + 5 bytes per 65KB segment ≈ +17 bytes

**Impact:** Negligible (<0.02% for typical frame sizes)

### Full Compression Benefits

**85KB H.264 Frame:**
- Uncompressed: 85,000 bytes
- ZGFX compressed: ~50,000-70,000 bytes (est.)
- Savings: 15,000-35,000 bytes per frame
- At 30fps: 450KB-1MB/sec bandwidth saved

**Worth it?** Yes for production, but not required for initial functionality.

---

## Implementation Checklist

### Immediate (Option A - Uncompressed Wrapper)

- [ ] Create `ironrdp-graphics/src/zgfx/wrapper.rs`
- [ ] Implement `wrap_uncompressed()` with single/multipart
- [ ] Add unit tests (small data, large data, round-trip)
- [ ] Export from `ironrdp-graphics/src/zgfx/mod.rs`
- [ ] Create `ZgfxWrappedGfxPdu` in `ironrdp-egfx/src/server.rs`
- [ ] Modify `drain_output()` to wrap PDUs
- [ ] Test with Windows client
- [ ] Verify no bulk decompression errors
- [ ] Verify frame ACKs received
- [ ] Document in session handover

### Future (Option B - Full Compression)

- [ ] Design match finding algorithm
- [ ] Implement BitWriter for token encoding
- [ ] Implement token selection logic
- [ ] Implement Compressor struct
- [ ] Add comprehensive tests
- [ ] Benchmark compression ratios
- [ ] File IronRDP PR
- [ ] Integrate after merge

---

## Decision Matrix

| Factor | Option A (Wrapper) | Option B (Full Compressor) | Option C (Hybrid) |
|--------|-------------------|---------------------------|-------------------|
| **Time to Working** | 4-6 hours | 12-20 hours | 4-6 hours (Phase 1) |
| **Unblocks Development** | ✅ Yes | ✅ Yes | ✅ Yes |
| **Bandwidth Efficiency** | ❌ No improvement | ✅ 2-10x savings | ⚠️ Later |
| **Complexity** | ⭐ Low | ⭐⭐⭐ High | ⭐⭐ Medium |
| **IronRDP PR Needed** | ✅ Yes | ✅ Yes | ✅ Yes (both phases) |
| **Risk** | Low | Medium | Low |
| **Production Ready** | ✅ Functional | ✅ Optimal | ✅ Staged |

---

## Recommendation: Option C (Hybrid Approach)

**Phase 1 (Now):** Implement uncompressed wrapper
- Unblocks EGFX development immediately
- Low risk, quick implementation
- Spec-compliant

**Phase 2 (Next Sprint):** Implement full compression
- Optimize bandwidth
- Better production performance
- Can be done alongside other features

**Benefits:**
- Fast path to working H.264 streaming
- Can test all other features (levels, damage tracking)
- Compression becomes optimization, not blocker
- Can parallelize: One person on compression, another on features

---

## Next Steps

1. **Implement Option A wrapper** (4-6 hours focused work)
2. **Test with Windows client** (should work immediately)
3. **Verify frame ACKs flow**
4. **Build comprehensive roadmap** with ZGFX Phase 2 scheduled
5. **Parallelize:**
   - Track A: Full ZGFX compression implementation
   - Track B: H.264 level configuration, damage tracking, other features

**Target:** Working EGFX by end of session, roadmap for all features.

Ready to start implementation?
