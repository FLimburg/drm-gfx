#[cfg(not(test))]
use crate::drm_render_target::FramebufferTarget;
use crate::drm_render_target::RenderTarget;
use crate::framebuffer::DmaReadyFramebuffer;
use log::{debug, error, info, trace};
use std::sync::{Arc, Mutex};

pub struct DoubleBuffer {
    #[cfg(not(feature = "tokio-threads"))]
    sender: Option<std::sync::mpsc::Sender<usize>>,
    #[cfg(feature = "tokio-threads")]
    sender: Option<tokio::sync::mpsc::Sender<usize>>,
    toggle: bool,
    size: usize,
    fbuf0: DmaReadyFramebuffer,
    fbuf1: DmaReadyFramebuffer,
    mutex: Arc<Mutex<bool>>,
}

impl DoubleBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        trace!("Creating new DoubleBuffer with raw framebuffers");

        let fbuf0 = DmaReadyFramebuffer::new(width, height, true);
        let fbuf1 = DmaReadyFramebuffer::new(width, height, true);

        Self {
            sender: None,
            toggle: false,
            size: width * height,
            fbuf0,
            fbuf1,
            mutex: Arc::new(Mutex::new(true)),
        }
    }

    pub fn start_thread(&mut self, display: Option<RenderTarget>) {
        debug!("Creating RenderTarget from card");

        #[cfg(not(test))]
        let mut display = display.unwrap();

        info!("Starting fb writer thread");
        #[cfg(not(feature = "tokio-threads"))]
        let (send, receive) = std::sync::mpsc::channel();
        #[cfg(feature = "tokio-threads")]
        let (send, mut receive) = tokio::sync::mpsc::channel(16);

        self.sender = Some(send);
        let mutex2 = self.mutex.clone();
        let size = self.size;

        #[cfg(not(feature = "tokio-threads"))]
        std::thread::spawn(move || {
            trace!("Framebuffer writer thread started for std runtime");
            loop {
                let ptr = receive.recv().unwrap();
                trace!("Received framebuffer pointer: {ptr}");
                unsafe {
                    let _lock = mutex2.lock().unwrap();

                    let ptr = ptr as *mut u32;
                    let slice = std::slice::from_raw_parts_mut(ptr, size);

                    #[cfg(not(test))]
                    display.eat_framebuffer(slice).unwrap();
                    slice.fill(0); // 2.2ms
                };
            }
        });

        #[cfg(feature = "tokio-threads")]
        tokio::spawn(async move {
            trace!("Framebuffer writer thread started for tokio runtime");
            loop {
                let ptr = receive.recv().await.unwrap();
                trace!("Received framebuffer pointer: {}", ptr);
                unsafe {
                    let _lock = mutex2.lock().unwrap();

                    let ptr = ptr as *mut u32;
                    let slice = std::slice::from_raw_parts_mut(ptr, size);

                    #[cfg(not(test))]
                    display.eat_framebuffer(slice).unwrap();
                    slice.fill(0); // 2.2ms
                };
            }
        });
    }

    pub fn swap_framebuffer(&mut self) -> &mut DmaReadyFramebuffer {
        trace!("Swapping framebuffer from {}", self.toggle);
        self.toggle = !self.toggle;

        if self.toggle {
            &mut self.fbuf0
        } else {
            &mut self.fbuf1
        }
    }

    pub fn get_current_framebuffer(&mut self) -> &mut DmaReadyFramebuffer {
        if self.toggle {
            &mut self.fbuf0
        } else {
            &mut self.fbuf1
        }
    }

    #[cfg(not(feature = "tokio-threads"))]
    pub fn send_framebuffer(&mut self) {
        {
            let _lock = self.mutex.lock().unwrap();
            std::mem::drop(_lock);
        }

        let fbuf = if self.toggle {
            trace!(
                "sending framebuffer 0 ({:?})",
                self.fbuf0.framebuffer.as_ptr()
            );
            &mut self.fbuf0
        } else {
            trace!(
                "sending framebuffer 1 ({:?})",
                self.fbuf1.framebuffer.as_ptr()
            );
            &mut self.fbuf1
        };

        if let Some(sender) = &self.sender {
            sender
                .send(fbuf.framebuffer.as_ptr() as usize)
                .inspect_err(|msg| {
                    error!("Failed to send framebuffer: {msg}");
                })
                .unwrap();
        }
    }

    #[cfg(feature = "tokio-threads")]
    pub async fn send_framebuffer(&mut self) {
        trace!("Sending framebuffer in async context");
        {
            let _lock = self.mutex.lock().unwrap();
            std::mem::drop(_lock);
        }

        let fbuf = if self.toggle {
            trace!(
                "sending framebuffer 0 ({:?})",
                self.fbuf0.framebuffer.as_ptr()
            );
            &mut self.fbuf0
        } else {
            trace!(
                "sending framebuffer 1 ({:?})",
                self.fbuf1.framebuffer.as_ptr()
            );
            &mut self.fbuf1
        };

        if let Some(sender) = &self.sender {
            sender
                .send(fbuf.framebuffer.as_ptr() as usize)
                .await
                .inspect_err(|msg| {
                    error!("Failed to send framebuffer: {}", msg);
                })
                .unwrap();
        }
    }
}

// Add at the end of doublebuffer.rs, after the struct definition
// SAFETY: The raw pointers in DmaReadyFramebuffer point to memory owned by
// RenderTarget which outlives the DoubleBuffer. Access to the framebuffers
// is synchronized via the mutex field, ensuring only one thread accesses
// a framebuffer at a time.
unsafe impl Send for DoubleBuffer {}

#[cfg(test)]
mod tests {
    use embedded_graphics::prelude::RgbColor;

    use super::*;
    // Note: We considered creating a MockRenderTarget for testing, but since we're not
    // testing the start_thread and send_framebuffer functionality directly (due to
    // the complexity of testing threads and channels), we've removed it to avoid unused code.

    #[test]
    fn test_doublebuffer_creation() {
        const WIDTH: usize = 64;
        const HEIGHT: usize = 64;

        // Create doublebuffer
        let db = DoubleBuffer::new(WIDTH, HEIGHT);

        // Test initial state
        assert!(!db.toggle); // Should start with toggle = false
        assert!(db.sender.is_none()); // No sender initially
    }

    #[test]
    fn test_framebuffer_swapping() {
        const WIDTH: usize = 64;
        const HEIGHT: usize = 64;

        // Create doublebuffer
        let mut db = DoubleBuffer::new(WIDTH, HEIGHT);

        // Initial toggle is false, so swap should make it true
        let fb1 = db.swap_framebuffer();
        // Note: We don't check toggle state directly as it's an internal implementation detail

        // Write to the first buffer using embedded-graphics compatible method
        use embedded_graphics_core::geometry::Point;
        use embedded_graphics_core::pixelcolor::Bgr888;

        fb1.set_pixel(Point::new(0, 0), Bgr888::new(255, 255, 255));

        // Swap again, should get the other buffer
        let fb2 = db.swap_framebuffer();

        // Check buffer contents using as_slice
        assert_eq!(fb2.get_pixel(Point { x: 0, y: 0 }), Some(Bgr888::BLACK)); // Second buffer should be empty

        // Swap again, should get back to the first buffer with our pixel set
        let fb3 = db.swap_framebuffer();

        // First pixel in first buffer should be white
        assert_eq!(fb3.get_pixel(Point { x: 0, y: 0 }), Some(Bgr888::WHITE)); // Should contain our white pixel
    }

    // We can't easily test start_thread and send_framebuffer without mocking the RenderTarget trait
    // which would require significant mocking infrastructure. These functions involve threads
    // and channel communication which are hard to test in unit tests.
    //
    // For now, we'll skip these tests and focus on the core functionality that can be
    // tested reliably.

    #[test]
    fn test_toggle_behavior() {
        const WIDTH: usize = 64;
        const HEIGHT: usize = 64;

        // Create doublebuffer
        let mut db = DoubleBuffer::new(WIDTH, HEIGHT);

        // Track initial value - should be false
        let initial_toggle = db.toggle;
        assert!(!initial_toggle);

        // First swap
        let _first_fb = db.swap_framebuffer();

        // Second swap
        let _second_fb = db.swap_framebuffer();

        // Third swap
        let _third_fb = db.swap_framebuffer();

        // After three swaps, toggle should be true again
        assert!(db.toggle);
    }
}
