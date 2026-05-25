use effie_macros::w_internal;

use crate::{Guid, HasGuid, HasProtocol, Result, Status, WStr};

/// UEFI Simple Text Output Protocol. Used to control text-based output devices.
/// (UEFI specification §12.4: EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL)
#[repr(C)]
pub struct SimpleTextOutput {
    /// Resets the text output device. (§12.4.2)
    reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: bool) -> Status,
    /// Displays a null-terminated UTF-16 string. (§12.4.3)
    output_string: unsafe extern "efiapi" fn(this: *mut Self, string: *const u16) -> Status,
    /// Tests whether the device supports a given string. (§12.4.4)
    test_string: unsafe extern "efiapi" fn(this: *mut Self, string: *const u16) -> Status,
    /// Queries the device for a supported text mode. (§12.4.5)
    query_mode: unsafe extern "efiapi" fn(
        this: *mut Self,
        mode_number: usize,
        columns: *mut usize,
        rows: *mut usize,
    ) -> Status,
    /// Sets the text output device mode. (§12.4.6)
    set_mode: unsafe extern "efiapi" fn(this: *mut Self, mode_number: usize) -> Status,
    /// Sets the background and foreground color attributes. (§12.4.7)
    set_attribute: unsafe extern "efiapi" fn(this: *mut Self, attribute: usize) -> Status,
    /// Clears the screen to the current background color. (§12.4.8)
    clear_screen: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    /// Sets the cursor position. (§12.4.9)
    set_cursor_position:
        unsafe extern "efiapi" fn(this: *mut Self, column: usize, row: usize) -> Status,
    /// Enables or disables the cursor. (§12.4.10)
    enable_cursor: unsafe extern "efiapi" fn(this: *mut Self, visible: bool) -> Status,
    /// Pointer to the current mode structure.
    mode: *mut SimpleTextOutputMode,
}

/// SIMPLE_TEXT_OUTPUT_MODE. Contains the current text output device mode state.
/// (UEFI specification §12.4)
#[repr(C)]
pub struct SimpleTextOutputMode {
    /// The number of modes supported by QueryMode() and SetMode().
    pub max_mode: i32,
    /// The text mode of the output device.
    pub mode: i32,
    /// The current character output attribute.
    pub attribute: i32,
    /// The cursor's column.
    pub cursor_column: i32,
    /// The cursor's row.
    pub cursor_row: i32,
    /// The cursor is currently visible or not.
    pub cursor_visible: bool,
}

impl HasGuid for SimpleTextOutput {
    const GUID: Guid = Guid::new(
        0x387477c2_u32.to_ne_bytes(),
        0x69c7_u16.to_ne_bytes(),
        0x11d2_u16.to_ne_bytes(),
        0x8e,
        0x39,
        [0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    );
}
impl HasProtocol for SimpleTextOutput {}

impl SimpleTextOutput {
    /// Resets the text output device hardware. (UEFI specification §12.4.2)
    pub fn reset(&mut self, extended_verification: bool) -> Result {
        unsafe { (self.reset)(self, extended_verification) }.into_result()
    }

    /// Displays the string on the device at the current cursor location. (UEFI specification §12.4.3)
    pub fn output_string(&mut self, string: &WStr) -> Result {
        unsafe { (self.output_string)(self, string.as_ptr()) }.into_result()
    }

    /// Tests to see if the ConsoleOut device supports this string. (UEFI specification §12.4.4)
    pub fn test_string(&mut self, string: &WStr) -> Result {
        unsafe { (self.test_string)(self, string.as_ptr()) }.into_result()
    }

    /// Queries information concerning the output device's supported text mode. (UEFI specification §12.4.5)
    pub fn query_mode(&mut self, mode_number: usize) -> Result<(usize, usize)> {
        let mut mode = (0, 0);

        unsafe { (self.query_mode)(self, mode_number, &mut mode.0, &mut mode.1) }.into_result()?;

        Ok(mode)
    }

    /// Sets the current mode of the output device. (UEFI specification §12.4.6)
    pub fn set_mode(&mut self, mode_number: usize) -> Result {
        unsafe { (self.set_mode)(self, mode_number) }.into_result()
    }

    /// Returns a reference to the current output mode state.
    pub fn current_mode(&self) -> &SimpleTextOutputMode {
        unsafe { &*self.mode }
    }

    /// Writes a string followed by CR+LF to the output device.
    pub fn output_line(&mut self, string: &WStr) -> Result {
        self.output_string(string)?;
        self.output_string(w_internal!("\r\n"))
    }

    /// Clears the screen with the currently set background color. (UEFI specification §12.4.8)
    pub fn clear_screen(&mut self) -> Result {
        unsafe { (self.clear_screen)(self) }.into_result()
    }

    /// Sets the background and foreground colors for subsequent output. (UEFI specification §12.4.7)
    pub fn set_attribute(&mut self, attribute: usize) -> Result {
        unsafe { (self.set_attribute)(self, attribute) }.into_result()
    }

    /// Sets the current cursor position. (UEFI specification §12.4.9)
    pub fn set_cursor_position(&mut self, column: usize, row: usize) -> Result {
        unsafe { (self.set_cursor_position)(self, column, row) }.into_result()
    }

    /// Turns the visibility of the cursor on/off. (UEFI specification §12.4.10)
    pub fn enable_cursor(&mut self, visible: bool) -> Result {
        unsafe { (self.enable_cursor)(self, visible) }.into_result()
    }
}
