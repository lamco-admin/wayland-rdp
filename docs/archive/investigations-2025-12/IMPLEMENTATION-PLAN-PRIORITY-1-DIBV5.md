# Implementation Plan: Priority 1 - DIB/DIBV5 Support

**Effort**: 11-15 hours
**Impact**: Complete clipboard image support with alpha channel
**Dependencies**: None (can start immediately)
**Files**: lamco-clipboard-core/src/image.rs, src/clipboard/manager.rs

---

## OVERVIEW

**Goal**: Add CF_DIBV5 (format 17) support for clipboard images with transparency

**Why**: Modern Windows apps use DIBV5 for alpha channel. Without it, transparency is lost.

**What**: Extend existing DIB implementation with DIBV5 variant (124-byte header vs 40-byte)

---

## PHASE 1: Core DIBV5 Functions (6-7 hours)

### Location: `../lamco-rdp-workspace/crates/lamco-clipboard-core/src/image.rs`

### Task 1.1: Add `create_dibv5_from_image()` (2 hours)

**Implementation**:

```rust
/// Create DIBV5 from DynamicImage
///
/// DIBV5 has a 124-byte BITMAPV5HEADER with alpha channel and color space support.
/// This creates an sRGB DIBV5 with standard BGRA color masks.
fn create_dibv5_from_image(image: &DynamicImage) -> ClipboardResult<Vec<u8>> {
    let rgba = image.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());

    let mut dib = BytesMut::new();

    // BITMAPV5HEADER structure (124 bytes total)
    dib.put_u32_le(124); // bV5Size
    dib.put_i32_le(width as i32); // bV5Width
    dib.put_i32_le(-(height as i32)); // bV5Height (negative = top-down)
    dib.put_u16_le(1); // bV5Planes
    dib.put_u16_le(32); // bV5BitCount (32-bit BGRA)
    dib.put_u32_le(3); // bV5Compression = BI_BITFIELDS

    let image_size = width * height * 4;
    dib.put_u32_le(image_size); // bV5SizeImage

    dib.put_i32_le(0); // bV5XPelsPerMeter
    dib.put_i32_le(0); // bV5YPelsPerMeter
    dib.put_u32_le(0); // bV5ClrUsed
    dib.put_u32_le(0); // bV5ClrImportant

    // Color channel masks (standard BGRA)
    dib.put_u32_le(0x00FF0000); // bV5RedMask (byte 2)
    dib.put_u32_le(0x0000FF00); // bV5GreenMask (byte 1)
    dib.put_u32_le(0x000000FF); // bV5BlueMask (byte 0)
    dib.put_u32_le(0xFF000000); // bV5AlphaMask (byte 3)

    // Color space: sRGB (0x73524742 = "sRGB" in ASCII)
    dib.put_u32_le(0x73524742); // bV5CSType = LCS_sRGB

    // CIEXYZTRIPLE endpoints (9 u32s = 36 bytes)
    // For sRGB, these can be zero
    for _ in 0..9 {
        dib.put_u32_le(0);
    }

    // Gamma values (0 = use sRGB defaults)
    dib.put_u32_le(0); // bV5GammaRed
    dib.put_u32_le(0); // bV5GammaGreen
    dib.put_u32_le(0); // bV5GammaBlue

    // Rendering intent: LCS_GM_IMAGES (2)
    dib.put_u32_le(2); // bV5Intent

    // ICC profile (not embedded)
    dib.put_u32_le(0); // bV5ProfileData
    dib.put_u32_le(0); // bV5ProfileSize
    dib.put_u32_le(0); // bV5Reserved

    // Pixel data (BGRA byte order)
    for pixel in rgba.pixels() {
        dib.put_u8(pixel[2]); // Blue
        dib.put_u8(pixel[1]); // Green
        dib.put_u8(pixel[0]); // Red
        dib.put_u8(pixel[3]); // Alpha
    }

    Ok(dib.to_vec())
}
```

**Testing**:
- Create 100×100 RGBA image with transparency
- Convert to DIBV5
- Verify header size = 124
- Verify masks correct
- Verify pixel data correct

---

### Task 1.2: Add `parse_dibv5_to_image()` (3 hours)

**Implementation**:

```rust
/// Parse DIBV5 data to DynamicImage
///
/// Handles both:
/// - Standard DIBV5 (124-byte header)
/// - "Short DIBV5" bug (40-byte header with format ID 17)
fn parse_dibv5_to_image(dibv5_data: &[u8]) -> ClipboardResult<DynamicImage> {
    if dibv5_data.len() < 4 {
        return Err(ClipboardError::ImageDecode("DIBV5 too small".to_string()));
    }

    // Read header size
    let header_size = u32::from_le_bytes([
        dibv5_data[0], dibv5_data[1], dibv5_data[2], dibv5_data[3]
    ]);

    // Handle "short DIBV5" bug (apps that use 40-byte header with format 17)
    if header_size == 40 {
        debug!("DIBV5 with 40-byte header detected - falling back to DIB parser");
        return parse_dib_to_image(dibv5_data);
    }

    if header_size != 124 {
        return Err(ClipboardError::ImageDecode(
            format!("Invalid DIBV5 header size: {} (expected 124)", header_size)
        ));
    }

    if dibv5_data.len() < 124 {
        return Err(ClipboardError::ImageDecode("DIBV5 data too small for header".to_string()));
    }

    // Parse dimensions
    let width = i32::from_le_bytes([dibv5_data[4..8]]).unsigned_abs();
    let height_raw = i32::from_le_bytes([dibv5_data[8..12]]);
    let height = height_raw.unsigned_abs();
    let top_down = height_raw < 0;

    // Parse bit depth and compression
    let bit_count = u16::from_le_bytes([dibv5_data[14..16]]);
    let compression = u32::from_le_bytes([dibv5_data[16..20]]);

    // Parse color masks (for BI_BITFIELDS compression)
    let (red_mask, green_mask, blue_mask, alpha_mask) = if compression == 3 {
        (
            u32::from_le_bytes([dibv5_data[40..44]]),
            u32::from_le_bytes([dibv5_data[44..48]]),
            u32::from_le_bytes([dibv5_data[48..52]]),
            u32::from_le_bytes([dibv5_data[52..56]]),
        )
    } else {
        // Default BGRA masks
        (0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000)
    };

    // Parse color space type
    let cs_type = u32::from_le_bytes([dibv5_data[56..60]]);

    // Only support sRGB for now
    if cs_type != 0x73524742 && cs_type != 0 {
        return Err(ClipboardError::ImageDecode(
            format!("Unsupported color space: 0x{:08x} (only sRGB supported)", cs_type)
        ));
    }

    // Pixel data starts after 124-byte header
    let pixel_data = &dibv5_data[124..];

    // Convert based on bit depth
    match bit_count {
        32 => convert_dibv5_32bit(pixel_data, width, height, top_down, &masks),
        24 => convert_dibv5_24bit(pixel_data, width, height, top_down),
        _ => Err(ClipboardError::ImageDecode(
            format!("Unsupported DIBV5 bit depth: {}", bit_count)
        )),
    }
}

/// Convert 32-bit DIBV5 pixel data with alpha
fn convert_dibv5_32bit(
    pixel_data: &[u8],
    width: u32,
    height: u32,
    top_down: bool,
    masks: &ColorMasks,
) -> ClipboardResult<DynamicImage> {
    // Similar to existing convert_32bit_dib but uses masks
    // Extract RGBA from BGRA using masks
    // Handle top-down vs bottom-up
    // Return RgbaImage
}
```

**Testing**:
- Parse DIBV5 from Paint.NET screenshot
- Verify dimensions
- Verify alpha channel preserved
- Test "short DIBV5" fallback
- Test different color space values

---

### Task 1.3: Add Public API Functions (1 hour)

```rust
/// Convert PNG to DIBV5 format
///
/// DIBV5 supports alpha channel and color space information.
/// This creates an sRGB DIBV5 with standard BGRA masks.
pub fn png_to_dibv5(png_data: &[u8]) -> ClipboardResult<Vec<u8>> {
    let image = image::load_from_memory_with_format(png_data, ImageFormat::Png)?;
    create_dibv5_from_image(&image)
}

/// Convert JPEG to DIBV5 format
pub fn jpeg_to_dibv5(jpeg_data: &[u8]) -> ClipboardResult<Vec<u8>> {
    let image = image::load_from_memory_with_format(jpeg_data, ImageFormat::Jpeg)?;
    create_dibv5_from_image(&image)
}

/// Convert DIBV5 to PNG format
///
/// Preserves alpha channel if present in DIBV5.
pub fn dibv5_to_png(dibv5_data: &[u8]) -> ClipboardResult<Vec<u8>> {
    let image = parse_dibv5_to_image(dibv5_data)?;

    let mut png_data = Vec::new();
    image.write_to(&mut std::io::Cursor::new(&mut png_data), ImageFormat::Png)?;
    Ok(png_data)
}

/// Convert DIBV5 to JPEG format
pub fn dibv5_to_jpeg(dibv5_data: &[u8]) -> ClipboardResult<Vec<u8>> {
    let image = parse_dibv5_to_image(dibv5_data)?;

    let mut jpeg_data = Vec::new();
    image.write_to(&mut std::io::Cursor::new(&mut jpeg_data), ImageFormat::Jpeg)?;
    Ok(jpeg_data)
}
```

---

## PHASE 2: Integration in Manager (2-3 hours)

### Location: `src/clipboard/manager.rs`

### Task 2.1: Add CF_DIBV5 to RDP → Portal conversion

**Around line 1150** in `handle_rdp_data_request()`:

```rust
} else if format_id == 17 {
    // CF_DIBV5 - Windows wants DIBV5, Portal has image format
    if mime_type.starts_with("image/png") {
        info!("🎨 Converting PNG to DIBV5 for Windows (with alpha)");
        lamco_clipboard_core::image::png_to_dibv5(&portal_data).map_err(|e| {
            error!("PNG to DIBV5 conversion failed: {}", e);
            ClipboardError::Core(e)
        })?
    } else if mime_type.starts_with("image/jpeg") {
        info!("🎨 Converting JPEG to DIBV5 for Windows");
        lamco_clipboard_core::image::jpeg_to_dibv5(&portal_data).map_err(|e| {
            error!("JPEG to DIBV5 conversion failed: {}", e);
            ClipboardError::Core(e)
        })?
    } else {
        // Unsupported MIME type for DIBV5
        error!("Cannot convert {} to DIBV5", mime_type);
        return Err(ClipboardError::UnsupportedFormat(mime_type.to_string()));
    }
}
```

### Task 2.2: Add Portal → CF_DIBV5 conversion

**Around line 1310** in `handle_portal_data_request()`:

```rust
// Auto-detect DIB vs DIBV5 based on header size
if requested_mime.starts_with("image/png") {
    if data.len() >= 4 {
        let header_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

        match header_size {
            124 => {
                // DIBV5 format
                info!("🎨 Converting DIBV5 to PNG for Portal (with alpha)");
                lamco_clipboard_core::image::dibv5_to_png(&data).map_err(|e| {
                    error!("DIBV5 to PNG conversion failed: {}", e);
                    ClipboardError::Core(e)
                })?
            }
            40 => {
                // Regular DIB format
                info!("🎨 Converting DIB to PNG for Portal");
                lamco_clipboard_core::image::dib_to_png(&data).map_err(|e| {
                    error!("DIB to PNG conversion failed: {}", e);
                    ClipboardError::Core(e)
                })?
            }
            _ => {
                return Err(ClipboardError::ImageDecode(
                    format!("Unknown DIB header size: {}", header_size)
                ));
            }
        }
    } else {
        return Err(ClipboardError::ImageDecode("Data too small for DIB header".to_string()));
    }
}
```

### Task 2.3: Announce Both DIB and DIBV5 to Windows

**When Portal has PNG with alpha**, announce both formats:

```rust
// In format announcement logic
if mime_type == "image/png" && has_alpha_channel(&data) {
    // Announce both DIBV5 and DIB for maximum compatibility
    rdp_formats.push(ClipboardFormat { id: 17, name: Some("CF_DIBV5".to_string()) });
    rdp_formats.push(ClipboardFormat { id: 8, name: Some("CF_DIB".to_string()) });
} else if mime_type.starts_with("image/") {
    // Opaque image - just DIB
    rdp_formats.push(ClipboardFormat { id: 8, name: Some("CF_DIB".to_string()) });
}
```

**Helper**:
```rust
fn has_alpha_channel(png_data: &[u8]) -> bool {
    // Quick check: load image and check if any pixel has alpha != 255
    if let Ok(img) = image::load_from_memory(png_data) {
        let rgba = img.to_rgba8();
        rgba.pixels().any(|p| p[3] != 255)
    } else {
        false
    }
}
```

---

## PHASE 3: Testing (2-3 hours)

### Test Suite

**Test 1: Windows → Linux (transparency preserved)**:
```
1. On Windows: Take screenshot with transparency
2. Copy to clipboard (Ctrl+C)
3. On Linux via RDP: Paste in GIMP
4. Verify: Alpha channel present, transparency correct
```

**Test 2: Linux → Windows (transparency preserved)**:
```
1. On Linux: Create PNG with transparency in GIMP
2. Copy to clipboard
3. On Windows via RDP: Paste in Paint.NET
4. Verify: Alpha channel present, transparency correct
```

**Test 3: Opaque images (backward compatibility)**:
```
1. Copy opaque JPEG
2. Verify: Still works via CF_DIB
3. Verify: No regression
```

**Test 4: Paint.NET compatibility**:
```
1. Create transparent image in Paint.NET
2. Copy (provides DIBV5 + PNG)
3. Paste in Linux
4. Verify: Works via either format
```

**Test 5: Old app compatibility**:
```
1. Old Windows app requests CF_DIB (not DIBV5)
2. Verify: Gets DIB format (no alpha)
3. Verify: Doesn't crash or fail
```

---

## PHASE 4: Publishing (1-2 hours)

### Update lamco-clipboard-core

**Version**: 0.4.0 → 0.5.0

**Changelog**:
```
## [0.5.0] - 2025-12-30

### Added
- CF_DIBV5 (format 17) support for alpha channel preservation
- png_to_dibv5() - Convert PNG to DIBV5 with alpha
- dibv5_to_png() - Convert DIBV5 to PNG preserving alpha
- jpeg_to_dibv5() - Convert JPEG to DIBV5
- dibv5_to_jpeg() - Convert DIBV5 to JPEG
- Automatic detection of DIB vs DIBV5 based on header size
- Support for "short DIBV5" compatibility (40-byte header bug)

### Changed
- Image clipboard operations now preserve transparency
- Both CF_DIB and CF_DIBV5 announced for maximum compatibility
```

**Publish**:
```bash
cd ../lamco-rdp-workspace/crates/lamco-clipboard-core
cargo publish
```

**Update lamco-rdp-server dependency**:
```toml
lamco-clipboard-core = { version = "0.5.0", features = ["image"] }
```

---

## SUCCESS CRITERIA

- [ ] Windows screenshot with transparency → Linux GIMP shows alpha ✅
- [ ] Linux GIMP transparent PNG → Windows Paint.NET shows alpha ✅
- [ ] Old Windows apps still work with CF_DIB ✅
- [ ] Paint.NET compatibility validated ✅
- [ ] No regressions in existing clipboard operations ✅

---

## RISKS AND MITIGATIONS

**Risk 1**: Color space handling complexity
- **Mitigation**: Only support sRGB initially, reject others gracefully

**Risk 2**: "Short DIBV5" compatibility
- **Mitigation**: Auto-detect header size, fall back to DIB parser

**Risk 3**: Performance impact
- **Mitigation**: Only use DIBV5 when alpha channel exists, use DIB for opaque

**Risk 4**: App compatibility bugs
- **Mitigation**: Announce both DIB and DIBV5, let Windows choose

---

## DELIVERABLES

1. **Code**: Updated lamco-clipboard-core/src/image.rs (~150 lines added)
2. **Code**: Updated src/clipboard/manager.rs (~30 lines added)
3. **Tests**: 5 validation scenarios documented
4. **Crate**: lamco-clipboard-core 0.5.0 published
5. **Documentation**: This implementation plan

---

**Estimated total**: 11-15 hours
**Priority**: #1 (complete clipboard support)
**Status**: Ready to implement
