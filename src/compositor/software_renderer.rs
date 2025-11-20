//! Software renderer implementation
//!
//! Renders Wayland surfaces to a memory framebuffer using software rendering.

use super::types::*;
use super::state::{Surface, SurfaceBuffer};
use anyhow::Result;
use tracing::{debug, info, trace};

/// Software renderer
pub struct SoftwareRenderer {
    /// Target framebuffer
    framebuffer: FrameBuffer,

    /// Damage accumulator
    damage_regions: Vec<Rectangle>,

    /// Background color
    background_color: [u8; 4],
}

impl SoftwareRenderer {
    /// Create new software renderer
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Self {
        info!("Creating software renderer: {}x{} {:?}", width, height, format);

        Self {
            framebuffer: FrameBuffer::new(width, height, format),
            damage_regions: Vec::new(),
            background_color: [0, 0, 0, 255], // Black
        }
    }

    /// Clear framebuffer to background color
    pub fn clear(&mut self) {
        let stride = self.framebuffer.stride();
        let bpp = self.framebuffer.format.bytes_per_pixel();

        for y in 0..self.framebuffer.height as usize {
            let row_offset = y * stride;
            for x in 0..self.framebuffer.width as usize {
                let offset = row_offset + x * bpp;
                self.framebuffer.data[offset..offset + bpp]
                    .copy_from_slice(&self.background_color);
            }
        }

        self.damage_all();
    }

    /// Clear specific region
    pub fn clear_region(&mut self, region: Rectangle) {
        let stride = self.framebuffer.stride();
        let bpp = self.framebuffer.format.bytes_per_pixel();

        let start_y = region.y.max(0) as usize;
        let end_y = (region.y + region.height as i32)
            .min(self.framebuffer.height as i32) as usize;
        let start_x = region.x.max(0) as usize;
        let end_x = (region.x + region.width as i32)
            .min(self.framebuffer.width as i32) as usize;

        for y in start_y..end_y {
            let row_offset = y * stride;
            for x in start_x..end_x {
                let offset = row_offset + x * bpp;
                self.framebuffer.data[offset..offset + bpp]
                    .copy_from_slice(&self.background_color);
            }
        }

        self.damage_region(region);
    }

    /// Render surface to framebuffer
    pub fn render_surface(&mut self, surface: &Surface, x: i32, y: i32) -> Result<()> {
        if let Some(buffer) = &surface.buffer {
            trace!("Rendering surface at ({}, {}): {}x{}", x, y, buffer.width, buffer.height);

            self.blit_buffer(buffer, x, y)?;

            // Add surface damage
            let surface_rect = Rectangle::new(x, y, buffer.width, buffer.height);
            self.damage_region(surface_rect);
        }

        Ok(())
    }

    /// Blit surface buffer to framebuffer
    fn blit_buffer(&mut self, buffer: &SurfaceBuffer, dst_x: i32, dst_y: i32) -> Result<()> {
        let fb_stride = self.framebuffer.stride();
        let fb_bpp = self.framebuffer.format.bytes_per_pixel();
        let src_stride = buffer.width as usize * buffer.format.bytes_per_pixel();
        let src_bpp = buffer.format.bytes_per_pixel();

        // Calculate clipped region
        let src_width = buffer.width as i32;
        let src_height = buffer.height as i32;

        let clip_left = (-dst_x).max(0);
        let clip_top = (-dst_y).max(0);
        let clip_right = (dst_x + src_width - self.framebuffer.width as i32).max(0);
        let clip_bottom = (dst_y + src_height - self.framebuffer.height as i32).max(0);

        let copy_width = (src_width - clip_left - clip_right).max(0) as usize;
        let copy_height = (src_height - clip_top - clip_bottom).max(0) as usize;

        if copy_width == 0 || copy_height == 0 {
            return Ok(());
        }

        let dst_start_x = (dst_x + clip_left).max(0) as usize;
        let dst_start_y = (dst_y + clip_top).max(0) as usize;
        let src_start_x = clip_left as usize;
        let src_start_y = clip_top as usize;

        // Copy pixels
        for row in 0..copy_height {
            let src_y = src_start_y + row;
            let dst_y = dst_start_y + row;

            let src_row_offset = src_y * src_stride + src_start_x * src_bpp;
            let dst_row_offset = dst_y * fb_stride + dst_start_x * fb_bpp;

            // Handle format conversion if needed
            if buffer.format == self.framebuffer.format && fb_bpp == src_bpp {
                // Direct copy (same format)
                let src_slice = &buffer.data[src_row_offset..src_row_offset + copy_width * src_bpp];
                let dst_slice = &mut self.framebuffer.data[dst_row_offset..dst_row_offset + copy_width * fb_bpp];
                dst_slice.copy_from_slice(src_slice);
            } else {
                // Format conversion - need to avoid simultaneous borrows
                let src_format = buffer.format;
                let dst_format = self.framebuffer.format;

                for col in 0..copy_width {
                    let src_offset = src_row_offset + col * src_bpp;
                    let dst_offset = dst_row_offset + col * fb_bpp;

                    let src_pixel = &buffer.data[src_offset..src_offset + src_bpp];

                    // Extract RGBA components from source
                    let (r, g, b, a) = match src_format {
                        PixelFormat::BGRA8888 => (src_pixel[2], src_pixel[1], src_pixel[0], src_pixel[3]),
                        PixelFormat::RGBA8888 => (src_pixel[0], src_pixel[1], src_pixel[2], src_pixel[3]),
                        PixelFormat::BGRX8888 => (src_pixel[2], src_pixel[1], src_pixel[0], 255),
                        PixelFormat::RGBX8888 => (src_pixel[0], src_pixel[1], src_pixel[2], 255),
                    };

                    // Write to destination format
                    let dst_pixel = &mut self.framebuffer.data[dst_offset..dst_offset + fb_bpp];
                    match dst_format {
                        PixelFormat::BGRA8888 => {
                            dst_pixel[0] = b;
                            dst_pixel[1] = g;
                            dst_pixel[2] = r;
                            dst_pixel[3] = a;
                        }
                        PixelFormat::RGBA8888 => {
                            dst_pixel[0] = r;
                            dst_pixel[1] = g;
                            dst_pixel[2] = b;
                            dst_pixel[3] = a;
                        }
                        PixelFormat::BGRX8888 => {
                            dst_pixel[0] = b;
                            dst_pixel[1] = g;
                            dst_pixel[2] = r;
                            dst_pixel[3] = 255;
                        }
                        PixelFormat::RGBX8888 => {
                            dst_pixel[0] = r;
                            dst_pixel[1] = g;
                            dst_pixel[2] = b;
                            dst_pixel[3] = 255;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Convert pixel from one format to another
    fn convert_pixel(&self, src: &[u8], dst: &mut [u8], src_fmt: PixelFormat, dst_fmt: PixelFormat) {
        // Extract RGBA components from source
        let (r, g, b, a) = match src_fmt {
            PixelFormat::BGRA8888 => (src[2], src[1], src[0], src[3]),
            PixelFormat::RGBA8888 => (src[0], src[1], src[2], src[3]),
            PixelFormat::BGRX8888 => (src[2], src[1], src[0], 255),
            PixelFormat::RGBX8888 => (src[0], src[1], src[2], 255),
        };

        // Write to destination format
        match dst_fmt {
            PixelFormat::BGRA8888 => {
                dst[0] = b;
                dst[1] = g;
                dst[2] = r;
                dst[3] = a;
            }
            PixelFormat::RGBA8888 => {
                dst[0] = r;
                dst[1] = g;
                dst[2] = b;
                dst[3] = a;
            }
            PixelFormat::BGRX8888 => {
                dst[0] = b;
                dst[1] = g;
                dst[2] = r;
                dst[3] = 255;
            }
            PixelFormat::RGBX8888 => {
                dst[0] = r;
                dst[1] = g;
                dst[2] = b;
                dst[3] = 255;
            }
        }
    }

    /// Composite cursor onto framebuffer
    pub fn render_cursor(&mut self, cursor: &CursorState) -> Result<()> {
        if !cursor.visible || cursor.image.is_none() {
            return Ok(());
        }

        let image = cursor.image.as_ref().unwrap();

        // Create temporary buffer for cursor
        let cursor_buffer = SurfaceBuffer {
            width: cursor.size.width,
            height: cursor.size.height,
            data: image.clone(),
            format: PixelFormat::RGBA8888, // Cursor images are typically RGBA
        };

        // Calculate cursor position (accounting for hotspot)
        let cursor_x = cursor.position.x - cursor.hotspot.x;
        let cursor_y = cursor.position.y - cursor.hotspot.y;

        // Blit cursor with alpha blending
        self.blit_cursor(&cursor_buffer, cursor_x, cursor_y)?;

        Ok(())
    }

    /// Blit cursor with alpha blending
    fn blit_cursor(&mut self, buffer: &SurfaceBuffer, dst_x: i32, dst_y: i32) -> Result<()> {
        let fb_stride = self.framebuffer.stride();
        let fb_bpp = self.framebuffer.format.bytes_per_pixel();

        // Calculate clipped region
        let src_width = buffer.width as i32;
        let src_height = buffer.height as i32;

        let clip_left = (-dst_x).max(0);
        let clip_top = (-dst_y).max(0);
        let clip_right = (dst_x + src_width - self.framebuffer.width as i32).max(0);
        let clip_bottom = (dst_y + src_height - self.framebuffer.height as i32).max(0);

        let copy_width = (src_width - clip_left - clip_right).max(0) as usize;
        let copy_height = (src_height - clip_top - clip_bottom).max(0) as usize;

        if copy_width == 0 || copy_height == 0 {
            return Ok(());
        }

        let dst_start_x = (dst_x + clip_left).max(0) as usize;
        let dst_start_y = (dst_y + clip_top).max(0) as usize;

        // Alpha blend cursor
        for row in 0..copy_height {
            for col in 0..copy_width {
                let src_offset = ((clip_top as usize + row) * buffer.width as usize +
                                  (clip_left as usize + col)) * 4;
                let dst_offset = ((dst_start_y + row) * self.framebuffer.width as usize +
                                  (dst_start_x + col)) * fb_bpp;

                // Get cursor pixel (RGBA)
                let r = buffer.data[src_offset] as u32;
                let g = buffer.data[src_offset + 1] as u32;
                let b = buffer.data[src_offset + 2] as u32;
                let alpha = buffer.data[src_offset + 3] as u32;

                if alpha == 0 {
                    continue; // Fully transparent
                }

                // Get background pixel
                let bg_data = &mut self.framebuffer.data[dst_offset..dst_offset + fb_bpp];

                let (bg_r, bg_g, bg_b) = match self.framebuffer.format {
                    PixelFormat::BGRA8888 | PixelFormat::BGRX8888 => {
                        (bg_data[2] as u32, bg_data[1] as u32, bg_data[0] as u32)
                    }
                    PixelFormat::RGBA8888 | PixelFormat::RGBX8888 => {
                        (bg_data[0] as u32, bg_data[1] as u32, bg_data[2] as u32)
                    }
                };

                // Alpha blend
                let inv_alpha = 255 - alpha;
                let final_r = ((r * alpha + bg_r * inv_alpha) / 255) as u8;
                let final_g = ((g * alpha + bg_g * inv_alpha) / 255) as u8;
                let final_b = ((b * alpha + bg_b * inv_alpha) / 255) as u8;

                // Write blended pixel
                match self.framebuffer.format {
                    PixelFormat::BGRA8888 => {
                        bg_data[0] = final_b;
                        bg_data[1] = final_g;
                        bg_data[2] = final_r;
                        bg_data[3] = 255;
                    }
                    PixelFormat::RGBA8888 => {
                        bg_data[0] = final_r;
                        bg_data[1] = final_g;
                        bg_data[2] = final_b;
                        bg_data[3] = 255;
                    }
                    PixelFormat::BGRX8888 => {
                        bg_data[0] = final_b;
                        bg_data[1] = final_g;
                        bg_data[2] = final_r;
                        bg_data[3] = 255;
                    }
                    PixelFormat::RGBX8888 => {
                        bg_data[0] = final_r;
                        bg_data[1] = final_g;
                        bg_data[2] = final_b;
                        bg_data[3] = 255;
                    }
                }
            }
        }

        // Damage cursor area
        let cursor_rect = Rectangle::new(dst_x, dst_y, buffer.width, buffer.height);
        self.damage_region(cursor_rect);

        Ok(())
    }

    /// Mark entire framebuffer as damaged
    pub fn damage_all(&mut self) {
        self.damage_regions.clear();
        self.damage_regions.push(Rectangle::new(
            0,
            0,
            self.framebuffer.width,
            self.framebuffer.height,
        ));
    }

    /// Mark region as damaged
    pub fn damage_region(&mut self, region: Rectangle) {
        // Clip to framebuffer bounds
        if let Some(clipped) = region.intersection(&Rectangle::new(
            0,
            0,
            self.framebuffer.width,
            self.framebuffer.height,
        )) {
            self.damage_regions.push(clipped);
        }
    }

    /// Get framebuffer reference
    pub fn framebuffer(&self) -> &FrameBuffer {
        &self.framebuffer
    }

    /// Get mutable framebuffer reference
    pub fn framebuffer_mut(&mut self) -> &mut FrameBuffer {
        &mut self.framebuffer
    }

    /// Get and clear damage
    pub fn take_damage(&mut self) -> DamageInfo {
        let regions = std::mem::take(&mut self.damage_regions);
        DamageInfo::from_regions(regions)
    }

    /// Update framebuffer with damage
    pub fn update_framebuffer_damage(&mut self) {
        self.framebuffer.damage = self.take_damage();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = SoftwareRenderer::new(800, 600, PixelFormat::BGRA8888);
        assert_eq!(renderer.framebuffer.width, 800);
        assert_eq!(renderer.framebuffer.height, 600);
    }

    #[test]
    fn test_clear() {
        let mut renderer = SoftwareRenderer::new(100, 100, PixelFormat::BGRA8888);
        renderer.clear();

        // Check that framebuffer is cleared to black
        let pixel = &renderer.framebuffer.data[0..4];
        assert_eq!(pixel, &[0, 0, 0, 255]); // BGRA black
    }

    #[test]
    fn test_pixel_conversion() {
        let renderer = SoftwareRenderer::new(100, 100, PixelFormat::BGRA8888);

        let src = [255, 128, 64, 255]; // RGBA
        let mut dst = [0, 0, 0, 0];

        renderer.convert_pixel(&src, &mut dst, PixelFormat::RGBA8888, PixelFormat::BGRA8888);

        assert_eq!(dst, [64, 128, 255, 255]); // BGRA
    }

    #[test]
    fn test_damage_tracking() {
        let mut renderer = SoftwareRenderer::new(800, 600, PixelFormat::BGRA8888);

        renderer.damage_region(Rectangle::new(10, 10, 100, 100));
        renderer.damage_region(Rectangle::new(50, 50, 100, 100));

        let damage = renderer.take_damage();
        assert_eq!(damage.regions.len(), 2);
        assert!(damage.total_area > 0);
    }
}
