use core::{
    marker::PhantomData,
    sync::atomic::{AtomicBool, Ordering},
};
use rbrew_shared::iotype;

iotype! {
    pub type VI: 0xcc002000, 0x100 {
        vtr: mut u16 = 0x00,
        dcr: mut u16 = 0x02,
        htro: mut u32 = 0x04,
        htr1: mut u32 = 0x08,
        vto: mut u32 = 0x0c,
        vte: mut u32 = 0x10,
        bbei: mut u32 = 0x14,
        bboi: mut u32 = 0x18,
        tfbl: mut u32 = 0x1c,
        tfbr: mut u32 = 0x20,
        bfbl: mut u32 = 0x24,
        bfbr: mut u32 = 0x28,
        dpv: const u16 = 0x2c,
        dph: const u16 = 0x2e,
        di0: mut u32 = 0x30,
        di1: mut u32 = 0x34,
        di2: mut u32 = 0x38,
        di3: mut u32 = 0x3c,
        dl0: mut u32 = 0x40,
        dl1: mut u32 = 0x44,
        hsw: mut u16 = 0x48,
        hsr: mut u16 = 0x4a,
        fct0: mut u32 = 0x4c,
        fct1: mut u32 = 0x50,
    }
}

static IS_INIT: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub enum VideoInitError {
    AlreadyInitialized,
}

#[derive(Clone, Copy)]
pub struct VideoContext {
    // Makes the type non-trivially constructible.
    _mark: PhantomData<()>,
}

impl VideoContext {
    fn init_video() -> Result<(), VideoInitError> {
        Framebuffer::init()?;
        Ok(())
    }

    pub fn init() -> Result<(), VideoInitError> {
        if IS_INIT.load(Ordering::Acquire) {
            IS_INIT.store(true, Ordering::Release);
            Self::init_video()
        } else {
            Err(VideoInitError::AlreadyInitialized)
        }
    }

    pub fn global() -> Self {
        #[allow(unreachable_patterns)]
        match Self::init() {
            Err(VideoInitError::AlreadyInitialized) | Ok(_) => {}
            Err(e) => panic!("video failed to initialize: {e:?}"),
        }
        unsafe { Self::global_unchecked() }
    }

    /// # Safety
    /// Requires that the global video context has been initialized.
    /// This is ensured by [`Self::global`] or [`Self::init`].
    pub unsafe fn global_unchecked() -> Self {
        Self { _mark: PhantomData }
    }

    #[inline]
    pub fn frambuffer(self) -> Framebuffer {
        Framebuffer { _mark: PhantomData }
    }
}

#[derive(Clone, Copy)]
pub struct Framebuffer {
    // Makes the type non-trivially constructible.
    _mark: PhantomData<()>,
}

impl Framebuffer {
    fn init() -> Result<(), VideoInitError> {
        Ok(())
    }

    #[inline]
    pub fn global() -> Self {
        VideoContext::global().frambuffer()
    }
}
