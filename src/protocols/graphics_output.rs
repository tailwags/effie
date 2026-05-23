use crate::{Guid, HasGuid, HasProtocol, Result, Status};

#[repr(C)]
pub struct GraphicsOutput {
    query_mode: unsafe extern "efiapi" fn(
        this: *const Self,
        mode_number: u32,
        size_of_info: *mut usize,
        info: *mut *const GraphicsOutputModeInformation,
    ) -> Status,
    set_mode: unsafe extern "efiapi" fn(this: *mut Self, mode_number: u32) -> Status,
    blt: unsafe extern "efiapi" fn(
        this: *mut Self,
        blt_buffer: *mut BltPixel,
        blt_operation: u32,
        source_x: usize,
        source_y: usize,
        destination_x: usize,
        destination_y: usize,
        width: usize,
        height: usize,
        delta: usize,
    ) -> Status,
    pub mode: *const GraphicsOutputMode,
}

#[repr(C)]
pub struct GraphicsOutputMode {
    pub max_mode: u32,
    pub mode: u32,
    pub info: *mut GraphicsOutputModeInformation,
    pub size_of_info: usize,
    pub frame_buffer_base: u64,
    pub frame_buffer_size: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BltPixel {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub reserved: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GraphicsOutputModeInformation {
    pub version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: GopPixelFormat,
    pub pixel_bitmask: PixelBitmask,
    pub pixels_per_scan_line: u32,
}

/// EFI_GRAPHICS_PIXEL_FORMAT — newtype so firmware-supplied values never create UB.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GopPixelFormat(u32);

impl GopPixelFormat {
    /// PixelRedGreenBlueReserved8BitPerColor
    pub const RGB: Self = Self(0);
    /// PixelBlueGreenRedReserved8BitPerColor
    pub const BGR: Self = Self(1);
    /// PixelBitMask — pixel layout defined by [`PixelBitmask`].
    pub const MASK: Self = Self(2);
    /// PixelBltOnly — no linear frame buffer.
    pub const BLT_ONLY: Self = Self(3);
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PixelBitmask {
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub reserved_mask: u32,
}

impl HasGuid for GraphicsOutput {
    const GUID: Guid = Guid::new(
        0x9042a9de_u32.to_ne_bytes(),
        0x23dc_u16.to_ne_bytes(),
        0x4a38_u16.to_ne_bytes(),
        0x96,
        0xfb,
        [0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
    );
}
impl HasProtocol for GraphicsOutput {}

impl GraphicsOutput {
    pub fn current_mode(&self) -> &GraphicsOutputMode {
        unsafe { &*self.mode }
    }

    pub fn current_mode_info(&self) -> &GraphicsOutputModeInformation {
        unsafe { &*self.current_mode().info }
    }

    pub fn query_mode(&self, mode_number: u32) -> Result<*const GraphicsOutputModeInformation> {
        let mut size: usize = 0;
        let mut info: *const GraphicsOutputModeInformation = core::ptr::null();
        unsafe { (self.query_mode)(self, mode_number, &mut size, &mut info) }.into_result()?;
        Ok(info)
    }

    /// Returns a by-value copy of the mode information for `mode_number`.
    /// The firmware-owned pointer returned by `query_mode` is valid only until
    /// the next `query_mode` or `set_mode` call, so copying immediately is safe.
    pub fn query_mode_info(&self, mode_number: u32) -> Result<GraphicsOutputModeInformation> {
        let ptr = self.query_mode(mode_number)?;
        Ok(unsafe { ptr.read() })
    }

    pub fn set_mode(&mut self, mode_number: u32) -> Result {
        unsafe { (self.set_mode)(self, mode_number) }.into_result()
    }
}
