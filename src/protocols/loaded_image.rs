use core::ffi::c_void;

use crate::{
    Guid, Handle, HasProtocol, Status,
    protocols::DevicePath,
    tables::{MemoryType, SystemTable},
};

#[repr(C)]
pub struct LoadedImage {
    revision: u32,
    parent_handle: Handle,
    system_table: *const SystemTable,
    device_handle: Handle,
    file_path: *const DevicePath,
    reserved: *const c_void,
    load_option_size: u32,
    load_options: *const c_void,
    image_base: *const c_void,
    image_size: u64,
    image_code_type: MemoryType,
    image_data_type: MemoryType,
    unload: Option<unsafe extern "efiapi" fn(image_handle: Handle) -> Status>,
}

impl HasProtocol for LoadedImage {
    const GUID: Guid = Guid::new(
        0x5B1B31A1_u32.to_ne_bytes(),
        0x9562_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8E,
        0x3F,
        [0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
    );
}

impl LoadedImage {
    pub fn device(&self) -> &Handle {
        &self.device_handle
    }

    /// Returns the device path of the loaded image, or `None` if the image was
    /// loaded from a buffer rather than a file
    pub fn device_path(&self) -> Option<&DevicePath> {
        if self.file_path.is_null() {
            None
        } else {
            Some(unsafe { &*self.file_path })
        }
    }
}
