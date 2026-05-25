use crate::{Guid, HasGuid, HasProtocol, Result, Status};

/// UEFI Graphics Output Protocol. Provides a framebuffer-based graphics output interface.
/// (UEFI specification §12.9.2: EFI_GRAPHICS_OUTPUT_PROTOCOL)
#[repr(C)]
pub struct GraphicsOutput {
    /// Returns mode information for a given mode number. (§12.9.2.1)
    query_mode: unsafe extern "efiapi" fn(
        this: *const Self,
        mode_number: u32,
        size_of_info: *mut usize,
        info: *mut *const GraphicsOutputModeInformation,
    ) -> Status,
    /// Sets the graphics output mode. (§12.9.2.2)
    set_mode: unsafe extern "efiapi" fn(this: *mut Self, mode_number: u32) -> Status,
    /// Performs a block-transfer (BLT) operation. (§12.9.2.3)
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
    /// Pointer to the current graphics output mode.
    pub mode: *const GraphicsOutputMode,
}

/// EFI_GRAPHICS_OUTPUT_PROTOCOL_MODE. Current mode information for the graphics output device.
/// (UEFI specification §12.9)
#[repr(C)]
pub struct GraphicsOutputMode {
    /// The number of modes the device supports.
    pub max_mode: u32,
    /// The current mode number.
    pub mode: u32,
    /// A pointer to the current mode information.
    pub info: *mut GraphicsOutputModeInformation,
    /// The size of the mode information structure in bytes.
    pub size_of_info: usize,
    /// The physical base address of the linear frame buffer.
    pub frame_buffer_base: u64,
    /// The size of the linear frame buffer in bytes.
    pub frame_buffer_size: usize,
}

/// EFI_GRAPHICS_OUTPUT_BLT_PIXEL. A 32-bit pixel with blue, green, red, and reserved components.
/// (UEFI specification §12.9)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BltPixel {
    /// The blue color component.
    pub blue: u8,
    /// The green color component.
    pub green: u8,
    /// The red color component.
    pub red: u8,
    /// Reserved field; must be zero.
    pub reserved: u8,
}

/// EFI_GRAPHICS_OUTPUT_MODE_INFORMATION. Describes a graphics output mode.
/// (UEFI specification §12.9)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GraphicsOutputModeInformation {
    /// The version of this structure.
    pub version: u32,
    /// The horizontal resolution in pixels.
    pub horizontal_resolution: u32,
    /// The vertical resolution in pixels.
    pub vertical_resolution: u32,
    /// The pixel format of the display.
    pub pixel_format: GopPixelFormat,
    /// Color component bit masks used when `pixel_format` is [`GopPixelFormat::MASK`].
    pub pixel_bitmask: PixelBitmask,
    /// The number of pixels per scan line (pitch of the frame buffer).
    pub pixels_per_scan_line: u32,
}

/// EFI_GRAPHICS_PIXEL_FORMAT. The pixel format of a graphics output mode.
/// (UEFI specification §12.9)
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GopPixelFormat(u32);

impl GopPixelFormat {
    /// Pixel formatted as Red, Green, Blue, Reserved.
    pub const RGB: Self = Self(0);
    /// Pixel formatted as Blue, Green, Red, Reserved.
    pub const BGR: Self = Self(1);
    /// Pixel layout defined by [`PixelBitmask`]; no fixed order.
    pub const MASK: Self = Self(2);
    /// PixelBltOnly; no linear frame buffer, use `Blt()` to draw.
    pub const BLT_ONLY: Self = Self(3);
}

/// EFI_PIXEL_BITMASK. Color component bit masks used when the pixel format is
/// [`GopPixelFormat::MASK`]. (UEFI specification §12.9)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PixelBitmask {
    /// Bit mask for the red color component.
    pub red_mask: u32,
    /// Bit mask for the green color component.
    pub green_mask: u32,
    /// Bit mask for the blue color component.
    pub blue_mask: u32,
    /// Bit mask for the reserved component.
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
    /// Returns a reference to the current graphics output mode.
    pub fn current_mode(&self) -> &GraphicsOutputMode {
        unsafe { &*self.mode }
    }

    /// Returns a reference to the current mode's mode information.
    pub fn current_mode_info(&self) -> &GraphicsOutputModeInformation {
        unsafe { &*self.current_mode().info }
    }

    /// Returns mode information for a given mode number. (UEFI specification §12.9.2.1)
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

    /// Sets the graphics output mode. (UEFI specification §12.9.2.2)
    pub fn set_mode(&mut self, mode_number: u32) -> Result {
        unsafe { (self.set_mode)(self, mode_number) }.into_result()
    }
}
