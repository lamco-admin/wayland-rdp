//! Clipboard Format Conversion
//!
//! Provides comprehensive format conversion between RDP clipboard formats
//! and Linux/Wayland MIME types. Supports text, images, and file formats.

use crate::clipboard::error::{ClipboardError, Result};
use bytes::{BufMut, BytesMut};
use image::{DynamicImage, ImageFormat};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::collections::HashMap;

/// RDP Clipboard format IDs (Windows clipboard formats)
pub mod format_id {
    /// Text format (ANSI)
    pub const CF_TEXT: u32 = 1;
    /// Bitmap format
    pub const CF_BITMAP: u32 = 2;
    /// Metafile picture format
    pub const CF_METAFILEPICT: u32 = 3;
    /// Unicode text format
    pub const CF_UNICODETEXT: u32 = 13;
    /// Enhanced metafile format
    pub const CF_ENHMETAFILE: u32 = 14;
    /// File drop format
    pub const CF_HDROP: u32 = 15;
    /// Locale identifier
    pub const CF_LOCALE: u32 = 16;
    /// Device-independent bitmap
    pub const CF_DIB: u32 = 8;
    /// Palette format
    pub const CF_PALETTE: u32 = 9;
    /// Pen data
    pub const CF_PENDATA: u32 = 10;
    /// RIFF format
    pub const CF_RIFF: u32 = 11;
    /// Wave audio format
    pub const CF_WAVE: u32 = 12;
    /// HTML format
    pub const CF_HTML: u32 = 0xD010;
    /// PNG format
    pub const CF_PNG: u32 = 0xD011;
    /// JPEG format
    pub const CF_JPEG: u32 = 0xD012;
    /// GIF format
    pub const CF_GIF: u32 = 0xD013;
    /// Rich Text Format
    pub const CF_RTF: u32 = 0xD014;
    /// Start of custom formats
    pub const CF_CUSTOM_START: u32 = 0xC000;
}

use format_id::*;

/// Clipboard format descriptor
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardFormat {
    /// Format ID
    pub format_id: u32,
    /// Format name
    pub format_name: String,
}

/// Format converter handles conversion between RDP and MIME types
pub struct FormatConverter {
    /// RDP format ID to MIME type mapping
    format_map: HashMap<u32, String>,
    /// MIME type to RDP format ID mapping
    mime_map: HashMap<String, u32>,
}

impl FormatConverter {
    /// Create a new format converter
    pub fn new() -> Self {
        let mut converter = Self {
            format_map: HashMap::new(),
            mime_map: HashMap::new(),
        };

        converter.init_format_mappings();
        converter
    }

    /// Initialize format mapping tables
    fn init_format_mappings(&mut self) {
        // Text formats
        self.add_mapping(CF_TEXT, "text/plain;charset=utf-8");
        self.add_mapping(CF_UNICODETEXT, "text/plain");
        self.add_mapping(CF_HTML, "text/html");
        self.add_mapping(CF_RTF, "application/rtf");

        // Image formats
        self.add_mapping(CF_DIB, "image/bmp");
        self.add_mapping(CF_BITMAP, "image/bmp");
        self.add_mapping(CF_PNG, "image/png");
        self.add_mapping(CF_JPEG, "image/jpeg");
        self.add_mapping(CF_GIF, "image/gif");

        // File formats
        self.add_mapping(CF_HDROP, "text/uri-list");

        // Audio formats
        self.add_mapping(CF_WAVE, "audio/wav");
        self.add_mapping(CF_RIFF, "application/riff");

        // Metadata
        self.add_mapping(CF_LOCALE, "application/x-locale");
    }

    /// Add a format mapping
    fn add_mapping(&mut self, format_id: u32, mime_type: &str) {
        self.format_map.insert(format_id, mime_type.to_string());
        self.mime_map.insert(mime_type.to_string(), format_id);
    }

    /// Convert data to RDP format
    pub async fn convert_to_rdp(
        &self,
        data: &[u8],
        mime_type: &str,
        format_id: u32,
    ) -> Result<Vec<u8>> {
        match (mime_type, format_id) {
            // Text conversions
            ("text/plain", CF_UNICODETEXT) | ("text/plain;charset=utf-8", CF_UNICODETEXT) => {
                self.convert_text_to_unicode(data)
            }
            ("text/plain;charset=utf-8", CF_TEXT) => Ok(data.to_vec()),
            ("text/html", CF_HTML) => self.convert_html_to_rdp_html(data),
            ("application/rtf", CF_RTF) => Ok(data.to_vec()),

            // Image conversions
            ("image/png", CF_PNG) => Ok(data.to_vec()),
            ("image/png", CF_DIB) => self.convert_png_to_dib(data).await,
            ("image/jpeg", CF_JPEG) => Ok(data.to_vec()),
            ("image/jpeg", CF_DIB) => self.convert_jpeg_to_dib(data).await,
            ("image/bmp", CF_DIB) => self.convert_bmp_to_dib(data),
            ("image/gif", CF_GIF) => Ok(data.to_vec()),

            // File list conversion
            ("text/uri-list", CF_HDROP) => self.convert_uri_list_to_hdrop(data),

            _ => Err(ClipboardError::UnsupportedFormat(format!(
                "MIME type '{}' to format ID {}",
                mime_type, format_id
            ))),
        }
    }

    /// Convert data from RDP format
    pub async fn convert_from_rdp(
        &self,
        data: &[u8],
        format_id: u32,
        mime_type: &str,
    ) -> Result<Vec<u8>> {
        match (format_id, mime_type) {
            // Text conversions
            (CF_UNICODETEXT, "text/plain") | (CF_UNICODETEXT, "text/plain;charset=utf-8") => {
                self.convert_unicode_to_text(data)
            }
            (CF_TEXT, "text/plain;charset=utf-8") => Ok(data.to_vec()),
            (CF_HTML, "text/html") => self.convert_rdp_html_to_html(data),
            (CF_RTF, "application/rtf") => Ok(data.to_vec()),

            // Image conversions
            (CF_PNG, "image/png") => Ok(data.to_vec()),
            (CF_DIB, "image/png") => self.convert_dib_to_png(data).await,
            (CF_DIB, "image/bmp") => self.convert_dib_to_bmp(data),
            (CF_JPEG, "image/jpeg") => Ok(data.to_vec()),
            (CF_GIF, "image/gif") => Ok(data.to_vec()),

            // File list conversion
            (CF_HDROP, "text/uri-list") => self.convert_hdrop_to_uri_list(data),

            _ => Err(ClipboardError::UnsupportedFormat(format!(
                "Format ID {} to MIME type '{}'",
                format_id, mime_type
            ))),
        }
    }

    // ===== Text Conversion Functions =====

    /// Convert UTF-8 text to UTF-16LE (Windows Unicode format)
    fn convert_text_to_unicode(&self, data: &[u8]) -> Result<Vec<u8>> {
        let text = std::str::from_utf8(data).map_err(|_| ClipboardError::InvalidUtf8)?;

        let mut result = Vec::new();
        for ch in text.encode_utf16() {
            result.extend_from_slice(&ch.to_le_bytes());
        }
        // Add null terminator
        result.extend_from_slice(&[0u8, 0u8]);

        Ok(result)
    }

    /// Convert UTF-16LE to UTF-8 text
    fn convert_unicode_to_text(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() % 2 != 0 {
            return Err(ClipboardError::InvalidData(
                "UTF-16 data must have even length".to_string(),
            ));
        }

        let mut utf16_data = Vec::new();
        for chunk in data.chunks_exact(2) {
            let value = u16::from_le_bytes([chunk[0], chunk[1]]);
            if value == 0 {
                break; // Null terminator
            }
            utf16_data.push(value);
        }

        let text = String::from_utf16(&utf16_data).map_err(|_| ClipboardError::InvalidUtf16)?;

        Ok(text.into_bytes())
    }

    /// Convert HTML to RDP HTML format (with CF_HTML header)
    fn convert_html_to_rdp_html(&self, data: &[u8]) -> Result<Vec<u8>> {
        let html = std::str::from_utf8(data).map_err(|_| ClipboardError::InvalidUtf8)?;

        // Wrap HTML in fragment markers
        let fragment = format!(
            "<html><body>\r\n<!--StartFragment-->{html}<!--EndFragment-->\r\n</body></html>"
        );

        // Calculate offsets
        let version_line = "Version:0.9\r\n";
        let start_html_line = "StartHTML:0000000000\r\n";
        let end_html_line = "EndHTML:0000000000\r\n";
        let start_fragment_line = "StartFragment:0000000000\r\n";
        let end_fragment_line = "EndFragment:0000000000\r\n";
        let source_url_line = "SourceURL:about:blank\r\n";

        let header_len = version_line.len()
            + start_html_line.len()
            + end_html_line.len()
            + start_fragment_line.len()
            + end_fragment_line.len()
            + source_url_line.len();

        let start_html = header_len;
        let end_html = start_html + fragment.len();

        let fragment_marker = "<!--StartFragment-->";
        let start_fragment =
            start_html + fragment.find(fragment_marker).unwrap() + fragment_marker.len();
        let end_fragment = start_html + fragment.find("<!--EndFragment-->").unwrap();

        // Build header with correct offsets
        let header = format!(
            "Version:0.9\r\n\
             StartHTML:{start_html:010}\r\n\
             EndHTML:{end_html:010}\r\n\
             StartFragment:{start_fragment:010}\r\n\
             EndFragment:{end_fragment:010}\r\n\
             SourceURL:about:blank\r\n"
        );

        let mut result = header.into_bytes();
        result.extend_from_slice(fragment.as_bytes());

        Ok(result)
    }

    /// Convert RDP HTML format to plain HTML
    fn convert_rdp_html_to_html(&self, data: &[u8]) -> Result<Vec<u8>> {
        let text = std::str::from_utf8(data).map_err(|_| ClipboardError::InvalidUtf8)?;

        // Parse CF_HTML header
        let mut start_fragment = 0;
        let mut end_fragment = text.len();

        for line in text.lines() {
            if let Some(offset_str) = line.strip_prefix("StartFragment:") {
                if let Ok(offset) = offset_str.trim().parse::<usize>() {
                    start_fragment = offset;
                }
            } else if let Some(offset_str) = line.strip_prefix("EndFragment:") {
                if let Ok(offset) = offset_str.trim().parse::<usize>() {
                    end_fragment = offset;
                }
            }
        }

        // Extract fragment
        if start_fragment < data.len() && end_fragment <= data.len() {
            Ok(data[start_fragment..end_fragment].to_vec())
        } else {
            // If parsing fails, return the whole content
            Ok(data.to_vec())
        }
    }

    // ===== Image Conversion Functions =====

    /// Convert PNG to DIB (Device Independent Bitmap)
    async fn convert_png_to_dib(&self, png_data: &[u8]) -> Result<Vec<u8>> {
        let image = image::load_from_memory_with_format(png_data, ImageFormat::Png)
            .map_err(|e| ClipboardError::ImageDecodeError(e.to_string()))?;

        self.create_dib_from_image(&image)
    }

    /// Convert JPEG to DIB
    async fn convert_jpeg_to_dib(&self, jpeg_data: &[u8]) -> Result<Vec<u8>> {
        let image = image::load_from_memory_with_format(jpeg_data, ImageFormat::Jpeg)
            .map_err(|e| ClipboardError::ImageDecodeError(e.to_string()))?;

        self.create_dib_from_image(&image)
    }

    /// Convert BMP to DIB (extract DIB portion from BMP file)
    fn convert_bmp_to_dib(&self, bmp_data: &[u8]) -> Result<Vec<u8>> {
        // BMP file format: File header (14 bytes) + DIB header + pixel data
        // DIB is everything after the 14-byte file header
        if bmp_data.len() < 14 {
            return Err(ClipboardError::InvalidData(
                "BMP file too small".to_string(),
            ));
        }

        // Verify BMP signature
        if &bmp_data[0..2] != b"BM" {
            return Err(ClipboardError::InvalidData(
                "Invalid BMP signature".to_string(),
            ));
        }

        // DIB is everything after file header
        Ok(bmp_data[14..].to_vec())
    }

    /// Create DIB from image
    fn create_dib_from_image(&self, image: &DynamicImage) -> Result<Vec<u8>> {
        let rgba = image.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());

        let mut dib = BytesMut::new();

        // BITMAPINFOHEADER structure (40 bytes)
        dib.put_u32_le(40); // biSize
        dib.put_i32_le(width as i32); // biWidth
        dib.put_i32_le(-(height as i32)); // biHeight (negative for top-down)
        dib.put_u16_le(1); // biPlanes
        dib.put_u16_le(32); // biBitCount (32 bits for BGRA)
        dib.put_u32_le(0); // biCompression (BI_RGB = 0)

        let image_size = width * height * 4;
        dib.put_u32_le(image_size); // biSizeImage

        dib.put_i32_le(0); // biXPelsPerMeter
        dib.put_i32_le(0); // biYPelsPerMeter
        dib.put_u32_le(0); // biClrUsed
        dib.put_u32_le(0); // biClrImportant

        // Pixel data (convert RGBA to BGRA)
        for pixel in rgba.pixels() {
            dib.put_u8(pixel[2]); // Blue
            dib.put_u8(pixel[1]); // Green
            dib.put_u8(pixel[0]); // Red
            dib.put_u8(pixel[3]); // Alpha
        }

        Ok(dib.to_vec())
    }

    /// Convert DIB to PNG
    async fn convert_dib_to_png(&self, dib_data: &[u8]) -> Result<Vec<u8>> {
        let image = self.parse_dib_to_image(dib_data)?;

        let mut png_data = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut png_data), ImageFormat::Png)
            .map_err(|e| ClipboardError::ImageEncodeError(e.to_string()))?;

        Ok(png_data)
    }

    /// Convert DIB to BMP
    fn convert_dib_to_bmp(&self, dib_data: &[u8]) -> Result<Vec<u8>> {
        if dib_data.len() < 40 {
            return Err(ClipboardError::InvalidData("DIB too small".to_string()));
        }

        // Parse DIB header to get dimensions and data size
        let width = i32::from_le_bytes([dib_data[4], dib_data[5], dib_data[6], dib_data[7]]);
        let height = i32::from_le_bytes([dib_data[8], dib_data[9], dib_data[10], dib_data[11]]);
        let bit_count = u16::from_le_bytes([dib_data[14], dib_data[15]]);

        // Calculate image data size
        let row_size = ((width.abs() as u32 * bit_count as u32 + 31) / 32) * 4;
        let image_size = row_size * height.abs() as u32;

        let file_size = 14 + dib_data.len() as u32;
        let pixel_offset = 14 + 40; // File header + DIB header

        let mut bmp = BytesMut::new();

        // BMP file header (14 bytes)
        bmp.put_slice(b"BM"); // Signature
        bmp.put_u32_le(file_size); // File size
        bmp.put_u16_le(0); // Reserved1
        bmp.put_u16_le(0); // Reserved2
        bmp.put_u32_le(pixel_offset); // Pixel data offset

        // Append DIB data
        bmp.put_slice(dib_data);

        Ok(bmp.to_vec())
    }

    /// Parse DIB data to image
    fn parse_dib_to_image(&self, dib_data: &[u8]) -> Result<DynamicImage> {
        if dib_data.len() < 40 {
            return Err(ClipboardError::InvalidData("DIB too small".to_string()));
        }

        // Parse BITMAPINFOHEADER
        let bi_size = u32::from_le_bytes([dib_data[0], dib_data[1], dib_data[2], dib_data[3]]);
        if bi_size < 40 {
            return Err(ClipboardError::InvalidData(
                "Invalid DIB header".to_string(),
            ));
        }

        let width =
            i32::from_le_bytes([dib_data[4], dib_data[5], dib_data[6], dib_data[7]]).abs() as u32;
        let height =
            i32::from_le_bytes([dib_data[8], dib_data[9], dib_data[10], dib_data[11]]).abs() as u32;
        let bit_count = u16::from_le_bytes([dib_data[14], dib_data[15]]);

        let header_size = bi_size as usize;
        let pixel_data = &dib_data[header_size..];

        // Convert based on bit depth
        let image = match bit_count {
            32 => {
                // 32-bit BGRA
                let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
                for chunk in pixel_data.chunks_exact(4) {
                    rgba_data.push(chunk[2]); // Red
                    rgba_data.push(chunk[1]); // Green
                    rgba_data.push(chunk[0]); // Blue
                    rgba_data.push(chunk[3]); // Alpha
                }
                DynamicImage::ImageRgba8(
                    image::RgbaImage::from_raw(width, height, rgba_data)
                        .ok_or(ClipboardError::ImageCreateError)?,
                )
            }
            24 => {
                // 24-bit BGR
                let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
                let row_size = ((width * 3 + 3) / 4) * 4; // Aligned to 4 bytes
                for y in 0..height {
                    let row_offset = (y * row_size) as usize;
                    for x in 0..width {
                        let pixel_offset = row_offset + (x * 3) as usize;
                        if pixel_offset + 2 < pixel_data.len() {
                            rgb_data.push(pixel_data[pixel_offset + 2]); // Red
                            rgb_data.push(pixel_data[pixel_offset + 1]); // Green
                            rgb_data.push(pixel_data[pixel_offset]); // Blue
                        }
                    }
                }
                DynamicImage::ImageRgb8(
                    image::RgbImage::from_raw(width, height, rgb_data)
                        .ok_or(ClipboardError::ImageCreateError)?,
                )
            }
            _ => return Err(ClipboardError::UnsupportedBitDepth(bit_count)),
        };

        Ok(image)
    }

    // ===== File Drop Conversion Functions =====

    /// Convert URI list to HDROP (Windows file drop format)
    fn convert_uri_list_to_hdrop(&self, data: &[u8]) -> Result<Vec<u8>> {
        let uri_list = std::str::from_utf8(data).map_err(|_| ClipboardError::InvalidUtf8)?;

        let mut hdrop = BytesMut::new();

        // DROPFILES structure (20 bytes)
        hdrop.put_u32_le(20); // pFiles (offset to file list)
        hdrop.put_i32_le(0); // pt.x
        hdrop.put_i32_le(0); // pt.y
        hdrop.put_i32_le(0); // fNC (not NC)
        hdrop.put_i32_le(1); // fWide (Unicode)

        // File paths (null-terminated UTF-16LE, double-null at end)
        for line in uri_list.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(path) = line.strip_prefix("file://") {
                // Decode percent-encoded URI
                let decoded = percent_encoding::percent_decode_str(path)
                    .decode_utf8()
                    .map_err(|_| ClipboardError::InvalidUtf8)?;

                // Convert to UTF-16LE
                for ch in decoded.encode_utf16() {
                    hdrop.put_u16_le(ch);
                }
                hdrop.put_u16_le(0); // Null terminator
            }
        }
        hdrop.put_u16_le(0); // Double null terminator

        Ok(hdrop.to_vec())
    }

    /// Convert HDROP to URI list
    fn convert_hdrop_to_uri_list(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 20 {
            return Err(ClipboardError::InvalidData(
                "HDROP structure too small".to_string(),
            ));
        }

        let offset = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let wide = i32::from_le_bytes([data[16], data[17], data[18], data[19]]) != 0;

        if offset >= data.len() {
            return Err(ClipboardError::InvalidData(
                "Invalid HDROP offset".to_string(),
            ));
        }

        let file_data = &data[offset..];
        let mut uri_list = String::new();

        if wide {
            // UTF-16LE paths
            let mut current_path = Vec::new();
            let mut i = 0;

            while i + 1 < file_data.len() {
                let ch = u16::from_le_bytes([file_data[i], file_data[i + 1]]);
                i += 2;

                if ch == 0 {
                    if !current_path.is_empty() {
                        let path = String::from_utf16(&current_path)
                            .map_err(|_| ClipboardError::InvalidUtf16)?;

                        uri_list.push_str("file://");
                        uri_list
                            .push_str(&utf8_percent_encode(&path, NON_ALPHANUMERIC).to_string());
                        uri_list.push('\n');

                        current_path.clear();
                    } else {
                        break; // Double null terminator
                    }
                } else {
                    current_path.push(ch);
                }
            }
        } else {
            // ANSI paths
            let mut current_path = Vec::new();

            for &byte in file_data {
                if byte == 0 {
                    if !current_path.is_empty() {
                        let path = String::from_utf8(current_path.clone())
                            .map_err(|_| ClipboardError::InvalidUtf8)?;

                        uri_list.push_str("file://");
                        uri_list
                            .push_str(&utf8_percent_encode(&path, NON_ALPHANUMERIC).to_string());
                        uri_list.push('\n');

                        current_path.clear();
                    } else {
                        break; // Double null terminator
                    }
                } else {
                    current_path.push(byte);
                }
            }
        }

        Ok(uri_list.into_bytes())
    }

    // ===== Format Mapping Functions =====

    /// Convert RDP formats to MIME types
    pub fn rdp_to_mime_types(&self, formats: &[ClipboardFormat]) -> Result<Vec<String>> {
        let mut mime_types = Vec::new();

        for format in formats {
            if let Some(mime) = self.format_map.get(&format.format_id) {
                mime_types.push(mime.clone());
            } else if format.format_id >= CF_CUSTOM_START {
                // Handle custom format
                mime_types.push(format!("application/x-rdp-custom-{}", format.format_id));
            }
        }

        Ok(mime_types)
    }

    /// Convert MIME types to RDP formats
    pub fn mime_to_rdp_formats(&self, mime_types: &[String]) -> Result<Vec<ClipboardFormat>> {
        let mut formats = Vec::new();

        for mime in mime_types {
            if let Some(format_id) = self.mime_map.get(mime) {
                formats.push(ClipboardFormat {
                    format_id: *format_id,
                    format_name: self.get_format_name(*format_id),
                });
            } else if let Some(id_str) = mime.strip_prefix("application/x-rdp-custom-") {
                // Handle custom format
                if let Ok(id) = id_str.parse::<u32>() {
                    formats.push(ClipboardFormat {
                        format_id: id,
                        format_name: mime.clone(),
                    });
                }
            }
        }

        Ok(formats)
    }

    /// Get format name for format ID
    fn get_format_name(&self, format_id: u32) -> String {
        match format_id {
            CF_TEXT => "CF_TEXT".to_string(),
            CF_BITMAP => "CF_BITMAP".to_string(),
            CF_UNICODETEXT => "CF_UNICODETEXT".to_string(),
            CF_DIB => "CF_DIB".to_string(),
            CF_HTML => "HTML Format".to_string(),
            CF_PNG => "PNG".to_string(),
            CF_JPEG => "JPEG".to_string(),
            CF_RTF => "Rich Text Format".to_string(),
            CF_HDROP => "CF_HDROP".to_string(),
            CF_GIF => "GIF".to_string(),
            _ => format!("Format_{}", format_id),
        }
    }

    /// Get MIME type for format ID
    pub fn format_id_to_mime(&self, format_id: u32) -> Result<String> {
        self.format_map
            .get(&format_id)
            .cloned()
            .ok_or(ClipboardError::UnknownFormat(format_id))
    }

    /// Get format ID for MIME type
    pub fn mime_to_format_id(&self, mime_type: &str) -> Result<u32> {
        self.mime_map
            .get(mime_type)
            .copied()
            .ok_or(ClipboardError::UnsupportedFormat(mime_type.to_string()))
    }
}

impl Default for FormatConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_text_to_unicode_conversion() {
        let converter = FormatConverter::new();
        let text = "Hello, 世界!";

        let unicode = converter.convert_text_to_unicode(text.as_bytes()).unwrap();

        // Verify UTF-16LE encoding with null terminator
        let expected: Vec<u8> = text
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .chain([0u8, 0u8])
            .collect();

        assert_eq!(unicode, expected);

        // Round-trip
        let back = converter.convert_unicode_to_text(&unicode).unwrap();
        assert_eq!(back, text.as_bytes());
    }

    #[tokio::test]
    async fn test_html_to_rdp_html_conversion() {
        let converter = FormatConverter::new();
        let html = "<b>Test HTML</b>";

        let rdp_html = converter.convert_html_to_rdp_html(html.as_bytes()).unwrap();
        let rdp_str = std::str::from_utf8(&rdp_html).unwrap();

        // Verify RDP HTML format
        assert!(rdp_str.starts_with("Version:0.9"));
        assert!(rdp_str.contains("StartHTML:"));
        assert!(rdp_str.contains("EndHTML:"));
        assert!(rdp_str.contains("StartFragment:"));
        assert!(rdp_str.contains("EndFragment:"));
        assert!(rdp_str.contains("<b>Test HTML</b>"));

        // Round-trip
        let back = converter.convert_rdp_html_to_html(&rdp_html).unwrap();
        let back_str = std::str::from_utf8(&back).unwrap();
        assert!(back_str.contains("<b>Test HTML</b>"));
    }

    #[tokio::test]
    async fn test_image_dib_conversion() {
        let converter = FormatConverter::new();

        // Create a small test image (10x10 red square)
        let image = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            10,
            10,
            image::Rgba([255, 0, 0, 255]),
        ));

        // Convert to DIB
        let dib = converter.create_dib_from_image(&image).unwrap();

        // Verify DIB header
        assert!(dib.len() >= 40);
        assert_eq!(u32::from_le_bytes([dib[0], dib[1], dib[2], dib[3]]), 40); // biSize

        // Convert back to image
        let parsed = converter.parse_dib_to_image(&dib).unwrap();
        assert_eq!(parsed.width(), 10);
        assert_eq!(parsed.height(), 10);
    }

    #[tokio::test]
    async fn test_uri_list_to_hdrop_conversion() {
        let converter = FormatConverter::new();
        let uri_list = "file:///home/user/document.txt\nfile:///home/user/image.png";

        let hdrop = converter
            .convert_uri_list_to_hdrop(uri_list.as_bytes())
            .unwrap();

        // Verify DROPFILES structure
        assert!(hdrop.len() >= 20);
        assert_eq!(
            u32::from_le_bytes([hdrop[0], hdrop[1], hdrop[2], hdrop[3]]),
            20
        );

        // Round-trip
        let back = converter.convert_hdrop_to_uri_list(&hdrop).unwrap();
        let back_str = std::str::from_utf8(&back).unwrap();
        assert!(back_str.contains("file://"));
        assert!(back_str.contains("document.txt"));
        assert!(back_str.contains("image.png"));
    }

    #[test]
    fn test_format_mapping() {
        let converter = FormatConverter::new();

        // Test RDP to MIME
        let formats = vec![
            ClipboardFormat {
                format_id: CF_UNICODETEXT,
                format_name: "CF_UNICODETEXT".to_string(),
            },
            ClipboardFormat {
                format_id: CF_PNG,
                format_name: "PNG".to_string(),
            },
        ];

        let mime_types = converter.rdp_to_mime_types(&formats).unwrap();
        assert_eq!(mime_types.len(), 2);
        assert!(mime_types.contains(&"text/plain".to_string()));
        assert!(mime_types.contains(&"image/png".to_string()));

        // Test MIME to RDP
        let rdp_formats = converter.mime_to_rdp_formats(&mime_types).unwrap();
        assert_eq!(rdp_formats.len(), 2);
    }

    #[test]
    fn test_format_id_to_mime() {
        let converter = FormatConverter::new();

        assert_eq!(
            converter.format_id_to_mime(CF_UNICODETEXT).unwrap(),
            "text/plain"
        );
        assert_eq!(converter.format_id_to_mime(CF_PNG).unwrap(), "image/png");
        assert_eq!(converter.format_id_to_mime(CF_HTML).unwrap(), "text/html");
    }

    #[test]
    fn test_mime_to_format_id() {
        let converter = FormatConverter::new();

        assert_eq!(
            converter.mime_to_format_id("text/plain").unwrap(),
            CF_UNICODETEXT
        );
        assert_eq!(converter.mime_to_format_id("image/png").unwrap(), CF_PNG);
        assert_eq!(converter.mime_to_format_id("text/html").unwrap(), CF_HTML);
    }

    #[test]
    fn test_custom_format() {
        let converter = FormatConverter::new();

        let formats = vec![ClipboardFormat {
            format_id: 0xC001,
            format_name: "CustomFormat".to_string(),
        }];

        let mime_types = converter.rdp_to_mime_types(&formats).unwrap();
        assert_eq!(mime_types.len(), 1);
        assert_eq!(mime_types[0], "application/x-rdp-custom-49153");

        // Round-trip
        let rdp_formats = converter.mime_to_rdp_formats(&mime_types).unwrap();
        assert_eq!(rdp_formats.len(), 1);
        assert_eq!(rdp_formats[0].format_id, 0xC001);
    }

    #[tokio::test]
    async fn test_png_to_dib_conversion() {
        let converter = FormatConverter::new();

        // Create a small PNG image
        let image = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            5,
            5,
            image::Rgba([100, 150, 200, 255]),
        ));

        let mut png_data = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut png_data), ImageFormat::Png)
            .unwrap();

        // Convert to DIB
        let dib = converter.convert_png_to_dib(&png_data).await.unwrap();

        // Verify header
        assert!(dib.len() >= 40);

        // Convert back to PNG
        let png_back = converter.convert_dib_to_png(&dib).await.unwrap();

        // Load and verify
        let loaded = image::load_from_memory(&png_back).unwrap();
        assert_eq!(loaded.width(), 5);
        assert_eq!(loaded.height(), 5);
    }

    #[tokio::test]
    async fn test_empty_text_conversion() {
        let converter = FormatConverter::new();

        let unicode = converter.convert_text_to_unicode(b"").unwrap();
        assert_eq!(unicode, vec![0u8, 0u8]); // Just null terminator

        let back = converter.convert_unicode_to_text(&unicode).unwrap();
        assert_eq!(back, b"");
    }

    #[test]
    fn test_invalid_utf16() {
        let converter = FormatConverter::new();

        // Odd length (invalid UTF-16)
        let result = converter.convert_unicode_to_text(&[0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_dib() {
        let converter = FormatConverter::new();

        // Too small
        let result = converter.parse_dib_to_image(&[0; 30]);
        assert!(result.is_err());

        // Invalid header size
        let mut invalid_dib = vec![0; 40];
        invalid_dib[0] = 10; // Invalid biSize
        let result = converter.parse_dib_to_image(&invalid_dib);
        assert!(result.is_err());
    }
}
