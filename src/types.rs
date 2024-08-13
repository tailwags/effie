use core::{ffi::c_void, ptr::null_mut};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Handle(*mut c_void);

impl Handle {
    pub const unsafe fn from_raw(raw: *mut c_void) -> Self {
        Self(raw)
    }

    pub const fn null() -> Self {
        Handle(null_mut())
    }
}

#[repr(transparent)]
pub struct Event(*mut c_void);

#[repr(C)]
pub struct Time {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    pad1: u8,
    nanosecond: u32,
    time_zone: i16,
    daylight: u8,
    pad2: u8,
}
