use core::{ffi::c_void, mem::MaybeUninit};

use alloc::vec;

use itoa::Integer;

use crate::{system_table, Guid, Result, Status, Time, WStr};

#[repr(C)]
pub struct File {
    revision: u64,
    open: unsafe extern "efiapi" fn(
        this: &Self,
        new_handle: *mut *mut File,
        file_name: *const u16,
        open_mode: FileMode,
        attributes: u64,
    ) -> Status, // FIXME: type open_mode and attributes
    close: unsafe extern "efiapi" fn() -> Status,
    delete: unsafe extern "efiapi" fn() -> Status,
    read: unsafe extern "efiapi" fn(
        this: &Self,
        buffer_size: *mut usize,
        buffer: *mut c_void,
    ) -> Status,
    get_position: unsafe extern "efiapi" fn() -> Status,
    set_position: unsafe extern "efiapi" fn(this: &Self, position: u64) -> Status,
    get_info: unsafe extern "efiapi" fn(
        this: &Self,
        information_type: *const Guid,
        buffer_size: *mut usize,
        buffer: *mut c_void,
    ) -> Status,
    set_info: unsafe extern "efiapi" fn() -> Status,
    flush: unsafe extern "efiapi" fn() -> Status,
    open_ex: unsafe extern "efiapi" fn() -> Status,
    read_ex: unsafe extern "efiapi" fn() -> Status,
    write_ex: unsafe extern "efiapi" fn() -> Status,
    flush_ex: unsafe extern "efiapi" fn() -> Status,
}

#[repr(transparent)]
pub struct FileMode(u64);

// TODO: modes combinations
impl FileMode {
    pub const READ: Self = Self(1);
    pub const WRITE: Self = Self(2);
    pub const CREATE: Self = Self(0x8000000000000000);
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
    const GUID: Guid = Guid::new(
        0x09576e92_u32.to_ne_bytes(),
        0x6d3f_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );

    pub fn file_name(&self) -> &WStr {
        unsafe { WStr::from_ptr(self.file_name.as_ptr()) }
    }
}

impl File {
    pub fn open(&self, file_name: &[u16], open_mode: FileMode) -> Result<&File> {
        let mut file = MaybeUninit::<*mut File>::uninit();

        unsafe { (self.open)(self, file.as_mut_ptr(), file_name.as_ptr(), open_mode, 0) }
            .as_result_with(unsafe { &*file.assume_init() })
    }

    pub fn set_position(&self, position: u64) -> Result {
        unsafe { (self.set_position)(self, position) }.as_result()
    }

    pub fn read(&self, buf: &mut [u8]) -> Result {
        unsafe { (self.read)(self, &mut buf.len(), buf.as_mut_ptr().cast()) }.as_result()
    }

    // pub fn get_info(&self) -> Result<Box<[u8]>> {
    //     let mut buffer: Box<[u8]> = unsafe { Box::try_new([0; 1024]).unwrap_unchecked() };

    //     let info = unsafe {
    //         (self.get_info)(
    //             self,
    //             &FileInfo::GUID,
    //             &mut buffer.len(),
    //             buffer.as_mut_ptr().cast(),
    //         )
    //     };

    //     info.as_result_with(buffer)
    // }

    pub fn get_info(&self) -> Result<&mut FileInfo> {
        let mut buf = vec![0u8; 1];

        let len = buf.len();

        let status =
            unsafe { (self.get_info)(self, &FileInfo::GUID, &mut 0, buf.as_mut_ptr().cast()) };

        system_table().con_out().output_line(status.description())?;
        _print_num(len)?;

        unsafe {
            let ptr = buf.as_mut_ptr();
            let offset_of_str = 80usize;
            let name_ptr = ptr.add(offset_of_str).cast::<u16>();

            let name_len = WStr::from_ptr(name_ptr).to_bytes().len();
            Ok(&mut *core::ptr::from_raw_parts_mut(
                ptr.cast::<()>(),
                name_len,
            ))
        }
    }
}

fn _print_num<I: Integer>(i: I) -> Result {
    let mut buffer = itoa::Buffer::new();
    let printed = buffer.format(i);

    _print_utf8(printed)?;

    Ok(())
}

fn _print_utf8(string: &str) -> Result {
    let system_table = system_table();

    for c in string.encode_utf16() {
        system_table.con_out().output_string(&[c, 0])?;
    }

    // system_table.con_out().output_string(w!("\r\n"))?;

    Ok(())
}
