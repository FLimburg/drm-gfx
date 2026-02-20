use drm_gfx::mesh::K3dMesh;
use drm_gfx::{K3dengine, draw::draw, mesh::Geometry, perfcounter::PerformanceCounter};
use embedded_graphics::Drawable;
use embedded_graphics::{
    geometry::Point,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    text::Text,
};
use embedded_graphics_core::pixelcolor::{Bgr888, WebColors};
use log::{debug, info};
use nalgebra::Point3;
use std::f32::consts::PI;

fn main() {
    env_logger::init();
    info!("drm-gfx example: lines");
    info!("Drawing two crossing green/red lines on screen");
    let points = vec![
        [-1.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [0.0, 1.0, -1.0],
        [0.0, -1.0, -1.0],
    ];
    let colors = vec![Bgr888::new(0, 0, 255), Bgr888::new(0, 255, 0)];
    let lines = vec![[0, 1], [2, 3]];

    let mut mesh = K3dMesh::new(Geometry {
        vertices: &points,
        faces: &[],
        colors: &colors,
        lines: &lines,
        normals: &[],
    });
    mesh.set_color(Bgr888::CSS_GREEN);
    mesh.set_render_mode(drm_gfx::mesh::RenderMode::Lines);

    let text_style = MonoTextStyle::new(&FONT_6X10, Bgr888::CSS_WHITE);

    let mut engine = K3dengine::new();
    engine.camera.set_position(Point3::new(0.0, 0.0, 4.0));
    engine.camera.set_target(Point3::new(0.0, 0.0, 0.0));
    engine.camera.set_fovy(PI / 4.0);

    let mut perf = PerformanceCounter::new();
    // perf.only_fps(true);

    debug!("Starting render loop ... ");
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
