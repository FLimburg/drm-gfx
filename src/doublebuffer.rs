use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};
use crate::{drm_render_target::{RenderTarget, FramebufferTarget}, framebuffer::DmaReadyFramebuffer};
use log::info;

pub struct DoubleBuffer<const W: usize, const H: usize> {
    sender: Option<std::sync::mpsc::Sender<usize>>,
    toggle: bool,
    fbuf0: DmaReadyFramebuffer<W, H>,
    fbuf1: DmaReadyFramebuffer<W, H>,
    mutex: Arc<Mutex<bool>>,
}

impl<const W: usize, const H: usize> DoubleBuffer<W, H> {
    pub fn new(raw_framebuffer_0: *mut c_void, raw_framebuffer_1: *mut c_void) -> Self {
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

    pub fn start_thread(
        &mut self,
        display: RenderTarget,
    ) {
        info!("Starting fb writer thread");
        let (send, receive) = std::sync::mpsc::channel();

        self.sender = Some(send);

        let mutex2 = self.mutex.clone();
        let mut display = display;

        std::thread::spawn(move || loop {
            let ptr = receive.recv().unwrap();
            // println!("Received framebuffer pointer: {}", ptr);
            unsafe {
                let _lock = mutex2.lock().unwrap();

                let ptr = ptr as *mut [u32; 1024 * 600];
                let ptr = &mut *ptr;

                display.eat_framebuffer(ptr).unwrap();
                ptr.fill(0); // 2.2ms
            };
        });
    }

    pub fn swap_framebuffer(&mut self) -> &mut DmaReadyFramebuffer<W, H> {
        self.toggle = !self.toggle;

        if self.toggle {
            &mut self.fbuf0
        } else {
            &mut self.fbuf1
        }
    }

    pub fn send_framebuffer(&mut self) {
        {
            let _lock = self.mutex.lock().unwrap();
            std::mem::drop(_lock);
        }

        let fbuf = if self.toggle {
            &mut self.fbuf0
        } else {
            &mut self.fbuf1
        };

        if let Some(sender) = &self.sender {
            sender.send(fbuf.framebuffer as usize).unwrap();
        }
    }
}