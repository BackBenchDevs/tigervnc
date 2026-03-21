// RuVNC Viewer - Modern Rust/egui VNC viewer
// Copyright (C) 2026 BackBenchDevs
//
// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub data: Vec<u8>,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let stride = width;
        Self {
            width,
            height,
            stride,
            data: vec![0u8; (width * height * 4) as usize],
        }
    }

    #[allow(dead_code)]
    pub fn pixel_at(&self, x: u32, y: u32) -> [u8; 4] {
        let offset = ((y * self.stride + x) * 4) as usize;
        if offset + 4 <= self.data.len() {
            [
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ]
        } else {
            [0, 0, 0, 255]
        }
    }

    #[allow(dead_code)]
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let offset = ((y * self.stride + x) * 4) as usize;
        if offset + 4 <= self.data.len() {
            self.data[offset] = rgba[0];
            self.data[offset + 1] = rgba[1];
            self.data[offset + 2] = rgba[2];
            self.data[offset + 3] = rgba[3];
        }
    }

    #[allow(dead_code)]
    pub fn from_raw_ptr(width: u32, height: u32, stride: u32, ptr: u64) -> Self {
        let byte_len = (height * stride * 4) as usize;
        let data = if ptr != 0 && byte_len > 0 {
            unsafe { std::slice::from_raw_parts(ptr as *const u8, byte_len).to_vec() }
        } else {
            vec![0u8; (width * height * 4) as usize]
        };
        Self {
            width,
            height,
            stride,
            data,
        }
    }

    #[allow(dead_code)]
    pub fn copy_region_from_ptr(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        src_ptr: u64,
        src_stride: u32,
    ) {
        if src_ptr == 0 {
            return;
        }
        let src_stride_bytes = (src_stride * 4) as usize;
        let dst_stride_bytes = (self.stride * 4) as usize;

        for row in 0..h {
            let sy = (y + row) as usize;
            let sx = x as usize;
            if sy >= self.height as usize {
                break;
            }

            let src_offset = sy * src_stride_bytes + sx * 4;
            let dst_offset = sy * dst_stride_bytes + sx * 4;
            let copy_len = (w as usize * 4).min(self.data.len().saturating_sub(dst_offset));

            if copy_len > 0 {
                unsafe {
                    let src = std::slice::from_raw_parts(
                        (src_ptr as *const u8).add(src_offset),
                        copy_len,
                    );
                    self.data[dst_offset..dst_offset + copy_len].copy_from_slice(src);
                }
            }
        }
    }
}

pub struct CursorData {
    pub width: u32,
    pub height: u32,
    pub hotspot_x: i32,
    pub hotspot_y: i32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZoomMode {
    Fit,
    OneToOne,
    Fixed(f32),
}

impl ZoomMode {
    pub fn compute_scale(
        &self,
        fb_w: u32,
        fb_h: u32,
        available: egui::Vec2,
        pixels_per_point: f32,
    ) -> f32 {
        match self {
            ZoomMode::Fit => {
                let scale_x = available.x / fb_w as f32;
                let scale_y = available.y / fb_h as f32;
                scale_x.min(scale_y)
            }
            ZoomMode::OneToOne => 1.0 / pixels_per_point,
            ZoomMode::Fixed(zoom) => *zoom,
        }
    }

    #[allow(dead_code)]
    pub fn label(&self) -> String {
        match self {
            ZoomMode::Fit => "Fit".to_string(),
            ZoomMode::OneToOne => "1:1".to_string(),
            ZoomMode::Fixed(z) => format!("{}%", (z * 100.0) as u32),
        }
    }
}

#[allow(dead_code)]
pub trait RenderBackend {
    fn set_framebuffer(&mut self, fb: FrameBuffer);
    fn update_region(&mut self, x: i32, y: i32, w: i32, h: i32);
    fn set_cursor(&mut self, cursor: CursorData);
    /// Returns (interaction_response, image_rect) where image_rect is the
    /// actual region the framebuffer is drawn into (may be smaller than the
    /// allocated area due to aspect-ratio letterboxing).
    fn render_to_egui(
        &mut self,
        ui: &mut egui::Ui,
        available_size: egui::Vec2,
    ) -> Option<(egui::Response, egui::Rect)>;
    fn framebuffer_size(&self) -> (u32, u32);
    fn zoom_mode(&self) -> &ZoomMode;
    fn set_zoom_mode(&mut self, mode: ZoomMode);
}

/// Represents a single screen in a multi-monitor VNC session.
/// Maps to the ExtendedDesktopSize pseudo-encoding (RFC 6143 Section 7.8.2).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Screen {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Tracks the server's screen layout for multi-monitor awareness.
/// A single-monitor session has exactly one screen covering the full framebuffer.
#[derive(Debug, Clone, Default)]
pub struct ScreenLayout {
    pub screens: Vec<Screen>,
}

#[allow(dead_code)]
impl ScreenLayout {
    pub fn single(width: u32, height: u32) -> Self {
        Self {
            screens: vec![Screen {
                id: 0,
                x: 0,
                y: 0,
                width,
                height,
            }],
        }
    }

    pub fn screen_count(&self) -> usize {
        self.screens.len()
    }

    pub fn is_multi_monitor(&self) -> bool {
        self.screens.len() > 1
    }

    pub fn bounding_box(&self) -> (i32, i32, u32, u32) {
        if self.screens.is_empty() {
            return (0, 0, 0, 0);
        }
        let min_x = self.screens.iter().map(|s| s.x).min().unwrap_or(0);
        let min_y = self.screens.iter().map(|s| s.y).min().unwrap_or(0);
        let max_x = self
            .screens
            .iter()
            .map(|s| s.x + s.width as i32)
            .max()
            .unwrap_or(0);
        let max_y = self
            .screens
            .iter()
            .map(|s| s.y + s.height as i32)
            .max()
            .unwrap_or(0);
        (min_x, min_y, (max_x - min_x) as u32, (max_y - min_y) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framebuffer_new() {
        let fb = FrameBuffer::new(100, 50);
        assert_eq!(fb.width, 100);
        assert_eq!(fb.height, 50);
        assert_eq!(fb.stride, 100);
        assert_eq!(fb.data.len(), 100 * 50 * 4);
    }

    #[test]
    fn test_framebuffer_new_zero() {
        let fb = FrameBuffer::new(0, 0);
        assert_eq!(fb.width, 0);
        assert_eq!(fb.data.len(), 0);
    }

    #[test]
    fn test_pixel_at_default_is_zero() {
        let fb = FrameBuffer::new(10, 10);
        assert_eq!(fb.pixel_at(0, 0), [0, 0, 0, 0]);
        assert_eq!(fb.pixel_at(9, 9), [0, 0, 0, 0]);
    }

    #[test]
    fn test_pixel_at_out_of_bounds() {
        let fb = FrameBuffer::new(10, 10);
        assert_eq!(fb.pixel_at(100, 100), [0, 0, 0, 255]);
    }

    #[test]
    fn test_set_pixel_and_read_back() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.set_pixel(5, 5, [255, 128, 64, 32]);
        assert_eq!(fb.pixel_at(5, 5), [255, 128, 64, 32]);
    }

    #[test]
    fn test_set_pixel_corners() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.set_pixel(0, 0, [1, 2, 3, 4]);
        fb.set_pixel(9, 0, [5, 6, 7, 8]);
        fb.set_pixel(0, 9, [9, 10, 11, 12]);
        fb.set_pixel(9, 9, [13, 14, 15, 16]);
        assert_eq!(fb.pixel_at(0, 0), [1, 2, 3, 4]);
        assert_eq!(fb.pixel_at(9, 0), [5, 6, 7, 8]);
        assert_eq!(fb.pixel_at(0, 9), [9, 10, 11, 12]);
        assert_eq!(fb.pixel_at(9, 9), [13, 14, 15, 16]);
    }

    #[test]
    fn test_set_pixel_out_of_bounds_is_noop() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.set_pixel(100, 100, [255, 255, 255, 255]);
        // Should not panic, and data should be unchanged
        assert_eq!(fb.pixel_at(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn test_from_raw_ptr_null() {
        let fb = FrameBuffer::from_raw_ptr(100, 100, 100, 0);
        assert_eq!(fb.width, 100);
        assert_eq!(fb.data.len(), 100 * 100 * 4);
        assert_eq!(fb.pixel_at(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn test_from_raw_ptr_valid() {
        let src = vec![0xAAu8; 4 * 4 * 4]; // 4x4 framebuffer
        let ptr = src.as_ptr() as u64;
        let fb = FrameBuffer::from_raw_ptr(4, 4, 4, ptr);
        assert_eq!(fb.width, 4);
        assert_eq!(fb.height, 4);
        assert_eq!(fb.pixel_at(0, 0), [0xAA, 0xAA, 0xAA, 0xAA]);
    }

    #[test]
    fn test_copy_region_from_ptr_null_is_noop() {
        let mut fb = FrameBuffer::new(10, 10);
        fb.copy_region_from_ptr(0, 0, 5, 5, 0, 10);
        assert_eq!(fb.pixel_at(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn test_copy_region_from_ptr_valid() {
        let mut fb = FrameBuffer::new(10, 10);
        let src = vec![0xBBu8; 10 * 10 * 4];
        let ptr = src.as_ptr() as u64;
        fb.copy_region_from_ptr(0, 0, 3, 3, ptr, 10);
        assert_eq!(fb.pixel_at(0, 0), [0xBB, 0xBB, 0xBB, 0xBB]);
        assert_eq!(fb.pixel_at(2, 2), [0xBB, 0xBB, 0xBB, 0xBB]);
        // Outside the copied region should still be zero
        assert_eq!(fb.pixel_at(5, 5), [0, 0, 0, 0]);
    }

    #[test]
    fn test_cursor_data_fields() {
        let cursor = CursorData {
            width: 16,
            height: 16,
            hotspot_x: 8,
            hotspot_y: 8,
            pixels: vec![0u8; 16 * 16 * 4],
        };
        assert_eq!(cursor.width, 16);
        assert_eq!(cursor.hotspot_x, 8);
        assert_eq!(cursor.pixels.len(), 16 * 16 * 4);
    }

    #[test]
    fn test_large_framebuffer() {
        let fb = FrameBuffer::new(1920, 1080);
        assert_eq!(fb.data.len(), 1920 * 1080 * 4);
        assert_eq!(fb.pixel_at(1919, 1079), [0, 0, 0, 0]);
    }

    #[test]
    fn test_zoom_mode_fit() {
        let mode = ZoomMode::Fit;
        let scale = mode.compute_scale(1920, 1080, egui::Vec2::new(960.0, 540.0), 1.0);
        assert!((scale - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_zoom_mode_one_to_one() {
        let mode = ZoomMode::OneToOne;
        let scale = mode.compute_scale(1920, 1080, egui::Vec2::new(960.0, 540.0), 2.0);
        assert!((scale - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_zoom_mode_fixed() {
        let mode = ZoomMode::Fixed(1.5);
        let scale = mode.compute_scale(1920, 1080, egui::Vec2::new(960.0, 540.0), 1.0);
        assert!((scale - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_zoom_mode_label() {
        assert_eq!(ZoomMode::Fit.label(), "Fit");
        assert_eq!(ZoomMode::OneToOne.label(), "1:1");
        assert_eq!(ZoomMode::Fixed(0.5).label(), "50%");
        assert_eq!(ZoomMode::Fixed(2.0).label(), "200%");
    }

    #[test]
    fn test_zoom_mode_fit_preserves_aspect_ratio() {
        let mode = ZoomMode::Fit;
        // Wide window, tall framebuffer
        let scale = mode.compute_scale(100, 200, egui::Vec2::new(400.0, 200.0), 1.0);
        assert!((scale - 1.0).abs() < 0.001); // limited by height
    }

    #[test]
    fn test_screen_layout_default_empty() {
        let layout = ScreenLayout::default();
        assert_eq!(layout.screen_count(), 0);
        assert!(!layout.is_multi_monitor());
    }

    #[test]
    fn test_screen_layout_single() {
        let layout = ScreenLayout::single(1920, 1080);
        assert_eq!(layout.screen_count(), 1);
        assert!(!layout.is_multi_monitor());
        assert_eq!(layout.bounding_box(), (0, 0, 1920, 1080));
    }

    #[test]
    fn test_screen_layout_multi_monitor() {
        let layout = ScreenLayout {
            screens: vec![
                Screen {
                    id: 0,
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                Screen {
                    id: 1,
                    x: 1920,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
            ],
        };
        assert_eq!(layout.screen_count(), 2);
        assert!(layout.is_multi_monitor());
        assert_eq!(layout.bounding_box(), (0, 0, 3840, 1080));
    }

    #[test]
    fn test_screen_layout_bounding_box_offset() {
        let layout = ScreenLayout {
            screens: vec![
                Screen {
                    id: 0,
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                Screen {
                    id: 1,
                    x: 1920,
                    y: 200,
                    width: 1280,
                    height: 720,
                },
            ],
        };
        let (x, y, w, h) = layout.bounding_box();
        assert_eq!(x, 0);
        assert_eq!(y, 0);
        assert_eq!(w, 3200);
        assert_eq!(h, 1080);
    }

    #[test]
    fn test_screen_layout_empty_bounding_box() {
        let layout = ScreenLayout::default();
        assert_eq!(layout.bounding_box(), (0, 0, 0, 0));
    }
}
