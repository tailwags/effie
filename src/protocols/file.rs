use core::{
    ffi::c_void,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use alloc::vec;

use bitflags::bitflags;

use crate::{Guid, HasGuid, Result, Status, Time, WStr, WString};

/// UEFI File Protocol. Provides file I/O operations on a UEFI-compliant file system.
/// (UEFI specification §13.5.1: EFI_FILE_PROTOCOL)
#[repr(C)]
pub struct File {
    /// Revision of the protocol.
    revision: u64,
    /// Opens a file relative to this file handle. (§13.5.2)
    open: unsafe extern "efiapi" fn(
        this: *mut Self,
        new_handle: *mut *mut File,
        file_name: *const u16,
        open_mode: FileMode,
        attributes: u64,
    ) -> Status,
    /// Closes the file handle. (§13.5.3)
    close: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    /// Deletes the file. (§13.5.4)
    delete: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    /// Reads data from the file. (§13.5.5)
    read: unsafe extern "efiapi" fn(
        this: *mut Self,
        buffer_size: *mut usize,
        buffer: *mut c_void,
    ) -> Status,
    /// Writes data to the file. (§13.5.6)
    write: unsafe extern "efiapi" fn(
        this: *mut Self,
        buffer_size: *mut usize,
        buffer: *const c_void,
    ) -> Status,
    /// Returns the current file position. (§13.5.12)
    get_position: unsafe extern "efiapi" fn(this: *const Self, position: *mut u64) -> Status,
    /// Sets the current file position. (§13.5.11)
    set_position: unsafe extern "efiapi" fn(this: *mut Self, position: u64) -> Status,
    /// Gets file information. (§13.5.13)
    get_info: unsafe extern "efiapi" fn(
        this: *mut Self,
        information_type: *const Guid,
        buffer_size: *mut usize,
        buffer: *mut c_void,
    ) -> Status,
    /// Sets file information. (§13.5.14)
    set_info: unsafe extern "efiapi" fn(
        this: *mut Self,
        information_type: *const Guid,
        buffer_size: usize,
        buffer: *const c_void,
    ) -> Status,
    /// Flushes buffered data to the device. (§13.5.15)
    flush: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    /// Opens a file (asynchronous variant). (§13.5.7)
    open_ex: unsafe extern "efiapi" fn(
        this: *mut Self,
        new_handle: *mut *mut File,
        file_name: *const u16,
        open_mode: FileMode,
        attributes: u64,
        token: *mut c_void,
    ) -> Status,
    /// Reads data (asynchronous variant). (§13.5.8)
    read_ex: unsafe extern "efiapi" fn(this: *mut Self, token: *mut c_void) -> Status,
    /// Writes data (asynchronous variant). (§13.5.9)
    write_ex: unsafe extern "efiapi" fn(this: *mut Self, token: *mut c_void) -> Status,
    /// Flushes data (asynchronous variant). (§13.5.10)
    flush_ex: unsafe extern "efiapi" fn(this: *mut Self, token: *mut c_void) -> Status,
}

/// RAII wrapper around a `*mut File`. Closes the file automatically on drop.
#[repr(transparent)]
pub struct FileHandle {
    /// Non-null pointer to the UEFI File protocol interface.
    raw: NonNull<File>,
}

impl FileHandle {
    /// Creates a new `FileHandle` from a raw `*mut File` pointer, returning
    /// `Err(Status::UNSUPPORTED)` if the pointer is null.
    pub(crate) fn new(raw: *mut File) -> Result<Self> {
        if raw.is_null() {
            return Err(Status::UNSUPPORTED);
        }
        Ok(Self {
            raw: unsafe { NonNull::new_unchecked(raw) },
        })
    }

    /// Returns a shared reference to the underlying `File`.
    pub const fn get(&self) -> &File {
        unsafe { self.raw.as_ref() }
    }

    /// Returns a mutable reference to the underlying `File`.
    pub const fn get_mut(&mut self) -> &mut File {
        unsafe { self.raw.as_mut() }
    }
}

impl Drop for FileHandle {
    /// Closes the file handle on drop.
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Dereferences to the underlying `File`.
impl Deref for FileHandle {
    type Target = File;
    fn deref(&self) -> &File {
        self.get()
    }
}

/// Mutably dereferences to the underlying `File`.
impl DerefMut for FileHandle {
    fn deref_mut(&mut self) -> &mut File {
        self.get_mut()
    }
}

bitflags! {
    /// UEFI file open mode. Specifies the access mode for opening a file.
    /// (UEFI specification §13.5.2)
    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct FileMode: u64 {
        /// Opens the file for reading.
        const READ   = 0x0000000000000001;
        /// Opens the file for writing.
        const WRITE  = 0x0000000000000002;
        /// Creates a new file if it does not exist. Must be combined with `READ` and/or `WRITE`.
        const CREATE = 0x8000000000000000;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FileInfoHeader {
    /// Size of the EFI_FILE_INFO structure (including the null terminator).
    size: u64,
    /// Size of the file in bytes.
    file_size: u64,
    /// Physical size of the file on the volume.
    physical_size: u64,
    /// Time the file was created.
    create_time: Time,
    /// Time the file was last accessed.
    last_access_time: Time,
    /// Time the file was last modified.
    modification_time: Time,
    /// File attributes.
    attribute: u64,
}

const FILE_INFO_HEADER_SIZE: usize = core::mem::size_of::<FileInfoHeader>();

/// EFI_FILE_INFO. Provides information about a file on the volume.
/// (UEFI specification §13.5.13)
pub struct FileInfo {
    /// Parsed EFI_FILE_INFO header fields.
    header: FileInfoHeader,
    /// File name extracted from the raw buffer.
    file_name: WString,
}

impl FileInfo {
    /// Returns the size of the file in bytes.
    pub const fn file_size(&self) -> u64 {
        self.header.file_size
    }

    /// Returns the physical size of the file on the volume.
    pub const fn physical_size(&self) -> u64 {
        self.header.physical_size
    }

    /// Returns the time the file was created.
    pub const fn create_time(&self) -> Time {
        self.header.create_time
    }

    /// Returns the time the file was last accessed.
    pub const fn last_access_time(&self) -> Time {
        self.header.last_access_time
    }

    /// Returns the time the file was last modified.
    pub const fn modification_time(&self) -> Time {
        self.header.modification_time
    }

    /// Returns the file attributes.
    pub const fn attribute(&self) -> u64 {
        self.header.attribute
    }

    /// Returns the file name.
    pub fn file_name(&self) -> &WStr {
        &self.file_name
    }
}

impl HasGuid for FileInfoHeader {
    const GUID: Guid = Guid::new(
        0x09576e92_u32.to_ne_bytes(),
        0x6d3f_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );
}

impl File {
    /// Opens a file relative to this directory handle. (UEFI specification §13.5.2)
    pub fn open(&mut self, file_name: &WStr, open_mode: FileMode) -> Result<FileHandle> {
        let mut file = MaybeUninit::<*mut File>::uninit();
        let status =
            unsafe { (self.open)(self, file.as_mut_ptr(), file_name.as_ptr(), open_mode, 0) };

        status.into_result()?;

        FileHandle::new(unsafe { file.assume_init() })
    }

    fn close(&mut self) -> Result<()> {
        unsafe { (self.close)(self) }.into_result()
    }

    /// Sets the current file position. (UEFI specification §13.5.11)
    pub fn set_position(&mut self, position: u64) -> Result {
        unsafe { (self.set_position)(self, position) }.into_result()
    }

    /// Reads data from the file at the current position. Returns the number of bytes
    /// read. (UEFI specification §13.5.5)
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut size = buf.len();

        unsafe { (self.read)(self, &mut size, buf.as_mut_ptr().cast()) }
            .into_result()
            .map(|_| size)
    }

    /// Writes data to the file at the current position. Returns the number of bytes
    /// written. (UEFI specification §13.5.6)
    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut size = buf.len();

        unsafe { (self.write)(self, &mut size, buf.as_ptr().cast()) }
            .into_result()
            .map(|_| size)
    }

    /// Returns the current file position. (UEFI specification §13.5.12)
    pub fn get_position(&self) -> Result<u64> {
        let mut pos: u64 = 0;

        unsafe { (self.get_position)(self, &mut pos) }
            .into_result()
            .map(|_| pos)
    }

    /// Deletes the file and closes the handle. (UEFI specification §13.5.4)
    pub fn delete(&mut self) -> Result {
        unsafe { (self.delete)(self) }.into_result()
    }

    /// Flushes all modified data to the file system. (UEFI specification §13.5.15)
    pub fn flush(&mut self) -> Result {
        unsafe { (self.flush)(self) }.into_result()
    }

    /// Returns the EFI_FILE_INFO for this file handle. (UEFI specification §13.5.13)
    pub fn get_info(&mut self) -> Result<FileInfo> {
        let mut size: usize = 0;

        let status = unsafe {
            (self.get_info)(
                self,
                &<FileInfoHeader as HasGuid>::GUID,
                &mut size,
                core::ptr::null_mut(),
            )
        };

        if status != Status::BUFFER_TOO_SMALL {
            return Err(if status.is_error() {
                status
            } else {
                Status::DEVICE_ERROR
            });
        }

        let mut buf = vec![0u8; size];

        unsafe {
            (self.get_info)(
                self,
                &<FileInfoHeader as HasGuid>::GUID,
                &mut size,
                buf.as_mut_ptr().cast(),
            )
        }
        .into_result()?;

        if size < FILE_INFO_HEADER_SIZE {
            return Err(Status::DEVICE_ERROR);
        }

        let header = unsafe { buf.as_ptr().cast::<FileInfoHeader>().read_unaligned() };

        let file_name_bytes = size - FILE_INFO_HEADER_SIZE;
        if file_name_bytes < 2 || !file_name_bytes.is_multiple_of(2) {
            return Err(Status::DEVICE_ERROR);
        }

        let file_name_slice = unsafe {
            core::slice::from_raw_parts(
                buf.as_ptr().add(FILE_INFO_HEADER_SIZE) as *const u16,
                file_name_bytes / 2,
            )
        };

        let file_name = WStr::from_slice(file_name_slice)
            .ok_or(Status::DEVICE_ERROR)?
            .into();

        Ok(FileInfo { header, file_name })
    }
}
