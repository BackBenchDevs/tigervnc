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

#[allow(dead_code)]
pub trait RenderBackend {
    fn set_framebuffer(&mut self, fb: FrameBuffer);
    fn update_region(&mut self, x: i32, y: i32, w: i32, h: i32);
    fn set_cursor(&mut self, cursor: CursorData);
    fn render_to_egui(
        &mut self,
        ui: &mut egui::Ui,
        available_size: egui::Vec2,
    ) -> Option<egui::Response>;
    fn framebuffer_size(&self) -> (u32, u32);
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
}
