use core::{
    ffi::c_void,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use alloc::vec;

use crate::{Guid, HasGuid, Result, Status, Time, WStr, WString};

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

#[repr(transparent)]
pub struct FileHandle {
    raw: NonNull<File>,
}

impl FileHandle {
    pub(crate) fn new(raw: *mut File) -> Result<Self> {
        if raw.is_null() {
            return Err(Status::UNSUPPORTED);
        }
        Ok(Self {
            raw: unsafe { NonNull::new_unchecked(raw) },
        })
    }

    pub const fn get(&self) -> &File {
        unsafe { self.raw.as_ref() }
    }

    pub const fn get_mut(&mut self) -> &mut File {
        unsafe { self.raw.as_mut() }
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

impl Deref for FileHandle {
    type Target = File;
    fn deref(&self) -> &File {
        self.get()
    }
}

impl DerefMut for FileHandle {
    fn deref_mut(&mut self) -> &mut File {
        self.get_mut()
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
#[derive(Clone, Copy)]
struct FileInfoHeader {
    size: u64,
    file_size: u64,
    physical_size: u64,
    create_time: Time,
    last_access_time: Time,
    modification_time: Time,
    attribute: u64,
}

const FILE_INFO_HEADER_SIZE: usize = core::mem::size_of::<FileInfoHeader>();

pub struct FileInfo {
    header: FileInfoHeader,
    file_name: WString,
}

impl FileInfo {
    pub const fn file_size(&self) -> u64 {
        self.header.file_size
    }

    pub const fn physical_size(&self) -> u64 {
        self.header.physical_size
    }

    pub const fn create_time(&self) -> Time {
        self.header.create_time
    }

    pub const fn last_access_time(&self) -> Time {
        self.header.last_access_time
    }

    pub const fn modification_time(&self) -> Time {
        self.header.modification_time
    }

    pub const fn attribute(&self) -> u64 {
        self.header.attribute
    }

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
    /// Opens a file relative to this directory handle.
    ///
    /// Returns a [`FileHandle`] that closes the file automatically on drop.
    /// The returned handle is independent of `self` and may outlive it.
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

    /// Returns the `EFI_FILE_INFO` for this file handle.
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
