use crate::renderer::{CursorData, FrameBuffer, Renderer};
use egui::{Key, PointerButton, Ui};

pub struct SessionView {
    renderer: Renderer,
    frame_count: u64,
    mouse_x: i32,
    mouse_y: i32,
    button_mask: u8,
    pending_keys: Vec<KeyEvent>,
    pending_pointer: Option<PointerEvent>,
    pending_clipboard: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub key_code: u32,
    pub key_sym: u32,
    pub pressed: bool,
}

#[derive(Debug, Clone)]
pub struct PointerEvent {
    pub x: i32,
    pub y: i32,
    pub button_mask: u8,
}

impl SessionView {
    pub fn new() -> Self {
        Self {
            renderer: Renderer::new(),
            frame_count: 0,
            mouse_x: 0,
            mouse_y: 0,
            button_mask: 0,
            pending_keys: Vec::new(),
            pending_pointer: None,
            pending_clipboard: None,
        }
    }

    pub fn set_framebuffer(&mut self, fb: FrameBuffer) {
        self.renderer.set_framebuffer(fb);
    }

    pub fn update_region(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.renderer.update_region(x, y, w, h);
        self.frame_count += 1;
    }

    pub fn set_cursor(&mut self, cursor: CursorData) {
        self.renderer.set_cursor(cursor);
    }

    #[allow(dead_code)]
    pub fn framebuffer_size(&self) -> (u32, u32) {
        self.renderer.framebuffer_size()
    }

    #[allow(dead_code)]
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn take_pending_keys(&mut self) -> Vec<KeyEvent> {
        std::mem::take(&mut self.pending_keys)
    }

    pub fn take_pending_pointer(&mut self) -> Option<PointerEvent> {
        self.pending_pointer.take()
    }

    pub fn take_pending_clipboard(&mut self) -> Option<String> {
        self.pending_clipboard.take()
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let available = ui.available_size();
        let (fb_w, fb_h) = self.renderer.framebuffer_size();

        if fb_w == 0 || fb_h == 0 {
            ui.vertical_centered(|ui| {
                ui.add_space(available.y / 3.0);
                ui.spinner();
                ui.label("Waiting for framebuffer...");
            });
            return;
        }

        let response = self.renderer.render_to_egui(ui, available);

        if let Some(response) = response {
            self.handle_input(ui, &response, fb_w, fb_h);

            let rect = response.rect;
            let painter = ui.painter_at(rect);
            painter.text(
                egui::pos2(rect.left() + 4.0, rect.bottom() - 16.0),
                egui::Align2::LEFT_BOTTOM,
                format!(
                    "{}x{} | frame #{} | {}",
                    fb_w,
                    fb_h,
                    self.frame_count,
                    self.renderer.backend_name()
                ),
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgba_premultiplied(150, 150, 150, 128),
            );
        }
    }

    fn handle_input(
        &mut self,
        ui: &mut Ui,
        response: &egui::Response,
        fb_w: u32,
        fb_h: u32,
    ) {
        if let Some(pos) = response.hover_pos() {
            let rect = response.rect;
            let scale_x = fb_w as f32 / rect.width();
            let scale_y = fb_h as f32 / rect.height();
            let rel_x = pos.x - rect.left();
            let rel_y = pos.y - rect.top();
            self.mouse_x = (rel_x * scale_x).clamp(0.0, fb_w as f32 - 1.0) as i32;
            self.mouse_y = (rel_y * scale_y).clamp(0.0, fb_h as f32 - 1.0) as i32;
        }

        let mut new_mask = 0u8;
        if ui.input(|i| i.pointer.button_down(PointerButton::Primary)) {
            new_mask |= 1;
        }
        if ui.input(|i| i.pointer.button_down(PointerButton::Middle)) {
            new_mask |= 2;
        }
        if ui.input(|i| i.pointer.button_down(PointerButton::Secondary)) {
            new_mask |= 4;
        }

        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll > 0.0 {
            new_mask |= 8;
        } else if scroll < 0.0 {
            new_mask |= 16;
        }

        if new_mask != self.button_mask || response.hovered() {
            self.button_mask = new_mask;
            self.pending_pointer = Some(PointerEvent {
                x: self.mouse_x,
                y: self.mouse_y,
                button_mask: self.button_mask,
            });
        }

        ui.input(|input| {
            for event in &input.events {
                match event {
                    egui::Event::Key {
                        key,
                        pressed,
                        ..
                    } => {
                        if let Some(sym) = egui_key_to_keysym(key) {
                            self.pending_keys.push(KeyEvent {
                                key_code: sym,
                                key_sym: sym,
                                pressed: *pressed,
                            });
                        }
                    }
                    egui::Event::Text(text) => {
                        for ch in text.chars() {
                            let sym = ch as u32;
                            self.pending_keys.push(KeyEvent {
                                key_code: sym,
                                key_sym: sym,
                                pressed: true,
                            });
                            self.pending_keys.push(KeyEvent {
                                key_code: sym,
                                key_sym: sym,
                                pressed: false,
                            });
                        }
                    }
                    egui::Event::Paste(text) => {
                        self.pending_clipboard = Some(text.clone());
                    }
                    _ => {}
                }
            }
        });
    }
}

fn egui_key_to_keysym(key: &Key) -> Option<u32> {
    match key {
        Key::Escape => Some(0xff1b),
        Key::Tab => Some(0xff09),
        Key::Backspace => Some(0xff08),
        Key::Enter => Some(0xff0d),
        Key::Space => Some(0x0020),
        Key::Insert => Some(0xff63),
        Key::Delete => Some(0xffff),
        Key::Home => Some(0xff50),
        Key::End => Some(0xff57),
        Key::PageUp => Some(0xff55),
        Key::PageDown => Some(0xff56),
        Key::ArrowLeft => Some(0xff51),
        Key::ArrowUp => Some(0xff52),
        Key::ArrowRight => Some(0xff53),
        Key::ArrowDown => Some(0xff54),
        Key::F1 => Some(0xffbe),
        Key::F2 => Some(0xffbf),
        Key::F3 => Some(0xffc0),
        Key::F4 => Some(0xffc1),
        Key::F5 => Some(0xffc2),
        Key::F6 => Some(0xffc3),
        Key::F7 => Some(0xffc4),
        Key::F8 => Some(0xffc5),
        Key::F9 => Some(0xffc6),
        Key::F10 => Some(0xffc7),
        Key::F11 => Some(0xffc8),
        Key::F12 => Some(0xffc9),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_view_init() {
        let sv = SessionView::new();
        assert_eq!(sv.framebuffer_size(), (0, 0));
        assert_eq!(sv.frame_count(), 0);
        assert_eq!(sv.mouse_x, 0);
        assert_eq!(sv.mouse_y, 0);
        assert_eq!(sv.button_mask, 0);
    }

    #[test]
    fn test_set_framebuffer() {
        let mut sv = SessionView::new();
        sv.set_framebuffer(FrameBuffer::new(1920, 1080));
        assert_eq!(sv.framebuffer_size(), (1920, 1080));
    }

    #[test]
    fn test_update_region_increments_frame_count() {
        let mut sv = SessionView::new();
        assert_eq!(sv.frame_count(), 0);
        sv.update_region(0, 0, 100, 100);
        assert_eq!(sv.frame_count(), 1);
        sv.update_region(0, 0, 50, 50);
        assert_eq!(sv.frame_count(), 2);
    }

    #[test]
    fn test_take_pending_keys_empty() {
        let mut sv = SessionView::new();
        assert!(sv.take_pending_keys().is_empty());
    }

    #[test]
    fn test_take_pending_keys_consumes() {
        let mut sv = SessionView::new();
        sv.pending_keys.push(KeyEvent {
            key_code: 0x41,
            key_sym: 0x41,
            pressed: true,
        });
        let keys = sv.take_pending_keys();
        assert_eq!(keys.len(), 1);
        assert!(sv.take_pending_keys().is_empty());
    }

    #[test]
    fn test_take_pending_pointer_empty() {
        let mut sv = SessionView::new();
        assert!(sv.take_pending_pointer().is_none());
    }

    #[test]
    fn test_take_pending_pointer_consumes() {
        let mut sv = SessionView::new();
        sv.pending_pointer = Some(PointerEvent {
            x: 100,
            y: 200,
            button_mask: 1,
        });
        let ptr = sv.take_pending_pointer().unwrap();
        assert_eq!(ptr.x, 100);
        assert_eq!(ptr.y, 200);
        assert_eq!(ptr.button_mask, 1);
        assert!(sv.take_pending_pointer().is_none());
    }

    #[test]
    fn test_take_pending_clipboard_empty() {
        let mut sv = SessionView::new();
        assert!(sv.take_pending_clipboard().is_none());
    }

    #[test]
    fn test_take_pending_clipboard_consumes() {
        let mut sv = SessionView::new();
        sv.pending_clipboard = Some("pasted text".to_string());
        let text = sv.take_pending_clipboard().unwrap();
        assert_eq!(text, "pasted text");
        assert!(sv.take_pending_clipboard().is_none());
    }

    // --- Keysym mapping tests ---

    #[test]
    fn test_keysym_escape() {
        assert_eq!(egui_key_to_keysym(&Key::Escape), Some(0xff1b));
    }

    #[test]
    fn test_keysym_tab() {
        assert_eq!(egui_key_to_keysym(&Key::Tab), Some(0xff09));
    }

    #[test]
    fn test_keysym_backspace() {
        assert_eq!(egui_key_to_keysym(&Key::Backspace), Some(0xff08));
    }

    #[test]
    fn test_keysym_enter() {
        assert_eq!(egui_key_to_keysym(&Key::Enter), Some(0xff0d));
    }

    #[test]
    fn test_keysym_space() {
        assert_eq!(egui_key_to_keysym(&Key::Space), Some(0x0020));
    }

    #[test]
    fn test_keysym_arrows() {
        assert_eq!(egui_key_to_keysym(&Key::ArrowLeft), Some(0xff51));
        assert_eq!(egui_key_to_keysym(&Key::ArrowUp), Some(0xff52));
        assert_eq!(egui_key_to_keysym(&Key::ArrowRight), Some(0xff53));
        assert_eq!(egui_key_to_keysym(&Key::ArrowDown), Some(0xff54));
    }

    #[test]
    fn test_keysym_function_keys() {
        assert_eq!(egui_key_to_keysym(&Key::F1), Some(0xffbe));
        assert_eq!(egui_key_to_keysym(&Key::F12), Some(0xffc9));
    }

    #[test]
    fn test_keysym_navigation() {
        assert_eq!(egui_key_to_keysym(&Key::Home), Some(0xff50));
        assert_eq!(egui_key_to_keysym(&Key::End), Some(0xff57));
        assert_eq!(egui_key_to_keysym(&Key::PageUp), Some(0xff55));
        assert_eq!(egui_key_to_keysym(&Key::PageDown), Some(0xff56));
        assert_eq!(egui_key_to_keysym(&Key::Insert), Some(0xff63));
        assert_eq!(egui_key_to_keysym(&Key::Delete), Some(0xffff));
    }

    #[test]
    fn test_keysym_f_keys_sequential() {
        let f_keys = [
            Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6,
            Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12,
        ];
        for (i, key) in f_keys.iter().enumerate() {
            let expected = 0xffbe + i as u32;
            assert_eq!(
                egui_key_to_keysym(key),
                Some(expected),
                "F{} should map to {:#x}",
                i + 1,
                expected
            );
        }
    }

    #[test]
    fn test_key_event_debug() {
        let ke = KeyEvent {
            key_code: 0x41,
            key_sym: 0x41,
            pressed: true,
        };
        let dbg = format!("{:?}", ke);
        assert!(dbg.contains("key_code"));
        assert!(dbg.contains("pressed"));
    }

    #[test]
    fn test_pointer_event_debug() {
        let pe = PointerEvent {
            x: 100,
            y: 200,
            button_mask: 3,
        };
        let dbg = format!("{:?}", pe);
        assert!(dbg.contains("100"));
        assert!(dbg.contains("200"));
    }
}
