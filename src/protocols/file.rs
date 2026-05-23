use core::{ffi::c_void, mem::MaybeUninit};

use alloc::vec;

use crate::{Guid, Result, Status, Time, WStr};

#[repr(C)]
pub struct File {
    revision: u64,
    open: unsafe extern "efiapi" fn(
        this: *mut Self,
        new_handle: *mut *mut File,
        file_name: *const u16,
        open_mode: FileMode,
        attributes: u64,
    ) -> Status,
    close: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    delete: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    read: unsafe extern "efiapi" fn(
        this: *mut Self,
        buffer_size: *mut usize,
        buffer: *mut c_void,
    ) -> Status,
    write: unsafe extern "efiapi" fn(
        this: *mut Self,
        buffer_size: *mut usize,
        buffer: *const c_void,
    ) -> Status,
    get_position: unsafe extern "efiapi" fn(this: *const Self, position: *mut u64) -> Status,
    set_position: unsafe extern "efiapi" fn(this: *mut Self, position: u64) -> Status,
    get_info: unsafe extern "efiapi" fn(
        this: *mut Self,
        information_type: *const Guid,
        buffer_size: *mut usize,
        buffer: *mut c_void,
    ) -> Status,
    set_info: unsafe extern "efiapi" fn(
        this: *mut Self,
        information_type: *const Guid,
        buffer_size: usize,
        buffer: *const c_void,
    ) -> Status,
    flush: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    open_ex: unsafe extern "efiapi" fn(
        this: *mut Self,
        new_handle: *mut *mut File,
        file_name: *const u16,
        open_mode: FileMode,
        attributes: u64,
        token: *mut c_void,
    ) -> Status,
    read_ex: unsafe extern "efiapi" fn(this: *mut Self, token: *mut c_void) -> Status,
    write_ex: unsafe extern "efiapi" fn(this: *mut Self, token: *mut c_void) -> Status,
    flush_ex: unsafe extern "efiapi" fn(this: *mut Self, token: *mut c_void) -> Status,
}

/// RAII owner of an open `EFI_FILE_PROTOCOL` handle.
///
/// Automatically calls `EFI_FILE_PROTOCOL.Close` on drop. Use
/// [`FileHandle::into_raw`] to take ownership of the raw pointer without
/// closing it.
pub struct FileHandle(pub(crate) *mut File);

impl FileHandle {
    /// Consumes the handle and returns the raw pointer without closing the file.
    /// The caller is responsible for eventually closing the handle.
    pub fn into_raw(self) -> *mut File {
        let ptr = self.0;
        core::mem::forget(self);
        ptr
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        // Errors from Close cannot be propagated here; ignore them.
        let _ = unsafe { ((*self.0).close)(self.0) };
    }
}

impl core::ops::Deref for FileHandle {
    type Target = File;
    fn deref(&self) -> &File {
        unsafe { &*self.0 }
    }
}

impl core::ops::DerefMut for FileHandle {
    fn deref_mut(&mut self) -> &mut File {
        unsafe { &mut *self.0 }
    }
}

#[repr(transparent)]
pub struct FileMode(u64);

impl FileMode {
    pub const READ: Self = Self(1);
    pub const WRITE: Self = Self(2);
    /// Must be combined with `READ` and/or `WRITE` (per UEFI spec §13.5.2).
    pub const CREATE: Self = Self(0x8000000000000000);
}

impl core::ops::BitOr for FileMode {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

#[repr(C)]
pub struct FileInfo {
    pub size: u64,
    pub file_size: u64,
    pub physical_size: u64,
    pub create_time: Time,
    pub last_access_time: Time,
    pub modification_time: Time,
    pub attribute: u64,
    pub file_name: [u16],
}

impl FileInfo {
    pub(crate) const GUID: Guid = Guid::new(
        0x09576e92_u32.to_ne_bytes(),
        0x6d3f_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );

    /// Interprets a raw byte buffer returned by [`File::get_info`] as a `&FileInfo`.
    ///
    /// # Safety
    ///
    /// `bytes` must have been filled by a successful `EFI_FILE_PROTOCOL.GetInfo`
    /// call and must be at least as large as the size reported by firmware.
    pub unsafe fn from_bytes(bytes: &[u8]) -> &Self {
        // `offset_of!` does not support DSTs. Use a sized sentinel with the same
        // leading fields to obtain the offset of `file_name` at compile time.
        #[repr(C)]
        struct FileInfoSized {
            size: u64,
            file_size: u64,
            physical_size: u64,
            create_time: Time,
            last_access_time: Time,
            modification_time: Time,
            attribute: u64,
        }
        const NAME_OFFSET: usize = core::mem::size_of::<FileInfoSized>();
        // Catch any future layout divergence between FileInfoSized and FileInfo.
        const _: () = assert!(
            NAME_OFFSET == 80,
            "FileInfoSized layout diverged from EFI_FILE_INFO"
        );

        let name_len = bytes.len().saturating_sub(NAME_OFFSET) / core::mem::size_of::<u16>();
        unsafe { &*core::ptr::from_raw_parts(bytes.as_ptr().cast::<()>(), name_len) }
    }

    pub fn file_name(&self) -> &WStr {
        unsafe { WStr::from_ptr(self.file_name.as_ptr()) }
    }
}

impl File {
    /// Opens a file relative to this directory handle.
    ///
    /// Returns a [`FileHandle`] that closes the file automatically on drop.
    /// The returned handle is independent of `self` and may outlive it.
    pub fn open(&mut self, file_name: &WStr, open_mode: FileMode) -> Result<FileHandle> {
        let mut file = MaybeUninit::<*mut File>::uninit();
        let status =
            unsafe { (self.open)(self, file.as_mut_ptr(), file_name.as_ptr(), open_mode, 0) };

        status.into_result()?;

        Ok(FileHandle(unsafe { file.assume_init() }))
    }

    pub fn set_position(&mut self, position: u64) -> Result {
        unsafe { (self.set_position)(self, position) }.into_result()
    }

    /// Reads up to `buf.len()` bytes from the current file position.
    /// Returns the number of bytes actually read; fewer bytes than requested
    /// indicates end-of-file.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut size = buf.len();

        unsafe { (self.read)(self, &mut size, buf.as_mut_ptr().cast()) }
            .into_result()
            .map(|_| size)
    }

    /// Writes up to `buf.len()` bytes at the current file position.
    /// Returns the number of bytes actually written.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut size = buf.len();

        unsafe { (self.write)(self, &mut size, buf.as_ptr().cast()) }
            .into_result()
            .map(|_| size)
    }

    pub fn get_position(&self) -> Result<u64> {
        let mut pos: u64 = 0;

        unsafe { (self.get_position)(self, &mut pos) }
            .into_result()
            .map(|_| pos)
    }

    /// Deletes the file and closes the handle. The handle must not be used after this call.
    pub fn delete(&mut self) -> Result {
        unsafe { (self.delete)(self) }.into_result()
    }

    pub fn flush(&mut self) -> Result {
        unsafe { (self.flush)(self) }.into_result()
    }

    /// Returns the raw `EFI_FILE_INFO` bytes for this file handle.
    /// Use [`FileInfo::from_bytes`] to interpret the result.
    pub fn get_info(&mut self) -> Result<alloc::vec::Vec<u8>> {
        let mut size: usize = 0;

        // First call: ask firmware for the required buffer size.
        let status =
            unsafe { (self.get_info)(self, &FileInfo::GUID, &mut size, core::ptr::null_mut()) };

        // Firmware must return BUFFER_TOO_SMALL with the required size.
        // Any other response (including an unexpected SUCCESS) is an error.
        if status != Status::BUFFER_TOO_SMALL {
            return Err(if status.is_error() {
                status
            } else {
                Status::DEVICE_ERROR
            });
        }

        // Second call: fill the correctly-sized buffer.
        let mut buf = vec![0u8; size];

        unsafe { (self.get_info)(self, &FileInfo::GUID, &mut size, buf.as_mut_ptr().cast()) }
            .into_result()
            .map(|_| buf)
    }
}
