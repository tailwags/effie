use core::ptr::addr_of;

#[repr(transparent)]
pub struct WStr {
    inner: [u16],
}

impl WStr {
    pub const unsafe fn from_bytes(bytes: &[u16]) -> &Self {
        unsafe { &*(bytes as *const [u16] as *const Self) }
    }

    pub const fn to_bytes(&self) -> &[u16] {
        // SAFETY: Transmuting a slice of `c_char`s to a slice of `u8`s
        // is safe on all supported targets.
        unsafe { &*(addr_of!(self.inner) as *const [u16]) }
    }

    pub const unsafe fn from_ptr<'a>(ptr: *const u16) -> &'a Self {
        #[inline]
        const unsafe fn u16_slice_from_ptr<'p>(ptr: *const u16) -> &'p [u16] {
            let mut len = 0;
            while *ptr.add(len) != 0u16 {
                len += 1
            }
            core::slice::from_raw_parts(ptr, len + 1)
        }

        Self::from_bytes(u16_slice_from_ptr(ptr))
    }

    pub const fn as_ptr(&self) -> *const u16 {
        self.inner.as_ptr()
    }
}
