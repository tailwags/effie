#[repr(transparent)]
pub struct WStr {
    inner: [u16],
}

impl WStr {
    /// # Safety
    ///
    /// The caller must ensure the bytes they are passing are valid utf-16
    pub const unsafe fn from_bytes(bytes: &[u16]) -> &Self {
        unsafe { &*(bytes as *const [u16] as *const Self) }
    }

    pub const fn to_bytes(&self) -> &[u16] {
        &self.inner
    }

    /// # Safety
    ///
    /// The caller must ensure it's passing a valid pointer
    pub const unsafe fn from_ptr<'a>(ptr: *const u16) -> &'a Self {
        unsafe {
            #[inline]
            const unsafe fn u16_slice_from_ptr<'p>(ptr: *const u16) -> &'p [u16] {
                unsafe {
                    let mut len = 0;
                    while *ptr.add(len) != 0u16 {
                        len += 1
                    }
                    core::slice::from_raw_parts(ptr, len + 1)
                }
            }

            Self::from_bytes(u16_slice_from_ptr(ptr))
        }
    }

    pub const fn as_ptr(&self) -> *const u16 {
        self.inner.as_ptr()
    }
}
