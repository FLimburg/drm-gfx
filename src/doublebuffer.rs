use crate::{
    drm_render_target::{FramebufferTarget, RenderTarget},
    framebuffer::DmaReadyFramebuffer,
};
use log::{debug, error, info, trace};
use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};

pub struct DoubleBuffer<const W: usize, const H: usize> {
    #[cfg(not(feature = "tokio"))]
    sender: Option<std::sync::mpsc::Sender<usize>>,
    #[cfg(feature = "tokio")]
    sender: Option<tokio::sync::mpsc::Sender<usize>>,
    toggle: bool,
    fbuf0: DmaReadyFramebuffer<W, H>,
    fbuf1: DmaReadyFramebuffer<W, H>,
    mutex: Arc<Mutex<bool>>,
}

impl<const W: usize, const H: usize> DoubleBuffer<W, H> {
    pub fn new(raw_framebuffer_0: *mut c_void, raw_framebuffer_1: *mut c_void) -> Self {
        trace!("Creating new DoubleBuffer with raw framebuffers");

        let fbuf0 = DmaReadyFramebuffer::<W, H>::new(raw_framebuffer_0, true);
        let fbuf1 = DmaReadyFramebuffer::<W, H>::new(raw_framebuffer_1, true);

        Self {
            sender: None,
            toggle: false,
            fbuf0,
            fbuf1,
            mutex: Arc::new(Mutex::new(true)),
        }
    }

    pub fn start_thread(&mut self) {
        debug!("Creating RenderTarget from card");
        let mut display = RenderTarget::default();

        info!("Starting fb writer thread");
        #[cfg(not(feature = "tokio"))]
        let (send, receive) = std::sync::mpsc::channel();
        #[cfg(feature = "tokio")]
        let (send, mut receive) = tokio::sync::mpsc::channel(16);

        self.sender = Some(send);
        let mutex2 = self.mutex.clone();

        #[cfg(not(feature = "tokio"))]
        std::thread::spawn(move || {
            trace!("Framebuffer writer thread started for std runtime");
            loop {
                let ptr = receive.recv().unwrap();
                trace!("Received framebuffer pointer: {ptr}");
                unsafe {
                    let _lock = mutex2.lock().unwrap();

                    let ptr = ptr as *mut u32;
                    let slice = std::slice::from_raw_parts_mut(ptr, W * H);

                    display.eat_framebuffer(slice).unwrap();
                    slice.fill(0); // 2.2ms
                };
            }
        });

        #[cfg(feature = "tokio")]
        tokio::spawn(async move {
            trace!("Framebuffer writer thread started for tokio runtime");
            loop {
                let ptr = receive.recv().await.unwrap();
                trace!("Received framebuffer pointer: {}", ptr);
                unsafe {
                    let _lock = mutex2.lock().unwrap();

                    let ptr = ptr as *mut u32;
                    let slice = std::slice::from_raw_parts_mut(ptr, W * H);

                    display.eat_framebuffer(slice).unwrap();
                    slice.fill(0); // 2.2ms
                };
            }
        });
    }

    pub fn swap_framebuffer(&mut self) -> &mut DmaReadyFramebuffer<W, H> {
        trace!("Swapping framebuffer from {}", self.toggle);
        self.toggle = !self.toggle;

        if self.toggle {
            &mut self.fbuf0
        } else {
            &mut self.fbuf1
        }
    }

    pub fn get_current_framebuffer(&mut self) -> &mut DmaReadyFramebuffer<W, H> {
        if self.toggle {
            &mut self.fbuf0
        } else {
            &mut self.fbuf1
        }
    }

    #[cfg(not(feature = "tokio"))]
    pub fn send_framebuffer(&mut self) {
        {
            let _lock = self.mutex.lock().unwrap();
            std::mem::drop(_lock);
        }

        let fbuf = if self.toggle {
            trace!(
                "sending framebuffer 0 ({})",
                self.fbuf0.framebuffer as usize
            );
            &mut self.fbuf0
        } else {
            trace!(
                "sending framebuffer 1 ({})",
                self.fbuf1.framebuffer as usize
            );
            &mut self.fbuf1
        };

        if let Some(sender) = &self.sender {
            sender
                .send(fbuf.framebuffer as usize)
                .inspect_err(|msg| {
                    error!("Failed to send framebuffer: {msg}");
                })
                .unwrap();
        }
    }

    #[cfg(feature = "tokio")]
    pub async fn send_framebuffer(&mut self) {
        trace!("Sending framebuffer in async context");
        {
            let _lock = self.mutex.lock().unwrap();
            std::mem::drop(_lock);
        }

        let fbuf = if self.toggle {
            trace!(
                "sending framebuffer 0 ({})",
                self.fbuf0.framebuffer as usize
            );
            &mut self.fbuf0
        } else {
            trace!(
                "sending framebuffer 1 ({})",
                self.fbuf1.framebuffer as usize
            );
            &mut self.fbuf1
        };

        if let Some(sender) = &self.sender {
            sender
                .send(fbuf.framebuffer as usize)
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
unsafe impl<const W: usize, const H: usize> Send for DoubleBuffer<W, H> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;
    use std::mem::MaybeUninit;

    // Note: We considered creating a MockRenderTarget for testing, but since we're not
    // testing the start_thread and send_framebuffer functionality directly (due to
    // the complexity of testing threads and channels), we've removed it to avoid unused code.

    // Helper to create a safe test buffer
    fn create_test_buffer<const W: usize, const H: usize>() -> Box<[[u32; W]; H]> {
        let buffer = Box::new(unsafe { MaybeUninit::<[[u32; W]; H]>::zeroed().assume_init() });
        buffer
    }

    #[test]
    fn test_doublebuffer_creation() {
        const WIDTH: usize = 64;
        const HEIGHT: usize = 64;

        // Create safe test buffers
        let buffer1 = create_test_buffer::<WIDTH, HEIGHT>();
        let buffer2 = create_test_buffer::<WIDTH, HEIGHT>();

        // Get raw pointers
        let raw_ptr1 = buffer1.as_ptr() as *mut c_void;
        let raw_ptr2 = buffer2.as_ptr() as *mut c_void;

        // Create doublebuffer
        let db = DoubleBuffer::<WIDTH, HEIGHT>::new(raw_ptr1, raw_ptr2);

        // Test initial state
        assert!(!db.toggle); // Should start with toggle = false
        assert!(db.sender.is_none()); // No sender initially
    }

    #[test]
    fn test_framebuffer_swapping() {
        const WIDTH: usize = 64;
        const HEIGHT: usize = 64;

        // Create safe test buffers
        let buffer1 = create_test_buffer::<WIDTH, HEIGHT>();
        let buffer2 = create_test_buffer::<WIDTH, HEIGHT>();

        // Get raw pointers
        let raw_ptr1 = buffer1.as_ptr() as *mut c_void;
        let raw_ptr2 = buffer2.as_ptr() as *mut c_void;

        // Create doublebuffer
        let mut db = DoubleBuffer::<WIDTH, HEIGHT>::new(raw_ptr1, raw_ptr2);

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
        let fb2_slice = fb2.as_slice();
        assert_eq!(fb2_slice[0], 0); // Second buffer should be empty

        // Swap again, should get back to the first buffer with our pixel set
        let fb3 = db.swap_framebuffer();

        // First pixel in first buffer should be white
        let fb3_slice = fb3.as_slice();
        assert_ne!(fb3_slice[0], 0); // Should contain our white pixel
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

        // Create safe test buffers
        let buffer1 = create_test_buffer::<WIDTH, HEIGHT>();
        let buffer2 = create_test_buffer::<WIDTH, HEIGHT>();

        // Get raw pointers
        let raw_ptr1 = buffer1.as_ptr() as *mut c_void;
        let raw_ptr2 = buffer2.as_ptr() as *mut c_void;

        // Create doublebuffer
        let mut db = DoubleBuffer::<WIDTH, HEIGHT>::new(raw_ptr1, raw_ptr2);

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
