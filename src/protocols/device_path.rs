use crate::{Guid, HasProtocol, WStr};

// FIXME: EFI_DEVICE_PATH_PROTOCOL
#[repr(C)]
pub struct DevicePath {
    ty: u8,
    sub_type: u8,
    length: [u8; 2],
    data: [u8; 0],
}

impl HasProtocol for DevicePath {
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

    pub fn as_path_name(&self) -> Option<&WStr> {
        match (self.ty, self.sub_type) {
            (4, 4) => {
                // node header: type(1) + sub_type(1) + length(2) = 4 bytes; data follows.
                let node_len = u16::from_le_bytes(self.length) as usize;
                let max_chars = node_len.saturating_sub(4) / core::mem::size_of::<u16>();
                let ptr = self.data.as_ptr().cast::<u16>();
                // UEFI allocates device path nodes from pool with ≥8-byte alignment, so
                // the 4-byte header guarantees `data` is at a u16-aligned offset.
                // Use read_unaligned defensively to remain sound even if that assumption
                // ever breaks.
                let terminator =
                    (0..max_chars).find(|&i| unsafe { ptr.add(i).read_unaligned() == 0 })?;
                let len = terminator + 1; // include the null
                Some(unsafe { WStr::from_slice_unchecked(core::slice::from_raw_parts(ptr, len)) })
            }
            _ => None,
        }
    }
}
