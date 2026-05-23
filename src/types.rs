use core::{ffi::c_void, ptr::null_mut};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Handle(*mut c_void);

impl Handle {
    /// # Safety
    ///
    /// The caller must ensure it's passing a valid pointer
    pub const unsafe fn from_raw(raw: *mut c_void) -> Self {
        Self(raw)
    }

    pub const fn null() -> Self {
        Handle(null_mut())
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Event(*mut c_void);

#[derive(Clone, Copy)]
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

impl Time {
    pub const fn year(&self) -> u16 {
        self.year
    }

    pub const fn month(&self) -> u8 {
        self.month
    }

    pub const fn day(&self) -> u8 {
        self.day
    }

    pub const fn hour(&self) -> u8 {
        self.hour
    }

    pub const fn minute(&self) -> u8 {
        self.minute
    }

    pub const fn second(&self) -> u8 {
        self.second
    }

    pub const fn nanosecond(&self) -> u32 {
        self.nanosecond
    }

    pub const fn time_zone(&self) -> i16 {
        self.time_zone
    }

    pub const fn daylight(&self) -> u8 {
        self.daylight
    }
}
