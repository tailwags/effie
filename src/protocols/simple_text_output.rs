use effie_macros::w_internal;

use crate::{Guid, HasGuid, HasProtocol, Result, Status, WStr};

#[repr(C)]
pub struct SimpleTextOutput {
    reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: bool) -> Status,
    output_string: unsafe extern "efiapi" fn(this: *mut Self, string: *const u16) -> Status,
    test_string: unsafe extern "efiapi" fn(this: *mut Self, string: *const u16) -> Status,
    query_mode: unsafe extern "efiapi" fn(
        this: *mut Self,
        mode_number: usize,
        columns: *mut usize,
        rows: *mut usize,
    ) -> Status,
    set_mode: unsafe extern "efiapi" fn(this: *mut Self, mode_number: usize) -> Status,
    set_attribute: unsafe extern "efiapi" fn(this: *mut Self, attribute: usize) -> Status,
    clear_screen: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    set_cursor_position:
        unsafe extern "efiapi" fn(this: *mut Self, column: usize, row: usize) -> Status,
    enable_cursor: unsafe extern "efiapi" fn(this: *mut Self, visible: bool) -> Status,
    mode: *mut SimpleTextOutputMode,
}

#[repr(C)]
pub struct SimpleTextOutputMode {
    pub max_mode: i32,
    pub mode: i32,
    pub attribute: i32,
    pub cursor_column: i32,
    pub cursor_row: i32,
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
    pub fn reset(&mut self, extended_verification: bool) -> Result {
        unsafe { (self.reset)(self, extended_verification) }.into_result()
    }

    pub fn output_string(&mut self, string: &WStr) -> Result {
        unsafe { (self.output_string)(self, string.as_ptr()) }.into_result()
    }

    /// Tests whether all characters in `string` can be rendered by this device.
    /// Returns `Ok(())` if all are supported, `Err(UNSUPPORTED)` if any are not.
    pub fn test_string(&mut self, string: &WStr) -> Result {
        unsafe { (self.test_string)(self, string.as_ptr()) }.into_result()
    }

    /// Queries the number of columns and rows for the given text mode number.
    pub fn query_mode(&mut self, mode_number: usize) -> Result<(usize, usize)> {
        let mut mode = (0, 0);

        unsafe { (self.query_mode)(self, mode_number, &mut mode.0, &mut mode.1) }.into_result()?;

        Ok(mode)
    }

    pub fn set_mode(&mut self, mode_number: usize) -> Result {
        unsafe { (self.set_mode)(self, mode_number) }.into_result()
    }

    /// Returns a reference to the current output mode state (read-only per spec).
    pub fn current_mode(&self) -> &SimpleTextOutputMode {
        unsafe { &*self.mode }
    }

    pub fn output_line(&mut self, string: &WStr) -> Result {
        self.output_string(string)?;
        self.output_string(w_internal!("\r\n"))
    }

    pub fn clear_screen(&mut self) -> Result {
        unsafe { (self.clear_screen)(self) }.into_result()
    }

    /// Sets the foreground and background colors for subsequent output.
    ///
    /// `attribute` encodes foreground (bits 0–3) and background (bits 4–6).
    /// Use `EFI_TEXT_ATTR(fg, bg) = fg | (bg << 4)`. Foreground supports 16
    /// colors (0x00–0x0F); background supports 8 colors (0x00–0x07).
    pub fn set_attribute(&mut self, attribute: usize) -> Result {
        unsafe { (self.set_attribute)(self, attribute) }.into_result()
    }

    pub fn set_cursor_position(&mut self, column: usize, row: usize) -> Result {
        unsafe { (self.set_cursor_position)(self, column, row) }.into_result()
    }

    pub fn enable_cursor(&mut self, visible: bool) -> Result {
        unsafe { (self.enable_cursor)(self, visible) }.into_result()
    }
}
