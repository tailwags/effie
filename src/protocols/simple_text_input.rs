use crate::{Event, Guid, HasGuid, HasProtocol, Result, Status};

/// UEFI Simple Text Input Protocol. Defines an input stream that contains Unicode
/// characters and UEFI scan codes. (UEFI specification §12.1: EFI_SIMPLE_TEXT_INPUT_PROTOCOL)
#[repr(C)]
pub struct SimpleTextInput {
    /// Resets the text input device. (§12.1.2.1)
    reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: bool) -> Status,
    /// Reads the next key stroke from the input device. (§12.1.2.2)
    read_key_stroke: unsafe extern "efiapi" fn(this: *mut Self, key: *mut InputKey) -> Status,
    /// Event that is signaled when a key is available.
    wait_for_key: Event,
}

/// EFI_INPUT_KEY. Represents a single key press. (UEFI specification §12.1.2)
#[repr(C)]
pub struct InputKey {
    /// The scan code for the key. If `0x00`, the Unicode character is valid.
    /// UEFI scan codes (UEFI specification §B). `scan_code == 0` means the key is
    /// a printable character; read `unicode_char`.
    pub scan_code: u16,
    /// The Unicode character for the key.
    pub unicode_char: u16,
}

impl InputKey {
    /// Scan code for null input.
    pub const SCAN_NULL: u16 = 0x00;
    /// Scan code for the Up Arrow key.
    pub const SCAN_UP: u16 = 0x01;
    /// Scan code for the Down Arrow key.
    pub const SCAN_DOWN: u16 = 0x02;
    /// Scan code for the Right Arrow key.
    pub const SCAN_RIGHT: u16 = 0x03;
    /// Scan code for the Left Arrow key.
    pub const SCAN_LEFT: u16 = 0x04;
    /// Scan code for the Home key.
    pub const SCAN_HOME: u16 = 0x05;
    /// Scan code for the End key.
    pub const SCAN_END: u16 = 0x06;
    /// Scan code for the Insert key.
    pub const SCAN_INSERT: u16 = 0x07;
    /// Scan code for the Delete key.
    pub const SCAN_DELETE: u16 = 0x08;
    /// Scan code for the Page Up key.
    pub const SCAN_PAGE_UP: u16 = 0x09;
    /// Scan code for the Page Down key.
    pub const SCAN_PAGE_DOWN: u16 = 0x0a;
    /// Scan code for the F1 key.
    pub const SCAN_F1: u16 = 0x0b;
    /// Scan code for the F2 key.
    pub const SCAN_F2: u16 = 0x0c;
    /// Scan code for the F3 key.
    pub const SCAN_F3: u16 = 0x0d;
    /// Scan code for the F4 key.
    pub const SCAN_F4: u16 = 0x0e;
    /// Scan code for the F5 key.
    pub const SCAN_F5: u16 = 0x0f;
    /// Scan code for the F6 key.
    pub const SCAN_F6: u16 = 0x10;
    /// Scan code for the F7 key.
    pub const SCAN_F7: u16 = 0x11;
    /// Scan code for the F8 key.
    pub const SCAN_F8: u16 = 0x12;
    /// Scan code for the F9 key.
    pub const SCAN_F9: u16 = 0x13;
    /// Scan code for the F10 key.
    pub const SCAN_F10: u16 = 0x14;
    /// Scan code for the Escape key.
    pub const SCAN_ESC: u16 = 0x17;
}

impl HasGuid for SimpleTextInput {
    const GUID: Guid = Guid::new(
        0x387477c1_u32.to_ne_bytes(),
        0x69c7_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );
}
impl HasProtocol for SimpleTextInput {}

impl SimpleTextInput {
    /// Resets the text input device hardware. (UEFI specification §12.1.2.1)
    pub fn reset(&mut self, extended_verification: bool) -> Result {
        unsafe { (self.reset)(self, extended_verification) }.into_result()
    }

    /// Returns the event that is signaled when a key is ready to read. Pass to
    /// `BootServices::wait_for_event` to block without busy-polling.
    pub fn wait_for_key_event(&self) -> Event {
        self.wait_for_key
    }

    /// Reads a single key stroke from the input device. Returns `Err(Status::NOT_READY)`
    /// if no key is pending. (UEFI specification §12.1.2.2)
    pub fn read_key_stroke(&mut self) -> Result<InputKey> {
        let mut key = InputKey {
            scan_code: 0,
            unicode_char: 0,
        };
        unsafe { (self.read_key_stroke)(self, &mut key) }.into_result()?;
        Ok(key)
    }
}
