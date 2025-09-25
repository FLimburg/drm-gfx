use crate::{
    drm_render_target::{FramebufferTarget, RenderTarget},
    framebuffer::DmaReadyFramebuffer,
};
use log::{debug, error, trace, info};
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

    pub fn start_thread(&mut self, display: RenderTarget) {
        info!("Starting fb writer thread");
        #[cfg(not(feature = "tokio"))]
        let (send, receive) = std::sync::mpsc::channel();
        #[cfg(feature = "tokio")]
        let (send, mut receive) = tokio::sync::mpsc::channel(16);

        self.sender = Some(send);

        let mutex2 = self.mutex.clone();
        let mut display = display;

        #[cfg(not(feature = "tokio"))]
        std::thread::spawn(move || {
            debug!("Framebuffer writer thread started for std runtime");
            loop {
                let ptr = receive.recv().unwrap();
                trace!("Received framebuffer pointer: {}", ptr);
                unsafe {
                    let _lock = mutex2.lock().unwrap();

                    let ptr = ptr as *mut [u32; 1024 * 600];
                    let ptr = &mut *ptr;

                    display.eat_framebuffer(ptr).unwrap();
                    ptr.fill(0); // 2.2ms
                };
            }
        });

        #[cfg(feature = "tokio")]
        tokio::spawn(async move {
            debug!("Framebuffer writer thread started for tokio runtime");
            loop {
                let ptr = receive.recv().await.unwrap();
                trace!("Received framebuffer pointer: {}", ptr);
                unsafe {
                    let _lock = mutex2.lock().unwrap();

                    let ptr = ptr as *mut [u32; 1024 * 600];
                    let ptr = &mut *ptr;

                    display.eat_framebuffer(ptr).unwrap();
                    ptr.fill(0); // 2.2ms
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

    #[cfg(not(feature = "tokio"))]
    pub fn send_framebuffer(&mut self) {
        {
            let _lock = self.mutex.lock().unwrap();
            std::mem::drop(_lock);
        }

        let fbuf = if self.toggle {
            trace!("sending framebuffer 0 ({})", self.fbuf0.framebuffer as usize);
            &mut self.fbuf0
        } else {
            trace!("sending framebuffer 1 ({})", self.fbuf1.framebuffer as usize);
            &mut self.fbuf1
        };

        if let Some(sender) = &self.sender {
            sender.send(fbuf.framebuffer as usize)
                .inspect_err(|msg| {
                    error!("Failed to send framebuffer: {}", msg);
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
            trace!("sending framebuffer 0 ({})", self.fbuf0.framebuffer as usize);
            &mut self.fbuf0
        } else {
            trace!("sending framebuffer 1 ({})", self.fbuf1.framebuffer as usize);
            &mut self.fbuf1
        };

        if let Some(sender) = &self.sender {
            sender.send(fbuf.framebuffer as usize).await
                .inspect_err(|msg| {
                    error!("Failed to send framebuffer: {}", msg);
                })
                .unwrap();
        }
    }
}
