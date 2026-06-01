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
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};

mod allocator;
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

pub use effie_macros::w;

use tables::SystemTable;

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
        system_table_raw()
            .expect("system_table() called before efi_main")
            .as_ref()
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
    image_handle_raw().expect("image_handle() called before efi_main")
}
