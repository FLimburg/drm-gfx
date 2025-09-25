use embedded_graphics_core::draw_target::DrawTarget;
use embedded_graphics_core::prelude::Point;

use crate::DrawPrimitive;

/// Fills a buffer with a solid color
#[inline]
pub fn fill_buffer<D: DrawTarget<Color = embedded_graphics_core::pixelcolor::Bgr888>>(
    fb: &mut D,
    color: embedded_graphics_core::pixelcolor::Bgr888,
) where
    <D as DrawTarget>::Error: std::fmt::Debug,
{
    fb.clear(color).unwrap();
}

#[inline]
pub fn draw<D: DrawTarget<Color = embedded_graphics_core::pixelcolor::Bgr888>>(
    primitive: DrawPrimitive,
    fb: &mut D,
) where
    <D as DrawTarget>::Error: std::fmt::Debug,
{
    match primitive {
        DrawPrimitive::Line([p1, p2], color) => {
            fb.draw_iter(
                line_drawing::Bresenham::new((p1.x, p1.y), (p2.x, p2.y))
                    .map(|(x, y)| embedded_graphics_core::Pixel(Point::new(x, y), color)),
            )
            .unwrap();
        }
        DrawPrimitive::ColoredPoint(p, c) => {
            let p = embedded_graphics_core::geometry::Point::new(p.x, p.y);

            fb.draw_iter([embedded_graphics_core::Pixel(p, c)]).unwrap();
        }
        DrawPrimitive::ColoredTriangle(mut vertices, color) => {
            //sort vertices by y
            vertices.sort_by(|a, b| a.y.cmp(&b.y));

            let [p1, p2, p3] = vertices
                .iter()
                .map(|p| embedded_graphics_core::geometry::Point::new(p.x, p.y))
                .collect::<Vec<embedded_graphics_core::geometry::Point>>()
                .try_into()
                .unwrap();

            if p2.y == p3.y {
                fill_bottom_flat_triangle(p1, p2, p3, color, fb);
            } else if p1.y == p2.y {
                fill_top_flat_triangle(p1, p2, p3, color, fb);
            } else {
                let p4 = Point::new(
                    (p1.x as f32
                        + ((p2.y - p1.y) as f32 / (p3.y - p1.y) as f32) * (p3.x - p1.x) as f32)
                        as i32,
                    p2.y,
                );

                fill_bottom_flat_triangle(p1, p2, p4, color, fb);
                fill_top_flat_triangle(p2, p4, p3, color, fb);
            }
        }
    }
}

fn fill_bottom_flat_triangle<D: DrawTarget<Color = embedded_graphics_core::pixelcolor::Bgr888>>(
    p1: Point,
    p2: Point,
    p3: Point,
    color: embedded_graphics_core::pixelcolor::Bgr888,
    fb: &mut D,
) where
    <D as DrawTarget>::Error: std::fmt::Debug,
{
    // Avoid accumulated floating point errors by recalculating for each scanline
    for scanline_y in p1.y..=p2.y {
        let t = (scanline_y - p1.y) as f32 / (p2.y - p1.y) as f32;

        // Calculate exact x-coordinates for this scanline
        let x1 = p1.x as f32 + t * (p2.x - p1.x) as f32;
        let x2 = p1.x as f32 + t * (p3.x - p1.x) as f32;

        draw_horizontal_line(
            Point::new(x1.round() as i32, scanline_y),
            Point::new(x2.round() as i32, scanline_y),
            color,
            fb,
        );
    }
}

fn fill_top_flat_triangle<D: DrawTarget<Color = embedded_graphics_core::pixelcolor::Bgr888>>(
    p1: Point,
    p2: Point,
    p3: Point,
    color: embedded_graphics_core::pixelcolor::Bgr888,
    fb: &mut D,
) where
    <D as DrawTarget>::Error: std::fmt::Debug,
{
    // Avoid accumulated floating point errors by recalculating for each scanline
    for scanline_y in p1.y..=p3.y {
        let t = (scanline_y - p1.y) as f32 / (p3.y - p1.y) as f32;

        // Calculate exact x-coordinates for this scanline
        let x1 = p1.x as f32 + t * (p3.x - p1.x) as f32;
        let x2 = p2.x as f32 + t * (p3.x - p2.x) as f32;

        draw_horizontal_line(
            Point::new(x1.round() as i32, scanline_y),
            Point::new(x2.round() as i32, scanline_y),
            color,
            fb,
        );
    }
}

fn draw_horizontal_line<D: DrawTarget<Color = embedded_graphics_core::pixelcolor::Bgr888>>(
    p1: Point,
    p2: Point,
    color: embedded_graphics_core::pixelcolor::Bgr888,
    fb: &mut D,
) where
    <D as DrawTarget>::Error: std::fmt::Debug,
{
    let start = p1.x.min(p2.x);
    let end = p1.x.max(p2.x);

    for x in start..=end {
        fb.draw_iter([embedded_graphics_core::Pixel(Point::new(x, p1.y), color)])
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics_core::{
        draw_target::DrawTarget,
        geometry::{OriginDimensions, Size},
        pixelcolor::Bgr888,
    };
    use nalgebra::Point2;

    // A simple mock framebuffer for testing drawing functions
    struct MockFrameBuffer {
        pixels: Vec<Vec<Option<Bgr888>>>,
        width: usize,
        height: usize,
        draw_calls: Vec<String>, // Track the drawing operations for verification
    }

    impl MockFrameBuffer {
        fn new(width: usize, height: usize) -> Self {
            let pixels = vec![vec![None; width]; height];
            Self {
                pixels,
                width,
                height,
                draw_calls: Vec::new(),
            }
        }

        fn get_pixel(&self, x: i32, y: i32) -> Option<Bgr888> {
            if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                return None;
            }
            self.pixels[y as usize][x as usize]
        }

        fn pixel_count(&self) -> usize {
            self.pixels.iter().flatten().filter(|p| p.is_some()).count()
        }

        fn contains_line(&self, p1: Point2<i32>, p2: Point2<i32>) -> bool {
            // Check if all pixels on the line are set
            for (x, y) in line_drawing::Bresenham::new((p1.x, p1.y), (p2.x, p2.y)) {
                if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                    continue;
                }
                if self.pixels[y as usize][x as usize].is_none() {
                    return false;
                }
            }
            true
        }
    }

    impl DrawTarget for MockFrameBuffer {
        type Color = Bgr888;
        type Error = std::convert::Infallible;

        fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
        where
            I: IntoIterator<Item = embedded_graphics_core::Pixel<Self::Color>>,
        {
            for embedded_graphics_core::Pixel(point, color) in pixels {
                if point.x >= 0
                    && point.y >= 0
                    && point.x < self.width as i32
                    && point.y < self.height as i32
                {
                    self.pixels[point.y as usize][point.x as usize] = Some(color);
                    self.draw_calls.push(format!("Pixel at ({}, {}) = {:?}", point.x, point.y, color));
                }
            }
            Ok(())
        }

        fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
            for y in 0..self.height {
                for x in 0..self.width {
                    self.pixels[y][x] = Some(color);
                }
            }
            self.draw_calls.push(format!("Clear with color {:?}", color));
            Ok(())
        }
    }

    impl OriginDimensions for MockFrameBuffer {
        fn size(&self) -> Size {
            Size::new(self.width as u32, self.height as u32)
        }
    }

    #[test]
    fn test_draw_colored_point() {
        let mut fb = MockFrameBuffer::new(100, 100);
        let point = Point2::new(50, 50);
        let color = Bgr888::new(255, 127, 63);

        draw(DrawPrimitive::ColoredPoint(point, color), &mut fb);

        assert_eq!(fb.get_pixel(50, 50), Some(color));
        assert_eq!(fb.pixel_count(), 1);
    }

    #[test]
    fn test_draw_line() {
        let mut fb = MockFrameBuffer::new(100, 100);
        let p1 = Point2::new(10, 10);
        let p2 = Point2::new(20, 20);
        let color = Bgr888::new(255, 127, 63);

        draw(DrawPrimitive::Line([p1, p2], color), &mut fb);

        // Check that the line is actually drawn
        assert!(fb.contains_line(p1, p2));

        // A diagonal line from (10,10) to (20,20) should have exactly 11 pixels
        let pixel_count = fb.pixel_count();
        assert_eq!(pixel_count, 11);
    }

    #[test]
    fn test_draw_horizontal_line() {
        let mut fb = MockFrameBuffer::new(100, 100);
        let p1 = embedded_graphics_core::geometry::Point::new(10, 50);
        let p2 = embedded_graphics_core::geometry::Point::new(20, 50);
        let color = Bgr888::new(255, 127, 63);

        draw_horizontal_line(p1, p2, color, &mut fb);

        // Check all pixels in the horizontal line
        for x in 10..=20 {
            assert_eq!(fb.get_pixel(x, 50), Some(color));
        }

        // A horizontal line from (10,50) to (20,50) should have exactly 11 pixels
        assert_eq!(fb.pixel_count(), 11);
    }

    #[test]
    fn test_fill_bottom_flat_triangle() {
        let mut fb = MockFrameBuffer::new(100, 100);
        let p1 = embedded_graphics_core::geometry::Point::new(50, 10);  // Top point
        let p2 = embedded_graphics_core::geometry::Point::new(30, 50);  // Bottom-left point
        let p3 = embedded_graphics_core::geometry::Point::new(70, 50);  // Bottom-right point
        let color = Bgr888::new(255, 127, 63);

        fill_bottom_flat_triangle(p1, p2, p3, color, &mut fb);

        // Check some key points
        assert_eq!(fb.get_pixel(50, 10), Some(color)); // Top vertex
        assert_eq!(fb.get_pixel(30, 50), Some(color)); // Bottom-left vertex
        assert_eq!(fb.get_pixel(70, 50), Some(color)); // Bottom-right vertex
        assert_eq!(fb.get_pixel(50, 30), Some(color)); // Middle point

        // The triangle should have a non-zero number of pixels
        assert!(fb.pixel_count() > 0);
    }

    #[test]
    fn test_fill_top_flat_triangle() {
        let mut fb = MockFrameBuffer::new(100, 100);
        let p1 = embedded_graphics_core::geometry::Point::new(30, 10);  // Top-left point
        let p2 = embedded_graphics_core::geometry::Point::new(70, 10);  // Top-right point
        let p3 = embedded_graphics_core::geometry::Point::new(50, 50);  // Bottom point
        let color = Bgr888::new(255, 127, 63);

        fill_top_flat_triangle(p1, p2, p3, color, &mut fb);

        // Check some key points
        assert_eq!(fb.get_pixel(30, 10), Some(color)); // Top-left vertex
        assert_eq!(fb.get_pixel(70, 10), Some(color)); // Top-right vertex
        assert_eq!(fb.get_pixel(50, 50), Some(color)); // Bottom vertex
        assert_eq!(fb.get_pixel(50, 30), Some(color)); // Middle point

        // The triangle should have a non-zero number of pixels
        assert!(fb.pixel_count() > 0);
    }

    #[test]
    fn test_draw_colored_triangle() {
        let mut fb = MockFrameBuffer::new(100, 100);
        let vertices = [
            Point2::new(50, 10),   // Top point
            Point2::new(30, 70),   // Bottom-left point
            Point2::new(70, 70),   // Bottom-right point
        ];
        let color = Bgr888::new(255, 127, 63);

        draw(DrawPrimitive::ColoredTriangle(vertices, color), &mut fb);

        // Check the vertices
        assert_eq!(fb.get_pixel(50, 10), Some(color));
        assert_eq!(fb.get_pixel(30, 70), Some(color));
        assert_eq!(fb.get_pixel(70, 70), Some(color));

        // Check the center of the triangle
        assert_eq!(fb.get_pixel(50, 50), Some(color));

        // The triangle should have a non-zero number of pixels
        assert!(fb.pixel_count() > 0);
    }

    #[test]
    fn test_draw_colored_triangle_flat_top() {
        let mut fb = MockFrameBuffer::new(100, 100);
        let vertices = [
            Point2::new(30, 10),   // Top-left point
            Point2::new(70, 10),   // Top-right point
            Point2::new(50, 70),   // Bottom point
        ];
        let color = Bgr888::new(255, 127, 63);

        draw(DrawPrimitive::ColoredTriangle(vertices, color), &mut fb);

        // Check the vertices
        assert_eq!(fb.get_pixel(30, 10), Some(color));
        assert_eq!(fb.get_pixel(70, 10), Some(color));
        assert_eq!(fb.get_pixel(50, 70), Some(color));

        // The triangle should have a non-zero number of pixels
        assert!(fb.pixel_count() > 0);
    }

    #[test]
    fn test_draw_colored_triangle_flat_bottom() {
        let mut fb = MockFrameBuffer::new(100, 100);
        let vertices = [
            Point2::new(50, 10),   // Top point
            Point2::new(30, 70),   // Bottom-left point
            Point2::new(70, 70),   // Bottom-right point
        ];
        let color = Bgr888::new(255, 127, 63);

        draw(DrawPrimitive::ColoredTriangle(vertices, color), &mut fb);

        // Check the vertices
        assert_eq!(fb.get_pixel(50, 10), Some(color));
        assert_eq!(fb.get_pixel(30, 70), Some(color));
        assert_eq!(fb.get_pixel(70, 70), Some(color));

        // The triangle should have a non-zero number of pixels
        assert!(fb.pixel_count() > 0);
    }

    #[test]
    fn test_draw_colored_triangle_general_case() {
        let mut fb = MockFrameBuffer::new(100, 100);
        let vertices = [
            Point2::new(20, 20),   // Top point
            Point2::new(40, 60),   // Middle point
            Point2::new(80, 40),   // Bottom point
        ];
        let color = Bgr888::new(255, 127, 63);

        draw(DrawPrimitive::ColoredTriangle(vertices, color), &mut fb);

        // Check the vertices
        assert_eq!(fb.get_pixel(20, 20), Some(color));
        assert_eq!(fb.get_pixel(40, 60), Some(color));
        assert_eq!(fb.get_pixel(80, 40), Some(color));

        // The triangle should have a non-zero number of pixels
        assert!(fb.pixel_count() > 0);
    }

    #[test]
    fn test_fill_buffer() {
        let mut fb = MockFrameBuffer::new(100, 100);
        let color = Bgr888::new(255, 127, 63);

        fill_buffer(&mut fb, color);

        // Check that all pixels are filled with the color
        for y in 0..100 {
            for x in 0..100 {
                assert_eq!(fb.get_pixel(x, y), Some(color));
            }
        }

        // The buffer should have all pixels filled
        assert_eq!(fb.pixel_count(), 100 * 100);
    }
}
