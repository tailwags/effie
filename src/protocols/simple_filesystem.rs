use core::mem::MaybeUninit;

use crate::{
    Guid, HasGuid, HasProtocol, Result, Status,
    protocols::{File, FileHandle},
};

/// UEFI Simple File System Protocol. Provides a minimal interface for file I/O on a
/// UEFI-compliant file system. (UEFI specification §13.4.1: EFI_SIMPLE_FILE_SYSTEM_PROTOCOL)
#[repr(C)]
pub struct SimpleFilesystem {
    /// Revision of the protocol.
    revision: u64,
    /// Opens the root directory of the volume. (§13.4.2)
    open_volume: unsafe extern "efiapi" fn(this: *mut Self, root: *mut *mut File) -> Status,
}

impl HasGuid for SimpleFilesystem {
    const GUID: Guid = Guid::new(
        0x0964e5b2_u32.to_ne_bytes(),
        0x6459_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );
}
impl HasProtocol for SimpleFilesystem {}

impl SimpleFilesystem {
    /// Opens the root directory of the volume. Returns a `FileHandle` for the root.
    /// (UEFI specification §13.4.2)
    pub fn open_volume(&mut self) -> Result<FileHandle> {
        let mut volume = MaybeUninit::<*mut File>::uninit();
        let status = unsafe { (self.open_volume)(self, volume.as_mut_ptr()) };

        status.into_result()?;

        FileHandle::new(unsafe { volume.assume_init() })
    }
}
