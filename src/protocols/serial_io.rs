use crate::{Guid, HasGuid, HasProtocol, Result, Status};

/// UEFI Serial I/O Protocol. Provides byte-stream communication with UART-style devices.
/// (UEFI specification §12.8.1: EFI_SERIAL_IO_PROTOCOL)
#[repr(C)]
pub struct SerialIo {
    /// Revision of the protocol. See [`SerialIo::REVISION`] and [`SerialIo::REVISION_1_1`].
    pub revision: u32,
    /// Resets the serial device. (§12.8.3.1)
    reset: unsafe extern "efiapi" fn(this: *mut Self) -> Status,
    /// Sets communication parameters (baud rate, parity, data bits, stop bits). (§12.8.3.2)
    set_attributes: unsafe extern "efiapi" fn(
        this: *mut Self,
        baud_rate: u64,
        receive_fifo_depth: u32,
        timeout: u32,
        parity: SerialParity,
        data_bits: u8,
        stop_bits: SerialStopBits,
    ) -> Status,
    /// Asserts or de-asserts hardware control signals. (§12.8.3.3)
    set_control: unsafe extern "efiapi" fn(this: *mut Self, control: u32) -> Status,
    /// Reads the current state of hardware control signals. (§12.8.3.4)
    get_control: unsafe extern "efiapi" fn(this: *mut Self, control: *mut u32) -> Status,
    /// Writes bytes to the serial device. (§12.8.3.5)
    write: unsafe extern "efiapi" fn(
        this: *mut Self,
        buffer_size: *mut usize,
        buffer: *const u8,
    ) -> Status,
    /// Reads bytes from the serial device. (§12.8.3.6)
    read: unsafe extern "efiapi" fn(
        this: *mut Self,
        buffer_size: *mut usize,
        buffer: *mut u8,
    ) -> Status,
    /// Pointer to the current serial mode configuration.
    pub mode: *const SerialIoMode,
    /// Pointer to a GUID identifying the device attached to the port, or null if unknown.
    /// Only valid when `revision >= REVISION_1_1`. (§12.8.2)
    pub device_type_guid: *const Guid,
}

/// Serial mode configuration. Read-only; updated by the firmware. (UEFI specification §12.8.1)
#[repr(C)]
pub struct SerialIoMode {
    /// Bitmask of settable control bits supported by the device.
    pub control_mask: u32,
    /// Per-character timeout in microseconds (applies to both transmit and receive).
    pub timeout: u32,
    /// Current baud rate. Zero means the device runs at its native speed.
    pub baud_rate: u64,
    /// Depth of the receive FIFO.
    pub receive_fifo_depth: u32,
    /// Number of data bits per character.
    pub data_bits: u32,
    /// Parity setting.
    pub parity: SerialParity,
    /// Number of stop bits.
    pub stop_bits: SerialStopBits,
}

/// Parity mode for [`SerialIo::set_attributes`]. (UEFI specification §12.8.1)
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerialParity {
    /// Use the device default parity.
    Default = 0,
    /// No parity bit.
    None = 1,
    /// Even parity.
    Even = 2,
    /// Odd parity.
    Odd = 3,
    /// Mark parity (parity bit always 1).
    Mark = 4,
    /// Space parity (parity bit always 0).
    Space = 5,
}

/// Stop-bit count for [`SerialIo::set_attributes`]. (UEFI specification §12.8.1)
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerialStopBits {
    /// Use the device default stop-bit count.
    Default = 0,
    /// One stop bit.
    One = 1,
    /// One and a half stop bits.
    OneFive = 2,
    /// Two stop bits.
    Two = 3,
}

impl HasGuid for SerialIo {
    const GUID: Guid = Guid::new(
        0xBB25CF6F_u32.to_ne_bytes(),
        0xF1D4_u16.to_ne_bytes(),
        0x11D2_u16.to_ne_bytes(),
        0x9a,
        0x0c,
        [0x00, 0x90, 0x27, 0x3f, 0xc1, 0xfd],
    );
}
impl HasProtocol for SerialIo {}

impl SerialIo {
    /// Original `EFI_SERIAL_IO_PROTOCOL` revision (1.0).
    pub const REVISION: u32 = 0x00010000;
    /// Revision 1.1 — adds [`SerialIo::device_type_guid`].
    pub const REVISION_1_1: u32 = 0x00010001;

    /// Resets the serial device to its default state. (UEFI specification §12.8.3.1)
    pub fn reset(&mut self) -> Result {
        unsafe { (self.reset)(self) }.into_result()
    }

    /// Configures baud rate, FIFO depth, timeout, parity, data bits, and stop bits.
    ///
    /// Pass `0` for any numeric parameter to keep the device's current/default value.
    /// Pass [`SerialParity::Default`] / [`SerialStopBits::Default`] likewise.
    /// (UEFI specification §12.8.3.2)
    pub fn set_attributes(
        &mut self,
        baud_rate: u64,
        receive_fifo_depth: u32,
        timeout: u32,
        parity: SerialParity,
        data_bits: u8,
        stop_bits: SerialStopBits,
    ) -> Result {
        unsafe {
            (self.set_attributes)(
                self,
                baud_rate,
                receive_fifo_depth,
                timeout,
                parity,
                data_bits,
                stop_bits,
            )
        }
        .into_result()
    }

    /// Writes `buf` to the serial device. Returns the number of bytes actually written.
    ///
    /// Transmission stops early on timeout; the returned count may be less than
    /// `buf.len()` in that case. (UEFI specification §12.8.3.5)
    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut size = buf.len();
        unsafe { (self.write)(self, &mut size, buf.as_ptr()) }
            .into_result()
            .map(|_| size)
    }

    /// Writes `buf` to the serial device through a raw pointer.
    ///
    /// Identical to [`write`] but takes `*mut Self` instead of `&mut self`,
    /// so callers holding only a raw pointer (e.g. a logger that stores the
    /// protocol pointer in an atomic) can call through without first creating
    /// a Rust reference, which would violate aliasing rules when the pointer
    /// was obtained via a shared-access path.
    ///
    /// # Safety
    ///
    /// `this` must be a valid, non-null pointer to a live `EFI_SERIAL_IO_PROTOCOL`
    /// interface. No other live Rust `&mut SerialIo` may exist for the duration
    /// of the call.
    ///
    /// [`write`]: SerialIo::write
    pub unsafe fn write_raw(this: *mut Self, buf: &[u8]) -> Result<usize> {
        let mut size = buf.len();
        unsafe { ((*this).write)(this, &mut size, buf.as_ptr()) }
            .into_result()
            .map(|_| size)
    }

    /// Reads up to `buf.len()` bytes from the serial device. Returns the number of
    /// bytes actually read.
    ///
    /// Stops early on timeout or overrun. (UEFI specification §12.8.3.6)
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut size = buf.len();
        unsafe { (self.read)(self, &mut size, buf.as_mut_ptr()) }
            .into_result()
            .map(|_| size)
    }
}
