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
    info!("drm-gfx example: spinning-cube");
    info!("Drawing a coloured spinnign cube on screen");
    let points = vec![
        [1.0, -1.0, 1.0],   // 0
        [1.0, 1.0, 1.0],    // 1
        [-1.0, 1.0, 1.0],   // 2
        [-1.0, -1.0, 1.0],  // 3
        [1.0, -1.0, -1.0],  // 4
        [1.0, 1.0, -1.0],   // 5
        [-1.0, 1.0, -1.0],  // 6
        [-1.0, -1.0, -1.0], // 7
    ];
    let faces = vec![
        // front
        [0, 1, 3],
        [3, 1, 2],
        // right
        [1, 0, 4],
        [4, 5, 1],
        // // top
        [2, 1, 5],
        [5, 6, 2],
        // left
        [2, 7, 3],
        [2, 6, 7],
        // bottom
        [3, 4, 0],
        [4, 3, 7],
        // back
        [4, 7, 5],
        [7, 6, 5],
    ];
    let colors = vec![
        // front
        Bgr888::new(255, 0, 0),
        Bgr888::new(255, 0, 0),
        // right
        Bgr888::new(0, 255, 0),
        Bgr888::new(0, 255, 0),
        // top
        Bgr888::new(0, 0, 255),
        Bgr888::new(0, 0, 255),
        // left
        Bgr888::new(0, 255, 255),
        Bgr888::new(0, 255, 255),
        // bottom
        Bgr888::new(255, 0, 255),
        Bgr888::new(255, 0, 255),
        // back
        Bgr888::new(255, 255, 0),
        Bgr888::new(255, 255, 0),
    ];
    let normals = vec![
        // front
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        // right
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        // top
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        // left
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        // bottom
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        // back
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
    ];

    debug!("creating mesh");
    let mut mesh = K3dMesh::new(Geometry {
        vertices: &points,
        faces: &faces,
        colors: &colors,
        lines: &[],
        normals: &normals,
    });
    mesh.set_render_mode(drm_gfx::mesh::RenderMode::Solid); //LightDir(
    //     nalgebra::Vector3::new(0.0, 2.0, 0.0),
    // ));

    let text_style = MonoTextStyle::new(&FONT_6X10, Bgr888::CSS_WHITE);

    debug!("creating engine");
    let mut engine = K3dengine::new();
    engine.camera.set_position(Point3::new(0.0, 0.0, 7.0));
    engine.camera.set_target(Point3::new(0.0, 0.0, 0.0));
    engine.camera.set_fovy(PI / 4.0);

    let mut perf = PerformanceCounter::new();
    // perf.only_fps(true);

    let mut roll = 0.0;
    let mut pitch = 0.0;
    let mut yaw = 0.0;
    // let rad = |deg: f32| deg * std::f32::consts::PI / 180.0;

    debug!("Starting render loop ... ");
    loop {
        perf.start_of_frame();

        let mut primitives = Vec::new();
        // mesh.set_attitude(roll, pitch, yaw);
        // yaw around x axis
        // pitch around y axis
        // roll around z axis
        mesh.set_attitude(roll, pitch, yaw);
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

        roll += 0.01;
        pitch += 0.01;
        yaw += 0.01;

        perf.print();
    }
}
