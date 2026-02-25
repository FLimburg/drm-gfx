// #![allow(dead_code)]
use drm::control::{Device as ControlDevice, dumbbuffer::DumbBuffer, framebuffer};
// use drm::{buffer, Device};
use crate::card::Card;
use drm::buffer::{Buffer, DrmFourcc};
use drm::control::{Mode, connector, crtc};
use log::{debug, error, trace};

pub struct RenderTarget {
    pub card: Card,
    pub crtc: crtc::Handle,
    pub connection: connector::Handle,
    pub fb: framebuffer::Handle,
    pub db: DumbBuffer,
    pub width: usize,
    pub height: usize,
    pub format: DrmFourcc,
    pub mode: Mode,
}

impl Default for RenderTarget {
    fn default() -> Self {
        let device_paths = [
            "/dev/dri/card0",
            "/dev/dri/card1",
            "/dev/dri/card2",
            "/dev/dri/renderD128",
            "/dev/dri/renderD129",
        ];
        for dev in device_paths {
            match Self::new(dev) {
                Ok(render_target) => {
                    trace!("Created render target for device {dev}");
                    return render_target;
                }
                Err(e) => {
                    trace!("Could not created render target for device {dev}: {e}");
                    continue;
                }
            }
        }

        error!("Could not create any render target!");
        panic!("Could not create any render target!");
    }
}

impl RenderTarget {
    pub fn new(device: &str) -> Result<Self, std::io::Error> {
        let card =
            Card::open(device).inspect_err(|e| error!("failed to open device {device}: {e}"))?;

        // Load the information.
        let res = card
            .resource_handles()
            .inspect_err(|e| error!("failed to load resource handle ids from {device}: {e}"))?;

        let coninfo: Vec<connector::Info> = res
            .connectors()
            .iter()
            .flat_map(|con| card.get_connector(*con, true))
            .collect();
        let crtcinfo: Vec<crtc::Info> = res
            .crtcs()
            .iter()
            .flat_map(|crtc| card.get_crtc(*crtc))
            .collect();

        // Filter each connector until we find one that's connected.
        let con = coninfo
            .iter()
            .find(|&i| i.state() == connector::State::Connected)
            .expect("No connected connectors");

        // Get the first (usually best) mode
        let &mode = con.modes().first().expect("No modes found on connector");

        let (width, height) = mode.size();

        // Find a crtc and FB
        let crtc = crtcinfo.first().expect("No crtcs found");

        // Select the pixel format
        let format = DrmFourcc::Xrgb8888;

        // Create a DB
        // If buffer resolution is larger than display resolution, an ENOSPC (not enough video memory)
        // error may occur
        let mut db = card
            .create_dumb_buffer((width.into(), height.into()), format, 32)
            .expect("Could not create dumb buffer");

        // Map it and grey it out.
        {
            let mut map = card
                .map_dumb_buffer(&mut db)
                .expect("Could not map dumbbuffer");
            // for b in map.as_mut() {
            //     *b = 128;
            // }
            let buf = map.as_mut();
            let line_length = width as u64 * 4;
            for j in 0..height {
                if j % 2 == 0 {
                    continue;
                }
                let line_offset = j as u64 * line_length;
                for i in 0..width {
                    if i % 2 == 0 {
                        continue;
                    }
                    let offset = (line_offset + i as u64 * 4) as usize;
                    if offset + 4 > buf.len() {
                        panic!("Buffer overflow at offset {offset}");
                    }
                    buf[offset] = 128; // B
                    buf[offset + 1] = 0; // G
                    buf[offset + 2] = 128; // R
                    buf[offset + 3] = 255; // A
                }
            }
        }

        // Create an FB:
        let fb = card
            .add_framebuffer(&db, 24, 32)
            .expect("Could not create FB");

        debug!("mode: {mode:#?}");
        trace!("frame buffer handle:{fb:#?}");
        trace!("dumb buffer{db:#?}");

        // Set the crtc
        // On many setups, this requires root access.
        card.set_crtc(crtc.handle(), Some(fb), (0, 0), &[con.handle()], Some(mode))
            .expect("Could not set CRTC");

        Ok(Self {
            card,
            crtc: crtc.handle(),
            connection: con.handle(),
            fb,
            db,
            width: width as usize,
            height: height as usize,
            format,
            mode,
        })
    }

    pub fn destroy(&self) {
        trace!("Destroy the framebuffer");
        self.card.destroy_framebuffer(self.fb).unwrap();
        self.card.destroy_dumb_buffer(self.db).unwrap();
    }

    pub fn get_info(&self) -> String {
        format!(
            "RenderTarget details:  mode: {:#?} -- buffer: {:#?}",
            self.mode, self.db,
        )
    }
}

#[derive(Debug)]
pub enum FbWriteError {
    Error,
}
pub trait FramebufferTarget {
    fn eat_framebuffer(&mut self, buf: &[u32]) -> Result<(), FbWriteError>;
}
impl FramebufferTarget for RenderTarget {
    fn eat_framebuffer(&mut self, buffer: &[u32]) -> Result<(), FbWriteError> {
        let pixel_count = self.db.size().0 as usize * self.db.size().1 as usize;
        if buffer.len() != pixel_count {
            panic!(
                "Buffer length mismatch: expected {}, got {}",
                pixel_count,
                buffer.len()
            );
        }
        // Map it and cycle colors.
        {
            let mut map = self
                .card
                .map_dumb_buffer(&mut self.db)
                .expect("Could not map dumbbuffer");
            let buf = map.as_mut();
            let buf_line_length = self.width as u64 * 4;
            let buffer_line_length = self.width as u64;
            for j in 0..self.height {
                let buf_line_offset = j as u64 * buf_line_length;
                let buffer_line_offset = j as u64 * buffer_line_length;
                for i in 0..self.width {
                    let buf_offset = (buf_line_offset + i as u64 * 4) as usize;
                    let buffer_offset = (buffer_line_offset + i as u64) as usize;
                    // if buf_offset + 4 > buf.len() {
                    //     panic!("Buf overflow at offset {}", buf_offset);
                    // }
                    // if buffer_offset > buffer.len() {
                    //     panic!("Buffer overflow at offset {}", buffer_offset);
                    // }

                    buf[buf_offset] = ((buffer[buffer_offset] >> 24) & 0xff) as u8; // B
                    buf[buf_offset + 1] = ((buffer[buffer_offset] >> 16) & 0xff) as u8; // G
                    buf[buf_offset + 2] = ((buffer[buffer_offset] >> 8) & 0xff) as u8; // R
                    buf[buf_offset + 3] = 255; // A
                }
            }
        }
        // Create an FB:
        let fb = self
            .card
            .add_framebuffer(&self.db, 24, 32)
            .expect("Could not create FB");
        self.card
            .set_crtc(
                self.crtc,
                Some(fb),
                (0, 0),
                &[self.connection],
                Some(self.mode),
            )
            .expect("Could not set CRTC");

        Ok(())
    }
}
