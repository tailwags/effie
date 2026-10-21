//! [`log`] implementation for UEFI applications built on [`effie`].
//!
//! Every record becomes one line, e.g.
//! `[  0.412] [INFO ] bread::esp: verified \EFI\bread\kernels\bzImage`, and
//! goes to the first `EFI_SERIAL_IO_PROTOCOL` device (QEMU shows it with
//! `-serial stdio`) and, if enabled, to an in-memory history that a panic
//! handler can save. Nothing is written to the screen.
//!
//! # Usage
//!
//! ```ignore
//! #[unsafe(no_mangle)]
//! fn main() -> effie::Result {
//!     effie_log::builder()
//!         .max_level(log::LevelFilter::Trace)
//!         .uptime(true)
//!         .history(32 * 1024)
//!         .init()
//!         .ok();
//!     log::info!("starting");
//!     // ...
//!     effie_log::detach();
//!     // exit_boot_services here
//! }
//! ```
//!
//! # ExitBootServices
//!
//! Call [`detach`] **before** `ExitBootServices`. It stops all firmware calls;
//! records still reach the history, which needs no firmware.
//!
//! # Panics
//!
//! A `#[panic_handler]` should start with [`report_panic`], which logs the
//! panic like any other record, then save the output from [`take_history`].

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

mod clock;
mod history;

use core::fmt::{self, Write as _};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use effie::protocols::SerialIo;
use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};

/// The located `EFI_SERIAL_IO_PROTOCOL` interface; null before [`Builder::init`],
/// after [`detach`], or when there is no serial device.
static SERIAL: AtomicPtr<SerialIo> = AtomicPtr::new(core::ptr::null_mut());
/// Whether lines start with the time since [`Builder::init`].
static UPTIME: AtomicBool = AtomicBool::new(false);
/// Set by [`report_panic`], to tell a nested panic from the first.
static PANICKING: AtomicBool = AtomicBool::new(false);

/// Stack-allocated UTF-8 string buffer.
///
/// Implements [`core::fmt::Write`] with saturating overflow: once a write does
/// not fit, it is cut at the last whole character and everything after it is
/// dropped silently. The contents are therefore always valid UTF-8.
pub struct Buf<const N: usize> {
    data: [u8; N],
    pos: usize,
    full: bool,
}

impl<const N: usize> Buf<N> {
    /// Creates an empty buffer.
    pub const fn new() -> Self {
        Self {
            data: [0u8; N],
            pos: 0,
            full: false,
        }
    }

    /// Returns the bytes written so far.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.pos]
    }

    /// Returns the text written so far.
    pub fn as_str(&self) -> &str {
        // SAFETY: `write_str` only ever copies whole characters of a `&str`.
        unsafe { core::str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl<const N: usize> Default for Buf<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Write for Buf<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.full {
            return Ok(());
        }
        let n = s.floor_char_boundary(N - self.pos);
        self.data[self.pos..self.pos + n].copy_from_slice(&s.as_bytes()[..n]);
        self.pos += n;
        self.full = n < s.len();
        Ok(())
    }
}

/// Configures the logger; created by [`builder`].
#[must_use = "the logger does nothing until `init` is called"]
pub struct Builder {
    max_level: LevelFilter,
    uptime: bool,
    history: usize,
}

/// Starts configuring the logger.
pub fn builder() -> Builder {
    Builder {
        max_level: LevelFilter::Info,
        uptime: false,
        history: 0,
    }
}

impl Builder {
    /// Sets the most verbose level that is logged. Defaults to `Info`.
    pub fn max_level(mut self, level: LevelFilter) -> Self {
        self.max_level = level;
        self
    }

    /// Starts every line with the seconds since [`init`](Self::init), e.g.
    /// `[  0.412] `. Off by default.
    ///
    /// The clock is a CPU counter, so it keeps working after
    /// `ExitBootServices`. On x86_64, `init` spends 10 ms calibrating it.
    pub fn uptime(mut self, on: bool) -> Self {
        self.uptime = on;
        self
    }

    /// Keeps the last `bytes` bytes of output in memory, for [`take_history`].
    /// Off (0) by default.
    pub fn history(mut self, bytes: usize) -> Self {
        self.history = bytes;
        self
    }

    /// Locates the serial device and registers the logger. If there is no
    /// serial device, records only reach the history.
    ///
    /// # Errors
    ///
    /// Returns [`InitError::AlreadyInitialized`] if a global logger was
    /// already registered.
    pub fn init(self) -> Result<(), InitError> {
        log::set_logger(&Logger)?;

        let boot_services = effie::system_table().boot_services();
        if let Ok(serial) = boot_services.locate_protocol::<SerialIo>() {
            let raw = serial.get() as *const SerialIo as *mut SerialIo;
            SERIAL.store(raw, Ordering::Release);
        }
        if self.uptime {
            clock::start();
            UPTIME.store(true, Ordering::Relaxed);
        }
        if self.history > 0 {
            history::init(self.history);
        }

        log::set_max_level(self.max_level);
        Ok(())
    }
}

/// Error returned by [`Builder::init`].
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

/// Stops writing to the serial device. Records still reach the history.
///
/// Call before `ExitBootServices`: logging then makes no firmware calls, so it
/// cannot change the memory map or touch a protocol that is gone.
pub fn detach() {
    SERIAL.store(core::ptr::null_mut(), Ordering::Release);
}

/// Logs a panic as an `ERROR` record with target `panic`, regardless of the
/// level filter.
///
/// Returns `false`, without logging, if a panic was already reported: the
/// panic handler itself panicked.
pub fn report_panic(info: &PanicInfo<'_>) -> bool {
    if PANICKING.swap(true, Ordering::Relaxed) {
        return false;
    }
    let message = info.message();
    match info.location() {
        Some(at) => emit(
            Level::Error,
            "panic",
            format_args!("{}:{}:{}: {message}", at.file(), at.line(), at.column()),
        ),
        None => emit(Level::Error, "panic", format_args!("{message}")),
    }
    true
}

/// Ends the history and returns its output as `(older, newer)` byte slices,
/// starting at a whole line. Later records only go to the serial device.
///
/// Both slices are empty if history is off or was already taken.
pub fn take_history() -> (&'static [u8], &'static [u8]) {
    history::take()
}

/// Returns a copy of the history so far, starting at a whole line, and keeps
/// recording. Empty if history is off or was already taken.
pub fn copy_history() -> alloc::vec::Vec<u8> {
    history::copy()
}

struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record<'_>) {
        emit(record.level(), record.target(), *record.args());
    }

    fn flush(&self) {}
}

/// Formats one line and writes it to the serial device and the history.
fn emit(level: Level, target: &str, args: fmt::Arguments<'_>) {
    let mut buf = Buf::<512>::new();
    if UPTIME.load(Ordering::Relaxed) {
        let _ = match clock::elapsed_ms() {
            Some(ms) => write!(buf, "[{:3}.{:03}] ", ms / 1000, ms % 1000),
            None => write!(buf, "[   ?.???] "),
        };
    }
    let label = match level {
        Level::Error => "ERROR",
        Level::Warn => "WARN ",
        Level::Info => "INFO ",
        Level::Debug => "DEBUG",
        Level::Trace => "TRACE",
    };
    let _ = write!(buf, "[{label}] {target}: {args}");

    // The line end is written apart so a cut-off line still ends.
    let serial = SERIAL.load(Ordering::Acquire);
    for bytes in [buf.as_bytes(), b"\r\n"] {
        if !serial.is_null() {
            // SAFETY: the pointer came from `LocateProtocol` and stays valid
            // until `detach`, which runs before ExitBootServices. Called through
            // the raw pointer to avoid creating a `&mut` to firmware-owned memory.
            let _ = unsafe { SerialIo::write_raw(serial, bytes) };
        }
        history::push(bytes);
    }
}
