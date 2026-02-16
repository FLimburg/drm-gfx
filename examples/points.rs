use drm_gfx::mesh::K3dMesh;
use drm_gfx::{K3dengine, draw::draw, mesh::Geometry, perfcounter::PerformanceCounter};
use embedded_graphics::Drawable;
use embedded_graphics::{
    geometry::Point,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    text::Text,
};
use embedded_graphics_core::pixelcolor::{Bgr888, WebColors};
use nalgebra::Point3;
use std::f32::consts::PI;

// #[tokio::main]
fn main() {
    println!("Hello, world!");
    let points = vec![
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.5, 0.0],
        [1.0, 0.5, 0.0],
        [0.0, 0.5, 0.0],
    ];

    let mut mesh = K3dMesh::new(Geometry {
        vertices: &points,
        faces: &[],
        colors: &[],
        lines: &[],
        normals: &[],
    });
    mesh.set_color(Bgr888::CSS_GREEN);

    let text_style = MonoTextStyle::new(&FONT_6X10, Bgr888::CSS_WHITE);

    let mut engine = K3dengine::new();
    engine.camera.set_position(Point3::new(0.0, 0.0, -4.0));
    engine.camera.set_target(Point3::new(0.0, 0.0, 0.0));
    engine.camera.set_fovy(PI / 4.0);

    let mut perf = PerformanceCounter::new();
    // perf.only_fps(true);

    println!("Starting render loop ... ");
    loop {
        perf.start_of_frame();

        let mut primitives = Vec::new();
        engine.render([&mesh], |p| {
            primitives.push(p);
        });

        perf.add_measurement("render");

        let fbuf = engine.swap_framebuffer();
        for primitive in primitives {
            draw(primitive, fbuf);
        }

        Text::new(perf.get_text(), Point::new(20, 20), text_style)
            .draw(fbuf)
            .unwrap();

        engine.send_framebuffer();
        perf.add_measurement("draw");

        perf.print();
    }
}
