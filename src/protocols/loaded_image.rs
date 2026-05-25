use core::ffi::c_void;

use crate::{
    Guid, Handle, HasGuid, HasProtocol, Status,
    protocols::DevicePath,
    tables::{MemoryType, SystemTable},
};

/// UEFI Loaded Image Protocol. Provides information about a loaded UEFI image.
/// (UEFI specification §9.1.1: EFI_LOADED_IMAGE_PROTOCOL)
#[repr(C)]
pub struct LoadedImage {
    /// Revision of the protocol.
    revision: u32,
    /// Handle of the image that loaded this image.
    parent_handle: Handle,
    /// Pointer to the UEFI System Table.
    system_table: *const SystemTable,
    /// Handle of the device on which the image is loaded.
    device_handle: Handle,
    /// Device path of the image file, or null for buffer-loaded images.
    file_path: *const DevicePath,
    /// Reserved. Must be null.
    reserved: *const c_void,
    /// Size of the load options, in bytes.
    load_option_size: u32,
    /// Pointer to the load options data.
    load_options: *const c_void,
    /// Base address of the loaded image.
    image_base: *const c_void,
    /// Size of the loaded image, in bytes.
    image_size: u64,
    /// Memory type for code sections.
    image_code_type: MemoryType,
    /// Memory type for data sections.
    image_data_type: MemoryType,
    /// Optional function to unload the image.
    unload: Option<unsafe extern "efiapi" fn(image_handle: Handle) -> Status>,
}

impl HasGuid for LoadedImage {
    const GUID: Guid = Guid::new(
        0x5B1B31A1_u32.to_ne_bytes(),
        0x9562_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8E,
        0x3F,
        [0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B],
    );
}
impl HasProtocol for LoadedImage {}

impl LoadedImage {
    /// Returns the handle of the device on which the image is loaded.
    pub fn device(&self) -> &Handle {
        &self.device_handle
    }

    /// Returns the device path of the loaded image, or `None` if the image was
    /// loaded from a buffer rather than a file.
    pub fn device_path(&self) -> Option<&DevicePath> {
        if self.file_path.is_null() {
            None
        } else {
            Some(unsafe { &*self.file_path })
        }
    }
}
