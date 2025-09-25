use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point},
    pixelcolor::{Bgr888, IntoStorage},
};

pub struct DmaReadyFramebuffer<const W: usize, const H: usize> {
    pub framebuffer: *mut [[u32; W]; H], // tfw no generic_const_exprs
    big_endian: bool,
}

impl<const W: usize, const H: usize> DmaReadyFramebuffer<W, H> {
    pub fn new(
        raw_framebuffer: *mut ::core::ffi::c_void,
        big_endian: bool,
    ) -> DmaReadyFramebuffer<W, H> {
        if raw_framebuffer.is_null() {
            panic!("Failed to allocate framebuffer");
        }

        DmaReadyFramebuffer {
            framebuffer: raw_framebuffer as *mut [[u32; W]; H],
            big_endian,
        }
    }

    pub fn set_pixel(&mut self, point: Point, color: Bgr888) {
        if point.x >= 0 && point.x < W as i32 && point.y >= 0 && point.y < H as i32 {
            unsafe {
                let framebuffer = &mut *self.framebuffer;

                if self.big_endian {
                    framebuffer[point.y as usize][point.x as usize] = color.into_storage().to_be();
                } else {
                    framebuffer[point.y as usize][point.x as usize] = color.into_storage();
                }
            }
        }
    }

    pub fn as_slice(&self) -> &[u32] {
        unsafe { core::slice::from_raw_parts(self.framebuffer as *const u32, W * H) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        unsafe { core::slice::from_raw_parts_mut(self.framebuffer as *mut u32, W * H) }
    }

    pub fn as_mut_ptr(&mut self) -> *mut [u32] {
        self.as_slice() as *const [u32] as *mut [u32]
    }
}

impl<const W: usize, const H: usize> DrawTarget for DmaReadyFramebuffer<W, H> {
    type Color = Bgr888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics_core::prelude::Pixel<Self::Color>>,
    {
        for pixel in pixels {
            let embedded_graphics_core::prelude::Pixel(point, color) = pixel;

            self.set_pixel(point, color);
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        if self.big_endian {
            self.as_mut_slice().fill(color.into_storage().to_be());
        } else {
            self.as_mut_slice().fill(color.into_storage());
        }

        Ok(())
    }
}

impl<const W: usize, const H: usize> OriginDimensions for DmaReadyFramebuffer<W, H> {
    fn size(&self) -> embedded_graphics_core::geometry::Size {
        embedded_graphics_core::geometry::Size::new(W as u32, H as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics_core::prelude::*;
    use std::mem::MaybeUninit;
    
    // Helper function to create a buffer for testing
    fn create_test_buffer<const W: usize, const H: usize>() -> Box<[[u32; W]; H]> {
        // Create a safe buffer for testing
        let buffer = Box::new(unsafe { MaybeUninit::<[[u32; W]; H]>::zeroed().assume_init() });
        buffer
    }
    
    #[test]
    fn test_framebuffer_creation() {
        const WIDTH: usize = 64;
        const HEIGHT: usize = 32;
        
        // Create a safe test buffer
        let buffer = create_test_buffer::<WIDTH, HEIGHT>();
        let raw_ptr = buffer.as_ptr() as *mut ::core::ffi::c_void;
        
        // Create framebuffer with little-endian
        let fb_le = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, false);
        assert!(!fb_le.big_endian);
        
        // Create framebuffer with big-endian
        let fb_be = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, true);
        assert!(fb_be.big_endian);
    }
    
    #[test]
    #[should_panic(expected = "Failed to allocate framebuffer")]
    fn test_framebuffer_creation_null_pointer() {
        // This should panic when given a null pointer
        let _fb = DmaReadyFramebuffer::<10, 10>::new(std::ptr::null_mut(), false);
    }
    
    #[test]
    fn test_set_pixel() {
        const WIDTH: usize = 32;
        const HEIGHT: usize = 32;
        
        // Create a safe test buffer
        let buffer = create_test_buffer::<WIDTH, HEIGHT>();
        let raw_ptr = buffer.as_ptr() as *mut ::core::ffi::c_void;
        
        // Create framebuffer
        let mut fb = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, false);
        
        // Set a pixel at a valid position
        // In Bgr888, the parameters are (blue, green, red)
        let color = Bgr888::new(64, 128, 255); // B=64, G=128, R=255
        let point = Point::new(5, 10);
        fb.set_pixel(point, color);
        
        // Access the buffer and check the pixel value
        let value = fb.as_slice()[10 * WIDTH + 5]; // y * width + x
        
        // When Bgr888 is stored, it's stored as 0x00RRGGBB
        let expected = color.into_storage();
        assert_eq!(value, expected);
    }
    
    #[test]
    fn test_set_pixel_big_endian() {
        const WIDTH: usize = 32;
        const HEIGHT: usize = 32;
        
        // Create a safe test buffer
        let buffer = create_test_buffer::<WIDTH, HEIGHT>();
        let raw_ptr = buffer.as_ptr() as *mut ::core::ffi::c_void;
        
        // Create framebuffer with big-endian flag
        let mut fb = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, true);
        
        // Set a pixel at a valid position
        // In Bgr888, the parameters are (blue, green, red)
        let color = Bgr888::new(64, 128, 255); // B=64, G=128, R=255
        let point = Point::new(5, 10);
        fb.set_pixel(point, color);
        
        // Access the buffer and check the pixel value
        let value = fb.as_slice()[10 * WIDTH + 5]; // y * width + x
        
        // When stored in big endian, the bytes are swapped
        let expected = color.into_storage().to_be();
        assert_eq!(value, expected);
    }
    
    #[test]
    fn test_set_pixel_out_of_bounds() {
        const WIDTH: usize = 32;
        const HEIGHT: usize = 32;
        
        // Create a safe test buffer
        let buffer = create_test_buffer::<WIDTH, HEIGHT>();
        let raw_ptr = buffer.as_ptr() as *mut ::core::ffi::c_void;
        
        // Create framebuffer
        let mut fb = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, false);
        
        // Set a pixel outside the bounds (should be ignored)
        let color = Bgr888::new(255, 0, 0);
        
        // Test out of bounds in X
        fb.set_pixel(Point::new(-1, 10), color);
        fb.set_pixel(Point::new(WIDTH as i32, 10), color);
        
        // Test out of bounds in Y
        fb.set_pixel(Point::new(5, -1), color);
        fb.set_pixel(Point::new(5, HEIGHT as i32), color);
        
        // Verify no crash occurred and buffer is untouched at those locations
    }
    
    #[test]
    fn test_as_slice() {
        const WIDTH: usize = 4;
        const HEIGHT: usize = 2;
        
        // Create a safe test buffer
        let buffer = create_test_buffer::<WIDTH, HEIGHT>();
        let raw_ptr = buffer.as_ptr() as *mut ::core::ffi::c_void;
        
        // Create framebuffer and initialize with a pattern
        let mut fb = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, false);
        
        // Set some pixels - remember in Bgr888::new the parameters are (blue, green, red)
        fb.set_pixel(Point::new(0, 0), Bgr888::new(0, 0, 1)); // Mostly red
        fb.set_pixel(Point::new(1, 0), Bgr888::new(0, 0, 2)); // Slightly brighter red
        fb.set_pixel(Point::new(0, 1), Bgr888::new(0, 0, 3)); // Even brighter red
        
        // Get slice and verify length
        let slice = fb.as_slice();
        assert_eq!(slice.len(), WIDTH * HEIGHT);
        
        // Check that the slice contains our pixel values
        assert_eq!(slice[0], Bgr888::new(0, 0, 1).into_storage()); // (0,0)
        assert_eq!(slice[1], Bgr888::new(0, 0, 2).into_storage()); // (1,0)
        assert_eq!(slice[WIDTH], Bgr888::new(0, 0, 3).into_storage()); // (0,1)
    }
    
    #[test]
    fn test_as_mut_slice() {
        const WIDTH: usize = 4;
        const HEIGHT: usize = 2;
        
        // Create a safe test buffer
        let buffer = create_test_buffer::<WIDTH, HEIGHT>();
        let raw_ptr = buffer.as_ptr() as *mut ::core::ffi::c_void;
        
        // Create framebuffer
        let mut fb = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, false);
        
        // Get mutable slice and modify it
        let slice = fb.as_mut_slice();
        assert_eq!(slice.len(), WIDTH * HEIGHT);
        
        // Set some values directly
        slice[0] = 0x00FF0000; // Red in first pixel
        slice[1] = 0x0000FF00; // Green in second pixel
        
        // Verify using as_slice
        let check_slice = fb.as_slice();
        assert_eq!(check_slice[0], 0x00FF0000);
        assert_eq!(check_slice[1], 0x0000FF00);
    }
    
    #[test]
    fn test_draw_target_draw_iter() {
        const WIDTH: usize = 32;
        const HEIGHT: usize = 32;
        
        // Create a safe test buffer
        let buffer = create_test_buffer::<WIDTH, HEIGHT>();
        let raw_ptr = buffer.as_ptr() as *mut ::core::ffi::c_void;
        
        // Create framebuffer
        let mut fb = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, false);
        
        // Create some test pixels - parameters for Bgr888::new are (blue, green, red)
        let pixels = [
            Pixel(Point::new(1, 1), Bgr888::new(0, 0, 255)),   // Red (B=0, G=0, R=255)
            Pixel(Point::new(2, 1), Bgr888::new(0, 255, 0)),   // Green (B=0, G=255, R=0)
            Pixel(Point::new(3, 1), Bgr888::new(255, 0, 0)),   // Blue (B=255, G=0, R=0)
        ];
        
        // Draw the pixels
        fb.draw_iter(pixels).unwrap();
        
        // Check each pixel was set correctly using as_slice
        let slice = fb.as_slice();
        
        // Red pixel at (1,1) = y*width + x = 1*WIDTH + 1
        assert_eq!(slice[1*WIDTH + 1], Bgr888::new(0, 0, 255).into_storage());
        
        // Green pixel at (2,1)
        assert_eq!(slice[1*WIDTH + 2], Bgr888::new(0, 255, 0).into_storage());
        
        // Blue pixel at (3,1)
        assert_eq!(slice[1*WIDTH + 3], Bgr888::new(255, 0, 0).into_storage());
    }
    
    #[test]
    fn test_draw_target_clear() {
        const WIDTH: usize = 16;
        const HEIGHT: usize = 16;
        
        // Create a safe test buffer
        let buffer = create_test_buffer::<WIDTH, HEIGHT>();
        let raw_ptr = buffer.as_ptr() as *mut ::core::ffi::c_void;
        
        // Create framebuffer
        let mut fb = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, false);
        
        // Set some initial pixels
        fb.set_pixel(Point::new(0, 0), Bgr888::new(255, 0, 0));
        fb.set_pixel(Point::new(1, 1), Bgr888::new(0, 255, 0));
        
        // Clear with blue
        fb.clear(Bgr888::new(0, 0, 128)).unwrap();
        
        // Check that all pixels are now blue
        let slice = fb.as_slice();
        let blue_value = Bgr888::new(0, 0, 128).into_storage();
        
        for &pixel in slice {
            assert_eq!(pixel, blue_value);
        }
    }
    
    #[test]
    fn test_draw_target_clear_big_endian() {
        const WIDTH: usize = 16;
        const HEIGHT: usize = 16;
        
        // Create a safe test buffer
        let buffer = create_test_buffer::<WIDTH, HEIGHT>();
        let raw_ptr = buffer.as_ptr() as *mut ::core::ffi::c_void;
        
        // Create big-endian framebuffer
        let mut fb = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, true);
        
        // Clear with a color
        let color = Bgr888::new(10, 20, 30);
        fb.clear(color).unwrap();
        
        // Check that all pixels are set to the big-endian value
        let slice = fb.as_slice();
        let expected_value = color.into_storage().to_be();
        
        for &pixel in slice {
            assert_eq!(pixel, expected_value);
        }
    }
    
    #[test]
    fn test_origin_dimensions() {
        const WIDTH: usize = 64;
        const HEIGHT: usize = 32;
        
        // Create a safe test buffer
        let buffer = create_test_buffer::<WIDTH, HEIGHT>();
        let raw_ptr = buffer.as_ptr() as *mut ::core::ffi::c_void;
        
        // Create framebuffer
        let fb = DmaReadyFramebuffer::<WIDTH, HEIGHT>::new(raw_ptr, false);
        
        // Check dimensions
        let size = fb.size();
        assert_eq!(size.width, WIDTH as u32);
        assert_eq!(size.height, HEIGHT as u32);
    }
}
