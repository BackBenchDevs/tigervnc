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

mod gpu_renderer;
mod software_renderer;
mod types;

pub use gpu_renderer::GpuRenderer;
pub use software_renderer::SoftwareRenderer;
#[allow(unused_imports)]
pub use types::{CursorData, FrameBuffer, RenderBackend, Screen, ScreenLayout, ZoomMode};

use log::info;

#[allow(dead_code)]
pub enum Renderer {
    Gpu(GpuRenderer),
    Software(SoftwareRenderer),
}

impl Renderer {
    pub fn new() -> Self {
        info!("Initializing software renderer (GPU renderer requires window context)");
        Renderer::Software(SoftwareRenderer::new())
    }

    pub fn set_framebuffer(&mut self, fb: FrameBuffer) {
        match self {
            Renderer::Gpu(r) => r.set_framebuffer(fb),
            Renderer::Software(r) => r.set_framebuffer(fb),
        }
    }

    pub fn update_region(&mut self, x: i32, y: i32, w: i32, h: i32) {
        match self {
            Renderer::Gpu(r) => r.update_region(x, y, w, h),
            Renderer::Software(r) => r.update_region(x, y, w, h),
        }
    }

    pub fn set_cursor(&mut self, cursor: CursorData) {
        match self {
            Renderer::Gpu(r) => r.set_cursor(cursor),
            Renderer::Software(r) => r.set_cursor(cursor),
        }
    }

    pub fn render_to_egui(
        &mut self,
        ui: &mut egui::Ui,
        available_size: egui::Vec2,
    ) -> Option<(egui::Response, egui::Rect)> {
        match self {
            Renderer::Gpu(r) => r.render_to_egui(ui, available_size),
            Renderer::Software(r) => r.render_to_egui(ui, available_size),
        }
    }

    pub fn framebuffer_size(&self) -> (u32, u32) {
        match self {
            Renderer::Gpu(r) => r.framebuffer_size(),
            Renderer::Software(r) => r.framebuffer_size(),
        }
    }

    pub fn backend_name(&self) -> &str {
        match self {
            Renderer::Gpu(_) => "wgpu",
            Renderer::Software(_) => "softbuffer/tiny-skia",
        }
    }

    pub fn zoom_mode(&self) -> &ZoomMode {
        match self {
            Renderer::Gpu(r) => r.zoom_mode(),
            Renderer::Software(r) => r.zoom_mode(),
        }
    }

    pub fn set_zoom_mode(&mut self, mode: ZoomMode) {
        match self {
            Renderer::Gpu(r) => r.set_zoom_mode(mode),
            Renderer::Software(r) => r.set_zoom_mode(mode),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::new();
        assert_eq!(renderer.framebuffer_size(), (0, 0));
    }

    #[test]
    fn test_software_renderer_framebuffer() {
        let mut renderer = SoftwareRenderer::new();
        let fb = FrameBuffer {
            width: 800,
            height: 600,
            stride: 800,
            data: vec![0u8; 800 * 600 * 4],
        };
        renderer.set_framebuffer(fb);
        assert_eq!(renderer.framebuffer_size(), (800, 600));
    }
}
