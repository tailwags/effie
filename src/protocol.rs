use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

pub use crate::Guid;
use crate::{Handle, Result, Status};

/// A protocol type that has an associated UEFI protocol GUID.
pub trait HasGuid {
    /// The UEFI protocol GUID that identifies this protocol interface.
    const GUID: Guid;
}

/// Marker trait for UEFI protocol types.
///
/// Implemented for types that represent UEFI protocol interfaces and have a
/// corresponding GUID via [`HasGuid`].
pub trait HasProtocol: HasGuid {}

/// RAII wrapper around a UEFI protocol interface pointer.
///
/// Opening a protocol via [`crate::tables::BootServices::open_protocol`] returns a
/// `Protocol<P>` that automatically calls `CloseProtocol` on drop. Protocols
/// obtained via `locate_protocol` or from the system table (e.g. `ConOut`) do
/// **not** close on drop, the firmware owns those pointers.
pub struct Protocol<P: HasProtocol> {
    /// Non-null pointer to the protocol interface.
    raw: NonNull<P>,
    /// `Some((handle, agent))` if this protocol should be closed on drop.
    close: Option<(Handle, Handle)>,
}

impl<P: HasProtocol> Protocol<P> {
    /// Creates a new `Protocol<P>` from a raw interface pointer, recording the handle and agent
    /// so that `CloseProtocol` is called on drop.
    pub(crate) fn new(raw: *mut P, handle: Handle, agent: Handle) -> Result<Self> {
        if raw.is_null() {
            return Err(Status::UNSUPPORTED);
        }
        Ok(Self {
            raw: unsafe { NonNull::new_unchecked(raw) },
            close: Some((handle, agent)),
        })
    }

    /// Creates a new `Protocol<P>` from a raw interface pointer without drop-side cleanup.
    ///
    /// Used for protocols obtained from the system table or via `LocateProtocol`, which the
    /// firmware owns and must not be closed by the caller.
    pub(crate) const fn new_unscoped(raw: *mut P) -> Result<Self> {
        if raw.is_null() {
            return Err(Status::UNSUPPORTED);
        }

        Ok(Self {
            raw: unsafe { NonNull::new_unchecked(raw) },
            close: None,
        })
    }

    /// Returns a shared reference to the underlying protocol interface.
    pub const fn get(&self) -> &P {
        unsafe { self.raw.as_ref() }
    }

    /// Returns a mutable reference to the underlying protocol interface.
    pub const fn get_mut(&mut self) -> &mut P {
        unsafe { self.raw.as_mut() }
    }
}

/// Dereferences to the underlying protocol interface.
impl<P: HasProtocol> Deref for Protocol<P> {
    type Target = P;

    fn deref(&self) -> &P {
        self.get()
    }
}

/// Mutably dereferences to the underlying protocol interface.
impl<P: HasProtocol> DerefMut for Protocol<P> {
    fn deref_mut(&mut self) -> &mut P {
        self.get_mut()
    }
}

/// Closes the protocol if this handle was created via [`open_protocol`].
///
/// [`open_protocol`]: crate::tables::BootServices::open_protocol
impl<P: HasProtocol> Drop for Protocol<P> {
    fn drop(&mut self) {
        if let Some((handle, agent)) = self.close {
            let _ = crate::system_table()
                .boot_services()
                .close_protocol::<P>(&handle, &agent);
        }
    }
}
