# drm-gfx
<a href="https://crates.io/crates/drm-gfx"><img alt="crates.io" src="https://img.shields.io/crates/v/drm-gfx"></a>
<a href="https://github.com/FLimburg/drm-gfx/actions"><img alt="actions" src="https://github.com/FLimburg/drm-gfx/actions/workflows/rust.yml/badge.svg"></a>

A 3D graphics rendering library for the Linux Direct Rendering Manager (DRM). This library provides a lightweight 3D rendering engine that works directly with the DRM interface, without requiring a full graphics stack or window manager.

Or to put it more bluntly:
I stitched together https://github.com/Kezii/embedded-gfx/tree/master with https://github.com/Smithay/drm-rs/tree/develop without any care and threw it into https://github.com/FLimburg/drm-gfx .

## Features

- 3D mesh rendering with multiple render modes (Points, Lines, Solid, Directional Lighting)
- Camera with configurable position, target, and field of view
- Support for transformations and model matrices
- Backface culling
- Simple lighting model
- Direct rendering to DRM framebuffers
- Double buffering support
- Performance counters for optimization

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
drm-gfx = "0.1.3"
```

## Limitations
Currently only displays with a resolution fo 1024 x 600 are supported.
If you need something else you will need to adapt the drm-gfx code at
drm_renter_target.rs:13
drm_renter_target.rs:14

### features

use feature tokio-thread to use drm-gfx in a tokio based application

### Basic Example

Define WIDTH and HEIGHT of your screen as env variables during compilation, like:
`WIDTH=800 HEIGHT=600 cargo build -r --features=tokio-threads`
```rust
use drm_gfx::mesh::K3dMesh;
use drm_gfx::{
    draw::draw,
    mesh::Geometry,
    perfcounter::PerformanceCounter,
    K3dengine,
    doublebuffer::DoubleBuffer,
};
use embedded_graphics::Drawable;
use embedded_graphics::{
    geometry::Point,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    text::Text,
};
use embedded_graphics_core::pixelcolor::{Bgr888, WebColors};
use nalgebra::Point3;
use std::f32::consts::PI;
use std::ffi::c_void;

mod points;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let points = Vec![[-1,0,0],[1,0,0],[0,1,0]];
    
    let mut locations = K3dMesh::new(Geometry{
        vertices: &locs,
        faces: &[],
        colors: &[],
        lines: &[],
        normals: &[],
    });
    locations.set_color(Bgr888::CSS_GREEN);

    let text_style = MonoTextStyle::new(&FONT_6X10, Bgr888::CSS_WHITE);

    // this will try the following list of devices and use the 1st that seems to be working:
    // "/dev/dri/card0",
    // "/dev/dri/card1",
    // "/dev/dri/card2",
    // "/dev/dri/renderD128",
    // "/dev/dri/renderD129",
    // If all fail it will panic.
    // the list can be changed at drm_render_target.rs:23
    let mut engine = K3dengine::new(WIDTH as u16, HEIGHT as u16);
    engine.camera.set_position(Point3::new(0.0, 0.0, -4.0));
    engine.camera.set_target(Point3::new(0.0, 0.0, 0.0));
    engine.camera.set_fovy(PI / 4.0);

    let mut perf = PerformanceCounter::new();
    // perf.only_fps(true);

    println!("Starting render loop ... ");
    loop {
        let fbuf = buffers.swap_framebuffer();

        perf.start_of_frame();

        engine.render([&locations], |p| draw(p, fbuf));
        perf.add_measurement("render");

        Text::new(perf.get_text(), Point::new(20, 20), text_style)
            .draw(fbuf)
            .unwrap();

        buffers.send_framebuffer();
        perf.add_measurement("draw");

        perf.print();
    }

    println!("all done. Last perf: {}", perf.get_text());
}
```

## Supported Render Modes

- **Points**: Renders just the vertices as points
- **Lines**: Renders edges of the mesh as lines
- **Solid**: Renders filled triangles with backface culling
- **SolidLightDir**: Adds directional lighting to solid rendering

## Dependencies

- `embedded-graphics-core`: For pixel color representation
- `nalgebra`: For matrix math and transformations
- `line_drawing`: For line rasterization
- `drm`: For interfacing with the Direct Rendering Manager

## Performance

The library includes performance counters that can be used to measure rendering time and optimize your application.

```rust
use drm_gfx::perfcounter::PerfCounter;

let mut perf = PerfCounter::new();
perf.start();
// Perform rendering
perf.end();
println!("Rendering took {} ms", perf.elapsed_ms());
```

## Framebuffer Optimization

For improved performance, the library supports double buffering:

```rust
use drm_gfx::doublebuffer::DoubleBuffer;

let mut buffer = DoubleBuffer::new(width, height);

// In your render loop:
buffer.swap();
let framebuffer = buffer.get_front();
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be dual licensed as above, without any additional terms or conditions.

## unit test disclaimer

All unit tests are autogenerated by some ai.
