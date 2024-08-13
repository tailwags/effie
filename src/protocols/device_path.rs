use crate::{util::u16_slice_from_ptr, Guid, Protocol};

// FIXME: EFI_DEVICE_PATH_PROTOCOL
#[repr(C)]
pub struct DevicePath {
    ty: u8,
    sub_type: u8,
    length: [u8; 2],
    data: [u8; 0],
}

impl Protocol for DevicePath {
    const GUID: Guid = Guid::new(
        0x09576e91_u32.to_ne_bytes(),
        0x6d3f_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );
}

impl DevicePath {
    // pub const fn null() -> Self {
    //     Self { inner: null_mut() }
    // }

    pub fn as_path_name(&self) -> Option<&[u16]> {
        match (self.ty, self.sub_type) {
            (4, 4) => Some(unsafe { u16_slice_from_ptr(self.data.as_ptr().cast()) }),
            _ => None,
        }
    }
}
