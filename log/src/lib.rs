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
//! macro invocations become silent no-ops — the cached protocol pointer is
//! cleared and will not be dereferenced.

#![no_std]
#![deny(missing_docs)]

use core::fmt::Write as _;
use core::sync::atomic::{AtomicPtr, Ordering};

use effie::protocols::SerialIo;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

// ── Global state ──────────────────────────────────────────────────────────────

/// Raw pointer to the located `EFI_SERIAL_IO_PROTOCOL` interface.
/// Null when the logger is shut down or no serial device was found.
///
/// Stored with `Release` on write and loaded with `Acquire` on read so that
/// any code that observes a non-null pointer also sees all firmware writes to
/// the protocol table that happened before `init` ran.
static SERIAL_PTR: AtomicPtr<SerialIo> = AtomicPtr::new(core::ptr::null_mut());

// ── Formatting helpers ────────────────────────────────────────────────────────

/// Stack-allocated UTF-8 scratch buffer used inside `Log::log`.
struct Buf<const N: usize> {
    data: [u8; N],
    pos: usize,
}

impl<const N: usize> Buf<N> {
    const fn new() -> Self {
        Self {
            data: [0u8; N],
            pos: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
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

struct SerialLogger;

impl Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // Acquire load pairs with the Release store in `init`. A non-null
        // value guarantees we see all writes the firmware made to the
        // protocol table before `init` stored the pointer.
        let ptr = SERIAL_PTR.load(Ordering::Acquire);
        if ptr.is_null() {
            return;
        }

        let level_str = match record.level() {
            log::Level::Error => "ERROR",
            log::Level::Warn => "WARN ",
            log::Level::Info => "INFO ",
            log::Level::Debug => "DEBUG",
            log::Level::Trace => "TRACE",
        };

        let mut buf = Buf::<512>::new();
        let _ = write!(
            buf,
            "[{}] {}: {}\r\n",
            level_str,
            record.target(),
            record.args(),
        );

        // SAFETY:
        // - `ptr` is non-null (checked above).
        // - `SERIAL_PTR` is set to the firmware's `EFI_SERIAL_IO_PROTOCOL`
        //   pointer in `init` and cleared to null in `shutdown`. Between
        //   those two calls we are in boot services, single-threaded, and
        //   the protocol interface pointer is stable.
        // - We call through the raw pointer instead of creating a `&mut`
        //   reference, because `Log::log` receives `&self` — materialising
        //   `&mut SerialIo` from a shared-access path would violate Rust's
        //   aliasing rules even though UEFI is single-threaded.
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

/// Initialises the serial logger.
///
/// Locates the first `EFI_SERIAL_IO_PROTOCOL` handle via `LocateProtocol` and
/// caches the raw interface pointer. If no serial device is found the logger is
/// still registered — log macro calls will be silent no-ops rather than
/// falling back to the text console.
///
/// `level` sets the initial runtime filter. Change it at any time with
/// [`set_level`].
///
/// # Errors
///
/// Returns [`InitError::AlreadyInitialized`] if a different global logger was
/// already registered.
pub fn init(level: LevelFilter) -> Result<(), InitError> {
    // Locate the serial protocol. If unavailable, SERIAL_PTR stays null and
    // all log calls are quiet no-ops.
    let bs = effie::system_table().boot_services();
    if let Ok(serial) = bs.locate_protocol::<SerialIo>() {
        // `locate_protocol` uses `new_unscoped` — dropping Protocol<SerialIo>
        // is a no-op (no CloseProtocol). The raw pointer stays valid until
        // ExitBootServices (firmware won't uninstall system protocols).
        //
        // Cast `*const` → `*mut`: the protocol is firmware-owned memory that
        // we access exclusively through its own vtable function pointers
        // (which take `*mut Self`). We never create a Rust reference from
        // this pointer; the cast records our intent to call through it.
        let raw = serial.get() as *const SerialIo as *mut SerialIo;
        SERIAL_PTR.store(raw, Ordering::Release);
    }

    log::set_logger(&LOGGER)?;
    log::set_max_level(level);

    Ok(())
}

/// Changes the runtime log level filter.
///
/// Takes effect immediately; no rebuild required.
pub fn set_level(level: LevelFilter) {
    log::set_max_level(level);
}

/// Disables the logger and clears the cached serial pointer.
///
/// Must be called before `ExitBootServices`. After this call all log macro
/// invocations are silent no-ops and the protocol pointer will not be
/// dereferenced.
///
/// # Safety
///
/// The caller must ensure that no log macro calls are in flight on other
/// processors at the time of this call. In practice UEFI is single-threaded
/// before ExitBootServices, so this is trivially safe.
pub unsafe fn shutdown() {
    SERIAL_PTR.store(core::ptr::null_mut(), Ordering::Release);
    log::set_max_level(LevelFilter::Off);
}
