use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

pub use crate::Guid;
use crate::{Handle, Result, Status};

pub trait HasGuid {
    const GUID: Guid;
}

pub trait HasProtocol: HasGuid {}

pub struct Protocol<P: HasProtocol> {
    raw: NonNull<P>,
    close: Option<(Handle, Handle)>,
}

impl<P: HasProtocol> Protocol<P> {
    pub(crate) fn new(raw: *mut P, handle: Handle, agent: Handle) -> Result<Self> {
        if raw.is_null() {
            return Err(Status::UNSUPPORTED);
        }
        Ok(Self {
            raw: unsafe { NonNull::new_unchecked(raw) },
            close: Some((handle, agent)),
        })
    }

    pub(crate) const fn new_unscoped(raw: *mut P) -> Result<Self> {
        if raw.is_null() {
            return Err(Status::UNSUPPORTED);
        }

        Ok(Self {
            raw: unsafe { NonNull::new_unchecked(raw) },
            close: None,
        })
    }

    pub const fn get(&self) -> &P {
        unsafe { self.raw.as_ref() }
    }

    pub const fn get_mut(&mut self) -> &mut P {
        unsafe { self.raw.as_mut() }
    }
}

impl<P: HasProtocol> Deref for Protocol<P> {
    type Target = P;

    fn deref(&self) -> &P {
        self.get()
    }
}

impl<P: HasProtocol> DerefMut for Protocol<P> {
    fn deref_mut(&mut self) -> &mut P {
        self.get_mut()
    }
}

impl<P: HasProtocol> Drop for Protocol<P> {
    fn drop(&mut self) {
        if let Some((handle, agent)) = self.close {
            let _ = crate::system_table()
                .boot_services()
                .close_protocol::<P>(&handle, &agent);
        }
    }
}
