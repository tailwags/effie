use crate::{Event, Guid, HasProtocol, Result, Status};

#[repr(C)]
pub struct SimpleTextInput {
    reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: bool) -> Status,
    read_key_stroke: unsafe extern "efiapi" fn(this: *mut Self, key: *mut InputKey) -> Status,
    wait_for_key: Event,
}

#[repr(C)]
pub struct InputKey {
    pub scan_code: u16,
    pub unicode_char: u16,
}

/// UEFI scan codes (Table B.1, UEFI spec §B).
///
/// `scan_code == 0` means the key is a printable character; read `unicode_char`.
impl InputKey {
    pub const SCAN_NULL: u16 = 0x00;
    pub const SCAN_UP: u16 = 0x01;
    pub const SCAN_DOWN: u16 = 0x02;
    pub const SCAN_RIGHT: u16 = 0x03;
    pub const SCAN_LEFT: u16 = 0x04;
    pub const SCAN_HOME: u16 = 0x05;
    pub const SCAN_END: u16 = 0x06;
    pub const SCAN_INSERT: u16 = 0x07;
    pub const SCAN_DELETE: u16 = 0x08;
    pub const SCAN_PAGE_UP: u16 = 0x09;
    pub const SCAN_PAGE_DOWN: u16 = 0x0a;
    pub const SCAN_F1: u16 = 0x0b;
    pub const SCAN_F2: u16 = 0x0c;
    pub const SCAN_F3: u16 = 0x0d;
    pub const SCAN_F4: u16 = 0x0e;
    pub const SCAN_F5: u16 = 0x0f;
    pub const SCAN_F6: u16 = 0x10;
    pub const SCAN_F7: u16 = 0x11;
    pub const SCAN_F8: u16 = 0x12;
    pub const SCAN_F9: u16 = 0x13;
    pub const SCAN_F10: u16 = 0x14;
    pub const SCAN_ESC: u16 = 0x17;
}

impl HasProtocol for SimpleTextInput {
    const GUID: Guid = Guid::new(
        0x387477c1_u32.to_ne_bytes(),
        0x69c7_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );
}

impl SimpleTextInput {
    pub fn reset(&mut self, extended_verification: bool) -> Result {
        unsafe { (self.reset)(self, extended_verification) }.into_result()
    }

    /// Returns the event that is signaled whenever a key is ready to read.
    /// Pass to `BootServices::wait_for_event` to block without busy-polling.
    pub fn wait_for_key_event(&self) -> Event {
        self.wait_for_key
    }

    /// Reads a single key stroke. Returns `Err(Status::NOT_READY)` if no key
    /// is currently pending; prefer `wait_for_key_event` over busy-polling.
    pub fn read_key_stroke(&mut self) -> Result<InputKey> {
        let mut key = InputKey {
            scan_code: 0,
            unicode_char: 0,
        };
        unsafe { (self.read_key_stroke)(self, &mut key) }.into_result()?;
        Ok(key)
    }
}
