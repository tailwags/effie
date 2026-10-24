#![no_std]
#![deny(missing_docs)]

//! **effie** -- idiomatic UEFI API in Rust.
//!
//! A `#![no_std]` wrapper around UEFI firmware interfaces that closely maps to
//! the UEFI specification rather than abstracting it away. Every protocol, table,
//! and status code corresponds directly to its UEFI counterpart.
//!
//! # Entry point
//!
//! effie owns the `efi_main` entry symbol. Your application provides a
//! `#[unsafe(no_mangle)] fn main() -> Result`:
//!
//! ```ignore
//! #![no_main]
//! #![no_std]
//! extern crate alloc;
//!
//! #[unsafe(no_mangle)]
//! fn main() -> effie::Result {
//!     let mut con_out = effie::system_table().con_out()?;
//!     con_out.output_line(effie::w!("Hello, UEFI!"))
//! }
//! ```
//!
//! # Key concepts
//!
//! | Concept | UEFI spec | effie |
//! |---|---|---|
//! | System Table | `EFI_SYSTEM_TABLE` | [`tables::SystemTable`] |
//! | Boot Services | `EFI_BOOT_SERVICES` | [`tables::BootServices`] |
//! | Protocols | various GUIDs | [`Protocol<P>`] + per-protocol structs |
//! | Status codes | `EFI_STATUS` | [`Status`] |
//! | UTF-16 strings | `CHAR16*` | [`WStr`] / [`WString`] |
//!
//! # Cargo features
//!
//! No features are currently defined.

extern crate alloc;

use core::{
    ffi::c_void,
    mem::ManuallyDrop,
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

mod allocator;
mod bits;
mod protocol;
mod status;
mod types;
mod wstr;

pub mod guid;
pub mod log;
pub mod protocols;
pub mod tables;

pub use allocator::Allocator;
pub use guid::Guid;
pub use protocol::{HasGuid, HasProtocol, Protocol};
pub use status::{Result, Status};
pub use types::*;
pub use wstr::{CharIndices, Chars, WStr, WString};

#[doc(hidden)]
pub use effie_macros::wstr_units as __wstr_units;

/// Constructs a statically allocated [`WStr`] from a UTF-8 string literal.
///
/// The literal is encoded as UTF-16 at compile time and terminated with a null code unit.
/// Interior null characters are rejected at compile time.
///
/// # Examples
///
/// ```ignore
/// let hello: &'static effie::WStr = effie::w!("Hello, UEFI!");
/// ```
#[macro_export]
macro_rules! w {
    ($literal:literal) => {{
        const __EFFIE_WSTR: &$crate::WStr =
            unsafe { $crate::WStr::from_slice_unchecked($crate::__wstr_units!($literal)) };
        __EFFIE_WSTR
    }};
}

use tables::{MemoryMap, SystemTable};

static SYSTEM_TABLE: AtomicPtr<SystemTable> = AtomicPtr::new(ptr::null_mut());
static IMAGE_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

#[unsafe(no_mangle)]
extern "efiapi" fn efi_main(image_handle: Handle, raw_system_table: *mut SystemTable) -> Status {
    unsafe extern "Rust" {
        fn main() -> Result;
    }

    SYSTEM_TABLE.store(raw_system_table, Ordering::Release);
    IMAGE_HANDLE.store(image_handle.as_ptr(), Ordering::Release);

    // Safety: we just stored a valid pointer above.

    unsafe {
        if let Err(status) = main() {
            log::error!("main() returned Err({status})");

            if let Ok(mut con_out) = system_table().con_out() {
                let _ = con_out.output_line(status.description());
            }

            status
        } else {
            log::warn!("main() returned Ok");

            Status::SUCCESS
        }
    }
}

/// Returns the UEFI System Table pointer, or `None` if called before
/// `efi_main` has initialised the global (e.g. during static initialisation).
///
/// The returned [`NonNull`] pointer is valid for the lifetime of the UEFI
/// session. Prefer [`system_table`] for normal use.
pub fn system_table_raw() -> Option<NonNull<SystemTable>> {
    NonNull::new(SYSTEM_TABLE.load(Ordering::Acquire))
}

/// Returns the UEFI System Table.
///
/// The system table contains pointers to boot services, runtime services,
/// console handles, and the configuration table.
///
/// # Panics
///
/// Panics if called before `efi_main` has written the pointer (i.e. during
/// static initialisation).
pub fn system_table() -> &'static SystemTable {
    // Safety: the pointer was stored by efi_main from a firmware-provided
    // `*mut SystemTable` that is valid for the lifetime of the UEFI session.
    unsafe {
        match system_table_raw() {
            Some(table) => table.as_ref(),
            None => not_initialized(),
        }
    }
}

/// Returns the image handle, or `None` if called before `efi_main` has
/// initialised the global.
///
/// Prefer [`image_handle`] for normal use.
pub fn image_handle_raw() -> Option<Handle> {
    let ptr = IMAGE_HANDLE.load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        // Safety: the pointer was stored by efi_main from a firmware-provided
        // image handle that is valid for the lifetime of the UEFI session.
        Some(unsafe { Handle::from_raw(ptr) })
    }
}

/// Returns the image handle that was passed to `efi_main`.
///
/// This handle identifies the currently executing UEFI image.
///
/// # Panics
///
/// Panics if called before `efi_main` has written the handle.
pub fn image_handle() -> Handle {
    match image_handle_raw() {
        Some(handle) => handle,
        None => not_initialized(),
    }
}

/// Shared by [`system_table`] and [`image_handle`], so that each of their many
/// inlined call sites only needs a branch to it.
#[cold]
#[inline(never)]
fn not_initialized() -> ! {
    panic!("effie used before efi_main")
}

/// Exits boot services on behalf of the running image and returns the final memory map.
///
/// `ExitBootServices` fails with `EFI_INVALID_PARAMETER` when the memory map changed
/// after it was fetched; the map is then fetched again and the call retried a few times.
/// (UEFI specification §7.4.6)
///
/// The map's buffer comes from the pool allocator, which no longer exists once this
/// returns `Ok`, so it is wrapped in [`ManuallyDrop`] and must never be dropped.
///
/// # Safety
///
/// On success every boot service is gone: no heap allocation, no dropping heap-backed
/// values, no protocol calls. Shut down anything that logs through boot services (such
/// as `effie-log`) before calling this, since logging would change the memory map.
pub unsafe fn exit_boot_services() -> Result<ManuallyDrop<MemoryMap>> {
    const ATTEMPTS: usize = 4;

    let boot_services = system_table().boot_services();
    let mut last_error = Status::INVALID_PARAMETER;

    for _ in 0..ATTEMPTS {
        let map = boot_services.get_memory_map()?;
        match unsafe { boot_services.exit_boot_services(image_handle(), map.map_key) } {
            Ok(()) => return Ok(ManuallyDrop::new(map)),
            Err(status) => last_error = status,
        }
        if last_error != Status::INVALID_PARAMETER {
            break;
        }
    }

    Err(last_error)
}
