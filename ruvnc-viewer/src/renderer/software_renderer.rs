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

use super::types::{CursorData, FrameBuffer, RenderBackend, ZoomMode};
use egui::{Color32, ColorImage, TextureHandle, TextureOptions, Ui, Vec2};

pub struct SoftwareRenderer {
    framebuffer: Option<FrameBuffer>,
    cursor: Option<CursorData>,
    texture: Option<TextureHandle>,
    dirty: bool,
    zoom_mode: ZoomMode,
}

impl SoftwareRenderer {
    pub fn new() -> Self {
        Self {
            framebuffer: None,
            cursor: None,
            texture: None,
            dirty: true,
            zoom_mode: ZoomMode::Fit,
        }
    }

    fn rebuild_texture(&mut self, ui: &mut Ui) {
        let fb = match &self.framebuffer {
            Some(fb) if fb.width > 0 && fb.height > 0 => fb,
            _ => return,
        };

        let w = fb.width as usize;
        let h = fb.height as usize;
        let stride = fb.stride as usize;

        let mut pixels = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let offset = (y * stride + x) * 4;
                if offset + 3 < fb.data.len() {
                    // C++ pixel format: BGRX (32bpp, 24 depth, little-endian, true-color)
                    // egui expects RGBA
                    let b = fb.data[offset];
                    let g = fb.data[offset + 1];
                    let r = fb.data[offset + 2];
                    pixels.push(Color32::from_rgb(r, g, b));
                } else {
                    pixels.push(Color32::BLACK);
                }
            }
        }

        // Composite software cursor on top
        if let Some(ref cursor) = self.cursor {
            self.composite_cursor(&mut pixels, w, h, cursor);
        }

        let image = ColorImage {
            size: [w, h],
            pixels,
        };

        let tex_opts = TextureOptions::LINEAR;
        match &mut self.texture {
            Some(tex) => {
                tex.set(image, tex_opts);
            }
            None => {
                self.texture = Some(ui.ctx().load_texture("vnc_framebuffer", image, tex_opts));
            }
        }

        self.dirty = false;
    }

    fn composite_cursor(
        &self,
        pixels: &mut [Color32],
        fb_w: usize,
        fb_h: usize,
        cursor: &CursorData,
    ) {
        let cw = cursor.width as usize;
        let ch = cursor.height as usize;

        for cy in 0..ch {
            for cx in 0..cw {
                let src_offset = (cy * cw + cx) * 4;
                if src_offset + 3 >= cursor.pixels.len() {
                    continue;
                }

                let alpha = cursor.pixels[src_offset + 3];
                if alpha == 0 {
                    continue;
                }

                let dx = cx as i32 - cursor.hotspot_x;
                let dy = cy as i32 - cursor.hotspot_y;

                if dx < 0 || dy < 0 || dx >= fb_w as i32 || dy >= fb_h as i32 {
                    continue;
                }

                let dst_idx = dy as usize * fb_w + dx as usize;
                if dst_idx < pixels.len() {
                    let r = cursor.pixels[src_offset];
                    let g = cursor.pixels[src_offset + 1];
                    let b = cursor.pixels[src_offset + 2];

                    if alpha == 255 {
                        pixels[dst_idx] = Color32::from_rgb(r, g, b);
                    } else {
                        let bg = pixels[dst_idx];
                        let a = alpha as f32 / 255.0;
                        let blend = |fg: u8, bg: u8| -> u8 {
                            (fg as f32 * a + bg as f32 * (1.0 - a)) as u8
                        };
                        pixels[dst_idx] =
                            Color32::from_rgb(blend(r, bg.r()), blend(g, bg.g()), blend(b, bg.b()));
                    }
                }
            }
        }
    }
}

impl RenderBackend for SoftwareRenderer {
    fn set_framebuffer(&mut self, fb: FrameBuffer) {
        self.framebuffer = Some(fb);
        self.dirty = true;
    }

    fn update_region(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) {
        self.dirty = true;
    }

    fn set_cursor(&mut self, cursor: CursorData) {
        self.cursor = Some(cursor);
        self.dirty = true;
    }

    fn render_to_egui(
        &mut self,
        ui: &mut Ui,
        available_size: Vec2,
    ) -> Option<(egui::Response, egui::Rect)> {
        if self.dirty {
            self.rebuild_texture(ui);
        }

        let texture = match &self.texture {
            Some(t) => t,
            None => {
                ui.label("No framebuffer");
                return None;
            }
        };

        let (fb_w, fb_h) = self.framebuffer_size();
        if fb_w == 0 || fb_h == 0 {
            return None;
        }

        let ppp = ui.ctx().pixels_per_point();
        let scale = self
            .zoom_mode
            .compute_scale(fb_w, fb_h, available_size, ppp);

        let display_w = fb_w as f32 * scale;
        let display_h = fb_h as f32 * scale;
        let display_size = Vec2::new(display_w, display_h);

        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());

        let center = response.rect.center();
        let img_rect = egui::Rect::from_center_size(center, display_size);

        painter.rect_filled(response.rect, 0.0, Color32::BLACK);

        painter.image(
            texture.id(),
            img_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        Some((response, img_rect))
    }

    fn zoom_mode(&self) -> &ZoomMode {
        &self.zoom_mode
    }

    fn set_zoom_mode(&mut self, mode: ZoomMode) {
        self.zoom_mode = mode;
    }

    fn framebuffer_size(&self) -> (u32, u32) {
        match &self.framebuffer {
            Some(fb) => (fb.width, fb.height),
            None => (0, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_software_renderer_init() {
        let r = SoftwareRenderer::new();
        assert_eq!(r.framebuffer_size(), (0, 0));
        assert!(r.dirty);
    }

    #[test]
    fn test_set_framebuffer() {
        let mut r = SoftwareRenderer::new();
        r.set_framebuffer(FrameBuffer::new(1920, 1080));
        assert_eq!(r.framebuffer_size(), (1920, 1080));
        assert!(r.dirty);
    }

    #[test]
    fn test_cursor_composite() {
        let r = SoftwareRenderer::new();
        let mut pixels = vec![Color32::BLACK; 100];
        let cursor = CursorData {
            width: 2,
            height: 2,
            hotspot_x: 0,
            hotspot_y: 0,
            pixels: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        };
        r.composite_cursor(&mut pixels, 10, 10, &cursor);
        assert_eq!(pixels[0], Color32::from_rgb(255, 0, 0));
        assert_eq!(pixels[1], Color32::from_rgb(0, 255, 0));
    }
}
