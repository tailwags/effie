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

use core::mem::MaybeUninit;

mod allocator;
mod protocol;
mod status;
mod types;
mod wstr;

pub mod protocols;
pub mod tables;

pub use allocator::Allocator;
pub use protocol::{HasGuid, HasProtocol, Protocol};
pub use status::{Result, Status};
pub use types::*;
pub use wstr::{CharIndices, Chars, WStr, WString};

pub use uguid::Guid;

pub use effie_macros::w;

use tables::SystemTable;

static mut SYSTEM_TABLE: MaybeUninit<&SystemTable> = MaybeUninit::uninit();
static mut IMAGE_HANDLE: MaybeUninit<Handle> = MaybeUninit::uninit();

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

#[unsafe(no_mangle)]
extern "efiapi" fn efi_main(image_handle: Handle, system_table: &'static SystemTable) -> Status {
    unsafe extern "Rust" {
        fn main() -> Result;
    }

    #[allow(clippy::deref_addrof)]
    unsafe {
        (*&raw mut SYSTEM_TABLE).write(system_table);
        (*&raw mut IMAGE_HANDLE).write(image_handle);
    };

    unsafe {
        if let Err(status) = main() {
            if let Ok(mut con_out) = system_table.con_out() {
                let _ = con_out.output_line(status.description());
            }

            status
        } else {
            Status::SUCCESS
        }
    }
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
    unsafe { SYSTEM_TABLE.assume_init() }
}

/// Returns the image handle that was passed to `efi_main`.
///
/// This handle identifies the currently executing UEFI image.
///
/// # Panics
///
/// Panics if called before `efi_main` has written the handle.
pub fn image_handle() -> Handle {
    unsafe { IMAGE_HANDLE.assume_init() }
}
