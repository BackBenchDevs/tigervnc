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
use log::info;

/// GPU-accelerated renderer using wgpu for texture upload.
///
/// In the current implementation, this uses egui's built-in texture system
/// (which itself uses wgpu/glow under the hood via eframe). For a future
/// zero-copy path, this would create a wgpu texture directly and render
/// it as a custom paint callback.
#[allow(dead_code)]
pub struct GpuRenderer {
    framebuffer: Option<FrameBuffer>,
    cursor: Option<CursorData>,
    texture: Option<TextureHandle>,
    dirty: bool,
    zoom_mode: ZoomMode,
}

#[allow(dead_code)]
impl GpuRenderer {
    pub fn new() -> Self {
        info!("GPU renderer initialized");
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
                    let b = fb.data[offset];
                    let g = fb.data[offset + 1];
                    let r = fb.data[offset + 2];
                    pixels.push(Color32::from_rgb(r, g, b));
                } else {
                    pixels.push(Color32::BLACK);
                }
            }
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
                self.texture = Some(
                    ui.ctx()
                        .load_texture("vnc_gpu_framebuffer", image, tex_opts),
                );
            }
        }

        self.dirty = false;
    }
}

impl RenderBackend for GpuRenderer {
    fn set_framebuffer(&mut self, fb: FrameBuffer) {
        self.framebuffer = Some(fb);
        self.dirty = true;
    }

    fn update_region(&mut self, _x: i32, _y: i32, _w: i32, _h: i32) {
        // For partial updates, a future version would use wgpu's
        // write_texture with a sub-region. For now, mark full dirty.
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
            None => return None,
        };

        let (fb_w, fb_h) = self.framebuffer_size();
        if fb_w == 0 || fb_h == 0 {
            return None;
        }

        let ppp = ui.ctx().pixels_per_point();
        let scale = self
            .zoom_mode
            .compute_scale(fb_w, fb_h, available_size, ppp);

        let display_size = Vec2::new(fb_w as f32 * scale, fb_h as f32 * scale);

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
    fn test_gpu_renderer_init() {
        let r = GpuRenderer::new();
        assert_eq!(r.framebuffer_size(), (0, 0));
    }
}
