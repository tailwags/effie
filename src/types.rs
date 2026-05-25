use core::{ffi::c_void, ptr::null_mut};

/// UEFI handle.
///
/// A pointer to an opaque firmware object that represents a UEFI image, device,
/// or protocol instance.
///
/// All UEFI handles are typed by the protocol interfaces they support.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Handle(*mut c_void);

impl Handle {
    /// Creates a `Handle` from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure it is passing a valid pointer to a UEFI handle.
    pub const unsafe fn from_raw(raw: *mut c_void) -> Self {
        Self(raw)
    }

    /// Returns a null handle.
    pub const fn null() -> Self {
        Handle(null_mut())
    }
}

/// UEFI event handle.
///
/// An opaque pointer to a UEFI event object used for timer, notification, and
/// wait operations.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Event(*mut c_void);

/// UEFI time structure.
///
/// Represents a date and time, with time zone and daylight saving information.
///
/// UEFI specification §8.3: EFI_TIME.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Time {
    /// 1900 – 9999
    year: u16,
    /// 1 – 12
    month: u8,
    /// 1 – 31
    day: u8,
    /// 0 – 23
    hour: u8,
    /// 0 – 59
    minute: u8,
    /// 0 – 59
    second: u8,
    /// Padding to align nanosecond to 4-byte boundary.
    pad1: u8,
    /// 0 – 999,999,999
    nanosecond: u32,
    /// Minutes from UTC (−1440..=1440). `0x07FF` = unspecified.
    time_zone: i16,
    /// Daylight saving time flags.
    daylight: u8,
    /// Padding to align to 4-byte boundary.
    pad2: u8,
}

impl Time {
    /// Returns the year.
    pub const fn year(&self) -> u16 {
        self.year
    }

    /// Returns the month (1–12).
    pub const fn month(&self) -> u8 {
        self.month
    }

    /// Returns the day (1–31).
    pub const fn day(&self) -> u8 {
        self.day
    }

    /// Returns the hour (0–23).
    pub const fn hour(&self) -> u8 {
        self.hour
    }

    /// Returns the minute (0–59).
    pub const fn minute(&self) -> u8 {
        self.minute
    }

    /// Returns the second (0–59).
    pub const fn second(&self) -> u8 {
        self.second
    }

    /// Returns the nanosecond (0–999,999,999).
    pub const fn nanosecond(&self) -> u32 {
        self.nanosecond
    }

    /// Returns the time zone offset in minutes from UTC.
    ///
    /// A value of `0x07FF` indicates that the time zone is unspecified.
    pub const fn time_zone(&self) -> i16 {
        self.time_zone
    }

    /// Returns the daylight saving time setting.
    pub const fn daylight(&self) -> u8 {
        self.daylight
    }
}
