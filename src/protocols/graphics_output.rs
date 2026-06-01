use crate::{Guid, HasGuid, HasProtocol, Result, Status, log};

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
    pub info: *const GraphicsOutputModeInformation,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct BltPixel {
    /// The blue color component. Intensity ranges from 0 (minimum) to 255 (maximum).
    pub blue: u8,
    /// The green color component. Intensity ranges from 0 (minimum) to 255 (maximum).
    pub green: u8,
    /// The red color component. Intensity ranges from 0 (minimum) to 255 (maximum).
    pub red: u8,
    /// Reserved field; must be zero.
    pub reserved: u8,
}

impl BltPixel {
    /// Creates a new [`BltPixel`] from red, green, and blue components.
    /// The reserved byte is set to zero.
    #[inline]
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            reserved: 0,
        }
    }
}

/// EFI_GRAPHICS_OUTPUT_BLT_OPERATION. Identifies the blt operation to perform.
/// (UEFI specification §12.9.2.3)
///
/// This type is a `#[repr(transparent)]` newtype over `u32` rather than a C-style enum so that
/// unknown values returned by future firmware revisions can be represented without triggering
/// undefined behaviour.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BltOperation(u32);

impl BltOperation {
    /// `EfiBltVideoFill` (0). Fill a video rectangle with the colour from
    /// `BltBuffer[0]`. Source coordinates and `Delta` are not used.
    pub const VIDEO_FILL: Self = Self(0);

    /// `EfiBltVideoToBltBuffer` (1). Read pixels from a video rectangle into a
    /// CPU-side buffer. The destination coordinates address the buffer, not the
    /// screen. `Delta` must be set to the byte stride of the buffer when
    /// `DestinationX` or `DestinationY` is non-zero.
    pub const VIDEO_TO_BUFFER: Self = Self(1);

    /// `EfiBltBufferToVideo` (2). Write pixels from a CPU-side buffer to a video
    /// rectangle. The source coordinates address the buffer. `Delta` must be set
    /// to the byte stride of the buffer when `SourceX` or `SourceY` is non-zero.
    pub const BUFFER_TO_VIDEO: Self = Self(2);

    /// `EfiBltVideoToVideo` (3). Copy a rectangle within the video framebuffer.
    /// `BltBuffer` and `Delta` are not used. Overlapping source and destination
    /// regions are permitted.
    pub const VIDEO_TO_VIDEO: Self = Self(3);
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

    /// Returns mode information for the given mode number. (UEFI specification §12.9.2.1)

    pub fn query_mode(&self, mode_number: u32) -> Result<GraphicsOutputModeInformation> {
        let mut size: usize = 0;
        let mut info: *const GraphicsOutputModeInformation = core::ptr::null();
        let status = unsafe { (self.query_mode)(self, mode_number, &mut size, &mut info) };
        if let Err(err) = status.into_result() {
            log::debug!("GraphicsOutput::query_mode: mode {mode_number} returned {err}");
            return Err(err);
        }
        // The spec says the size of Info must never be assumed; verify the firmware
        // returned at least enough bytes to cover the structure we will copy.
        assert!(
            size >= core::mem::size_of::<GraphicsOutputModeInformation>(),
            "firmware returned a QueryMode Info buffer smaller than GraphicsOutputModeInformation"
        );

        Ok(unsafe { info.read() })
    }

    /// Sets the graphics output mode. (UEFI specification §12.9.2.2)
    pub fn set_mode(&mut self, mode_number: u32) -> Result {
        let status = unsafe { (self.set_mode)(self, mode_number) }.into_result();
        if let Err(err) = status {
            log::warn!("GraphicsOutput::set_mode: mode {mode_number} returned {err}");
            return Err(err);
        }
        Ok(())
    }

    /// `EfiBltVideoFill` — fill a rectangle on the framebuffer with a single colour.
    /// (UEFI specification §12.9.2.3)
    ///
    /// Writes `pixel` to every pixel in the video rectangle whose upper-left corner
    /// is `(dest_x, dest_y)` and whose size is `width × height`. The source
    /// coordinates and `Delta` parameter are not used for this operation.
    pub fn blt_video_fill(
        &mut self,
        pixel: BltPixel,
        dest_x: usize,
        dest_y: usize,
        width: usize,
        height: usize,
    ) -> Result {
        // The spec requires a pointer to *at least one* BltPixel; we pass a stack copy.
        let mut px = pixel;
        let status = unsafe {
            (self.blt)(
                self,
                &mut px,
                BltOperation::VIDEO_FILL.0,
                0, // SourceX — not used
                0, // SourceY — not used
                dest_x,
                dest_y,
                width,
                height,
                0, // Delta — not used
            )
        };
        status.into_result()
    }

    /// `EfiBltVideoToBltBuffer` — read a rectangle from the framebuffer into a CPU buffer.
    /// (UEFI specification §12.9.2.3)
    ///
    /// Copies the `width × height` video rectangle whose upper-left corner is
    /// `(src_x, src_y)` into `buffer` starting at offset `(dest_x, dest_y)` within
    /// the buffer.
    ///
    /// # `buffer_width`
    ///
    /// `buffer_width` is the number of pixels per row in `buffer` (its logical stride).
    /// Pass `0` to indicate that the buffer is exactly `width` pixels wide and the
    /// pixels to write begin at `buffer[0]` — i.e. `dest_x == 0 && dest_y == 0`.
    /// When `dest_x != 0` or `dest_y != 0` you **must** supply the true row width so
    /// that the firmware can compute the correct byte offset.
    ///
    /// `buffer` must contain at least `(dest_y + height - 1) * stride + dest_x + width`
    /// pixels (where `stride = buffer_width` when non-zero, otherwise `width`).
    pub fn blt_video_to_buffer(
        &mut self,
        buffer: &mut [BltPixel],
        src_x: usize,
        src_y: usize,
        dest_x: usize,
        dest_y: usize,
        width: usize,
        height: usize,
        buffer_width: usize,
    ) -> Result {
        // Delta = byte length of one row in the buffer; 0 means "use the whole buffer".
        let delta = if buffer_width == 0 {
            0
        } else {
            buffer_width * core::mem::size_of::<BltPixel>()
        };
        let status = unsafe {
            (self.blt)(
                self,
                buffer.as_mut_ptr(),
                BltOperation::VIDEO_TO_BUFFER.0,
                src_x,
                src_y,
                dest_x,
                dest_y,
                width,
                height,
                delta,
            )
        };
        status.into_result()
    }

    /// `EfiBltBufferToVideo` — write a CPU buffer into a rectangle on the framebuffer.
    /// (UEFI specification §12.9.2.3)
    ///
    /// Copies the `width × height` rectangle starting at `(src_x, src_y)` within
    /// `buffer` to the video rectangle whose upper-left corner is `(dest_x, dest_y)`.
    ///
    /// # `buffer_width`
    ///
    /// `buffer_width` is the number of pixels per row in `buffer` (its logical stride).
    /// Pass `0` to indicate that the buffer is exactly `width` pixels wide and the
    /// read begins at `buffer[0]` — i.e. `src_x == 0 && src_y == 0`. When
    /// `src_x != 0` or `src_y != 0` you **must** supply the true row width so that
    /// the firmware can compute the correct byte offset.
    ///
    /// `buffer` must contain at least `(src_y + height - 1) * stride + src_x + width`
    /// pixels (where `stride = buffer_width` when non-zero, otherwise `width`).
    pub fn blt_buffer_to_video(
        &mut self,
        buffer: &[BltPixel],
        src_x: usize,
        src_y: usize,
        dest_x: usize,
        dest_y: usize,
        width: usize,
        height: usize,
        buffer_width: usize,
    ) -> Result {
        let delta = if buffer_width == 0 {
            0
        } else {
            buffer_width * core::mem::size_of::<BltPixel>()
        };
        let status = unsafe {
            (self.blt)(
                self,
                // The UEFI ABI takes *mut BltPixel even for a read-only buffer;
                // we promise the firmware will only read from it for this operation.
                buffer.as_ptr().cast_mut(),
                BltOperation::BUFFER_TO_VIDEO.0,
                src_x,
                src_y,
                dest_x,
                dest_y,
                width,
                height,
                delta,
            )
        };
        status.into_result()
    }

    /// `EfiBltVideoToVideo` — copy a rectangle within the framebuffer.
    /// (UEFI specification §12.9.2.3)
    ///
    /// Copies the `width × height` video rectangle whose upper-left corner is
    /// `(src_x, src_y)` to the video rectangle whose upper-left corner is
    /// `(dest_x, dest_y)`. Overlapping source and destination regions are permitted.
    /// Neither a CPU buffer nor `Delta` are used for this operation.
    pub fn blt_video_to_video(
        &mut self,
        src_x: usize,
        src_y: usize,
        dest_x: usize,
        dest_y: usize,
        width: usize,
        height: usize,
    ) -> Result {
        let status = unsafe {
            (self.blt)(
                self,
                core::ptr::null_mut(), // BltBuffer — not used
                BltOperation::VIDEO_TO_VIDEO.0,
                src_x,
                src_y,
                dest_x,
                dest_y,
                width,
                height,
                0, // Delta — not used
            )
        };
        status.into_result()
    }
}
