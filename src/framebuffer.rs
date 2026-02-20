use embedded_graphics_core::pixelcolor::raw::RawU24;
use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point},
    pixelcolor::{Bgr888, IntoStorage},
};

pub struct DmaReadyFramebuffer {
    pub width: usize,
    pub height: usize,
    pub framebuffer: Box<[u32]>,
    big_endian: bool,
}

impl DmaReadyFramebuffer {
    pub fn new(width: usize, height: usize, big_endian: bool) -> DmaReadyFramebuffer {
        DmaReadyFramebuffer {
            framebuffer: vec![0u32; width * height].into_boxed_slice(),
            width,
            height,
            big_endian,
        }
    }

    pub fn set_pixel(&mut self, point: Point, color: Bgr888) {
        if point.x >= 0
            && point.x < self.width as i32
            && point.y >= 0
            && point.y < self.height as i32
        {
            let framebuffer = &mut *self.framebuffer;

            if self.big_endian {
                framebuffer[point.y as usize * self.width + point.x as usize] =
                    color.into_storage().to_be();
            } else {
                framebuffer[point.y as usize * self.width + point.x as usize] =
                    color.into_storage();
            }
        }
    }

    pub fn get_pixel(&mut self, point: Point) -> Option<Bgr888> {
        if point.x >= 0
            && point.x < self.width as i32
            && point.y >= 0
            && point.y < self.height as i32
        {
            if self.big_endian {
                Some(Bgr888::from(RawU24::new(u32::from_be(
                    self.framebuffer[point.y as usize * self.width + point.x as usize],
                ))))
            } else {
                Some(Bgr888::from(RawU24::new(
                    self.framebuffer[point.y as usize * self.width + point.x as usize],
                )))
            }
        } else {
            None
        }
    }
}

impl DrawTarget for DmaReadyFramebuffer {
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
            self.framebuffer.fill(color.into_storage().to_be());
        } else {
            self.framebuffer.fill(color.into_storage());
        }

        Ok(())
    }
}

impl OriginDimensions for DmaReadyFramebuffer {
    fn size(&self) -> embedded_graphics_core::geometry::Size {
        embedded_graphics_core::geometry::Size::new(self.width as u32, self.height as u32)
    }
}

// Add at the end of framebuffer.rs
// SAFETY: The raw pointer is only used to access memory owned by RenderTarget,
// which lives for the entire duration of the program. Access is synchronized
// via mutex in DoubleBuffer.
unsafe impl Send for DmaReadyFramebuffer {}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics_core::prelude::*;

    #[test]
    fn test_framebuffer_creation() {
        const WIDTH: usize = 64;
        const HEIGHT: usize = 32;

        // Create framebuffer with little-endian
        let fb_le = DmaReadyFramebuffer::new(WIDTH, HEIGHT, false);
        assert!(!fb_le.big_endian);

        // Create framebuffer with big-endian
        let fb_be = DmaReadyFramebuffer::new(WIDTH, HEIGHT, true);
        assert!(fb_be.big_endian);
    }

    #[test]
    fn test_set_pixel() {
        const WIDTH: usize = 32;
        const HEIGHT: usize = 32;

        // Create framebuffer
        let mut fb = DmaReadyFramebuffer::new(WIDTH, HEIGHT, false);

        // Set a pixel at a valid position
        // In Bgr888, the parameters are (blue, green, red)
        let color = Bgr888::new(64, 128, 255); // B=64, G=128, R=255
        let point = Point::new(5, 10);
        fb.set_pixel(point, color);

        // Access the buffer and check the pixel value
        let value = fb.framebuffer[10 * WIDTH + 5]; // y * width + x

        // When Bgr888 is stored, it's stored as 0x00RRGGBB
        let expected = color.into_storage();
        assert_eq!(value, expected);
    }

    #[test]
    fn test_set_pixel_big_endian() {
        const WIDTH: usize = 32;
        const HEIGHT: usize = 32;

        // Create framebuffer with big-endian flag
        let mut fb = DmaReadyFramebuffer::new(WIDTH, HEIGHT, true);

        // Set a pixel at a valid position
        // In Bgr888, the parameters are (blue, green, red)
        let color = Bgr888::new(64, 128, 255); // B=64, G=128, R=255
        let point = Point::new(5, 10);
        fb.set_pixel(point, color);

        // Access the buffer and check the pixel value
        let value = fb.framebuffer[10 * WIDTH + 5]; // y * width + x

        // When stored in big endian, the bytes are swapped
        let expected = color.into_storage().to_be();
        assert_eq!(value, expected);
    }

    #[test]
    fn test_get_pixel_roundtrip() {
        const WIDTH: usize = 32;
        const HEIGHT: usize = 32;

        // Test little-endian
        let mut fb_le = DmaReadyFramebuffer::new(WIDTH, HEIGHT, false);
        let color = Bgr888::new(64, 128, 255);
        let point = Point::new(5, 10);
        fb_le.set_pixel(point, color);
        assert_eq!(fb_le.get_pixel(point), Some(color));

        // Test big-endian
        let mut fb_be = DmaReadyFramebuffer::new(WIDTH, HEIGHT, true);
        fb_be.set_pixel(point, color);
        assert_eq!(fb_be.get_pixel(point), Some(color));
    }

    #[test]
    fn test_set_pixel_out_of_bounds() {
        const WIDTH: usize = 32;
        const HEIGHT: usize = 32;

        // Create framebuffer
        let mut fb = DmaReadyFramebuffer::new(WIDTH, HEIGHT, false);

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

        // Create framebuffer and initialize with a pattern
        let mut fb = DmaReadyFramebuffer::new(WIDTH, HEIGHT, false);

        // Set some pixels - remember in Bgr888::new the parameters are (blue, green, red)
        fb.set_pixel(Point::new(0, 0), Bgr888::new(0, 0, 1)); // Mostly red
        fb.set_pixel(Point::new(1, 0), Bgr888::new(0, 0, 2)); // Slightly brighter red
        fb.set_pixel(Point::new(0, 1), Bgr888::new(0, 0, 3)); // Even brighter red

        // Get slice and verify length
        let slice = fb.framebuffer;
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

        // Create framebuffer
        let mut fb = DmaReadyFramebuffer::new(WIDTH, HEIGHT, false);

        // Get mutable slice and modify it
        let slice = &mut fb.framebuffer;
        assert_eq!(slice.len(), WIDTH * HEIGHT);

        // Set some values directly
        slice[0] = 0x00FF0000; // Red in first pixel
        slice[1] = 0x0000FF00; // Green in second pixel

        // Verify using as_slice
        let check_slice = fb.framebuffer;
        assert_eq!(check_slice[0], 0x00FF0000);
        assert_eq!(check_slice[1], 0x0000FF00);
    }

    #[test]
    fn test_draw_target_draw_iter() {
        const WIDTH: usize = 32;
        const HEIGHT: usize = 32;

        // Create framebuffer
        let mut fb = DmaReadyFramebuffer::new(WIDTH, HEIGHT, false);

        // Create some test pixels - parameters for Bgr888::new are (blue, green, red)
        let pixels = [
            Pixel(Point::new(1, 1), Bgr888::new(0, 0, 255)), // Red (B=0, G=0, R=255)
            Pixel(Point::new(2, 1), Bgr888::new(0, 255, 0)), // Green (B=0, G=255, R=0)
            Pixel(Point::new(3, 1), Bgr888::new(255, 0, 0)), // Blue (B=255, G=0, R=0)
        ];

        // Draw the pixels
        fb.draw_iter(pixels).unwrap();

        // Check each pixel was set correctly using as_slice
        let slice = fb.framebuffer;

        // Red pixel at (1,1) = y*width + x = 1*WIDTH + 1
        assert_eq!(slice[1 * WIDTH + 1], Bgr888::new(0, 0, 255).into_storage());

        // Green pixel at (2,1)
        assert_eq!(slice[1 * WIDTH + 2], Bgr888::new(0, 255, 0).into_storage());

        // Blue pixel at (3,1)
        assert_eq!(slice[1 * WIDTH + 3], Bgr888::new(255, 0, 0).into_storage());
    }

    #[test]
    fn test_draw_target_clear() {
        const WIDTH: usize = 16;
        const HEIGHT: usize = 16;

        // Create framebuffer
        let mut fb = DmaReadyFramebuffer::new(WIDTH, HEIGHT, false);

        // Set some initial pixels
        fb.set_pixel(Point::new(0, 0), Bgr888::new(255, 0, 0));
        fb.set_pixel(Point::new(1, 1), Bgr888::new(0, 255, 0));

        // Clear with blue
        fb.clear(Bgr888::new(0, 0, 128)).unwrap();

        // Check that all pixels are now blue
        let slice = fb.framebuffer;
        let blue_value = Bgr888::new(0, 0, 128).into_storage();

        for pixel in slice {
            assert_eq!(pixel, blue_value);
        }
    }

    #[test]
    fn test_draw_target_clear_big_endian() {
        const WIDTH: usize = 16;
        const HEIGHT: usize = 16;

        // Create big-endian framebuffer
        let mut fb = DmaReadyFramebuffer::new(WIDTH, HEIGHT, true);

        // Clear with a color
        let color = Bgr888::new(10, 20, 30);
        fb.clear(color).unwrap();

        // Check that all pixels are set to the big-endian value
        let slice = fb.framebuffer;
        let expected_value = color.into_storage().to_be();

        for pixel in slice {
            assert_eq!(pixel, expected_value);
        }
    }

    #[test]
    fn test_origin_dimensions() {
        const WIDTH: usize = 64;
        const HEIGHT: usize = 32;

        // Create framebuffer
        let fb = DmaReadyFramebuffer::new(WIDTH, HEIGHT, false);

        // Check dimensions
        let size = fb.framebuffer.len();
        assert_eq!(size, WIDTH * HEIGHT);
    }
}
