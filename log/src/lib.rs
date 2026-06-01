//! Serial-backed [`log`] implementation for UEFI applications built on [`effie`].
//!
//! Locates the first `EFI_SERIAL_IO_PROTOCOL` handle at initialisation time and
//! routes all [`log`] macro output to it as UTF-8 bytes. QEMU captures that
//! output on `-serial stdio` or `-serial file:debug.log` without touching the
//! framebuffer or UEFI text console.
//!
//! # Usage
//!
//! ```ignore
//! #[unsafe(no_mangle)]
//! fn main() -> effie::Result {
//!     effie_log::init(log::LevelFilter::Trace).ok();
//!     log::info!("bread starting");
//!     // ...
//!     unsafe { effie_log::shutdown(); }
//!     // exit_boot_services here
//! }
//! ```
//!
//! # Shutdown
//!
//! Call [`shutdown`] **before** `ExitBootServices`. After that call all log
//! macro invocations are silent no-ops.

#![no_std]
#![deny(missing_docs)]

use core::fmt::Write as _;
use core::sync::atomic::{AtomicPtr, Ordering};

use effie::protocols::SerialIo;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

/// Raw pointer to the located `EFI_SERIAL_IO_PROTOCOL` interface.
/// Null when the logger is shut down or no serial device was found.
static SERIAL_PTR: AtomicPtr<SerialIo> = AtomicPtr::new(core::ptr::null_mut());

/// Stack-allocated UTF-8 scratch buffer.
///
/// Implements [`core::fmt::Write`] with saturating overflow -- once full,
/// additional bytes are dropped silently. Use [`Buf::as_bytes`] to read the
/// filled portion.
///
/// `pub` so callers that supply their own [`log::Log`] can reuse it.
pub struct Buf<const N: usize> {
    data: [u8; N],
    pos: usize,
}

impl<const N: usize> Buf<N> {
    /// Creates an empty buffer.
    pub const fn new() -> Self {
        Self {
            data: [0u8; N],
            pos: 0,
        }
    }

    /// Returns the bytes written so far.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.pos]
    }
}

impl<const N: usize> core::fmt::Write for Buf<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.data.len() - self.pos;
        let n = bytes.len().min(remaining);
        self.data[self.pos..self.pos + n].copy_from_slice(&bytes[..n]);
        self.pos += n;
        Ok(())
    }
}

/// Maps a [`log::Level`] to a fixed-width five-character label for log line alignment.
pub fn level_str(level: log::Level) -> &'static str {
    match level {
        log::Level::Error => "ERROR",
        log::Level::Warn => "WARN ",
        log::Level::Info => "INFO ",
        log::Level::Debug => "DEBUG",
        log::Level::Trace => "TRACE",
    }
}

struct SerialLogger;

impl Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let ptr = SERIAL_PTR.load(Ordering::Acquire);
        if ptr.is_null() {
            return;
        }

        let mut buf = Buf::<512>::new();
        let _ = write!(
            buf,
            "[{}] {}: {}\r\n",
            level_str(record.level()),
            record.target(),
            record.args(),
        );

        // SAFETY: ptr is non-null firmware-owned memory, stable between init and shutdown.
        // Called through the raw pointer rather than &mut to avoid aliasing a shared-access path.
        let _ = unsafe { SerialIo::write_raw(ptr, buf.as_bytes()) };
    }

    fn flush(&self) {}
}

static LOGGER: SerialLogger = SerialLogger;

/// Error returned by [`init`].
#[derive(Debug)]
pub enum InitError {
    /// A global logger was already registered via [`log::set_logger`].
    AlreadyInitialized(SetLoggerError),
}

impl From<SetLoggerError> for InitError {
    fn from(e: SetLoggerError) -> Self {
        Self::AlreadyInitialized(e)
    }
}

/// Locates `EFI_SERIAL_IO_PROTOCOL` and caches its pointer.
///
/// This is the serial-location half of [`init`]. Call this when supplying a
/// custom logger but still wanting serial output via [`write_serial_raw`].
/// Safe to call multiple times; each call overwrites the cached pointer.
pub fn locate_serial() {
    let bs = effie::system_table().boot_services();
    if let Ok(serial) = bs.locate_protocol::<SerialIo>() {
        let raw = serial.get() as *const SerialIo as *mut SerialIo;
        SERIAL_PTR.store(raw, Ordering::Release);
    }
}

/// Writes raw bytes directly to the cached serial device.
///
/// Low-level escape hatch for panic handlers and other code that cannot use
/// the `log` facade. Bypasses all formatting and level filtering. No-ops if
/// no serial device was located or after [`shutdown`].
pub fn write_serial_raw(buf: &[u8]) {
    let ptr = SERIAL_PTR.load(Ordering::Acquire);
    if ptr.is_null() {
        return;
    }
    // SAFETY: same as in SerialLogger::log.
    let _ = unsafe { SerialIo::write_raw(ptr, buf) };
}

/// Initialises the serial logger.
///
/// Locates the first `EFI_SERIAL_IO_PROTOCOL` via `LocateProtocol` and caches
/// the raw interface pointer. If no serial device is found, log calls are
/// silent no-ops. `level` sets the initial runtime filter.
///
/// # Errors
///
/// Returns [`InitError::AlreadyInitialized`] if a different global logger was
/// already registered.
pub fn init(level: LevelFilter) -> Result<(), InitError> {
    locate_serial();
    log::set_logger(&LOGGER)?;
    log::set_max_level(level);
    Ok(())
}

/// Changes the runtime log level filter.
pub fn set_level(level: LevelFilter) {
    log::set_max_level(level);
}

/// Disables the logger and clears the cached serial pointer.
///
/// Must be called before `ExitBootServices`.
///
/// # Safety
///
/// No log calls may be in flight on other processors at the time of this call.
/// In practice UEFI is single-threaded before ExitBootServices.
pub unsafe fn shutdown() {
    SERIAL_PTR.store(core::ptr::null_mut(), Ordering::Release);
    log::set_max_level(LevelFilter::Off);
}
