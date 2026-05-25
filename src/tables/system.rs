use core::ffi::c_void;

use crate::{
    Guid, Handle, Protocol, Result, WStr,
    protocols::{SimpleTextInput, SimpleTextOutput},
    tables::{BootServices, RuntimeServices, TableHeader},
};

use super::{Signature, SpecificationRevision};

/// Contains pointers to the runtime and boot services tables. (UEFI specification §4.3.1:
/// EFI_SYSTEM_TABLE). The system table is the entry point for all UEFI firmware interaction.
#[repr(C)]
pub struct SystemTable {
    /// Standard EFI table header.
    hdr: TableHeader,
    /// Null-terminated string identifying the firmware vendor.
    firmware_vendor: *const u16,
    /// Firmware revision level.
    firmware_version: u32,
    /// Handle for the console input device.
    console_in_handle: Handle,
    /// Simple Text Input Protocol interface for the active console input.
    con_in: *mut SimpleTextInput,
    /// Handle for the console output device.
    console_out_handle: Handle,
    /// Simple Text Output Protocol interface for the active console output.
    con_out: *mut SimpleTextOutput,
    /// Handle for the standard error device.
    standard_error_handler: Handle,
    /// Simple Text Output Protocol interface for the standard error device.
    std_err: *mut SimpleTextOutput,
    /// Runtime Services Table pointer.
    runtime_services: *mut RuntimeServices,
    /// Boot Services Table pointer.
    boot_services: *mut BootServices,
    /// Number of entries in the configuration table array.
    number_of_table_entries: usize,
    /// Pointer to the array of configuration table entries.
    configuration_table: *mut ConfigurationTable,
}

/// An entry in the system table's configuration table array. Each entry associates a vendor GUID
/// with a firmware-provided configuration table. (UEFI specification §4.3)
#[repr(C)]
pub struct ConfigurationTable {
    /// The GUID that identifies this configuration table entry.
    vendor_guid: Guid,
    /// Pointer to the firmware-provided configuration table data.
    vendor_table: *mut c_void,
}

impl SystemTable {
    /// Returns the table signature. Must match [`Signature::SYSTEM_TABLE`].
    pub fn signature(&self) -> Signature {
        self.hdr.signature
    }

    /// Returns the revision of the UEFI Specification to which this table conforms.
    pub fn revision(&self) -> SpecificationRevision {
        self.hdr.revision
    }

    /// Returns a null-terminated string that identifies the vendor that produces the system
    /// firmware for the platform.
    pub fn firmware_vendor(&self) -> &WStr {
        unsafe { WStr::from_ptr(self.firmware_vendor) }
    }

    /// Returns the `EFI_SIMPLE_TEXT_INPUT_PROTOCOL` interface for the active console input device.
    pub fn con_in(&self) -> Result<Protocol<SimpleTextInput>> {
        Protocol::new_unscoped(self.con_in)
    }

    /// Returns the `EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL` interface for the active console output device.
    pub fn con_out(&self) -> Result<Protocol<SimpleTextOutput>> {
        Protocol::new_unscoped(self.con_out)
    }

    /// Returns a reference to the EFI Boot Services Table.
    /// Panics via debug_assert if boot services have been exited.
    pub fn boot_services(&self) -> &BootServices {
        debug_assert!(
            !self.boot_services.is_null(),
            "boot services unavailable (ExitBootServices already called?)"
        );
        unsafe { &*self.boot_services }
    }

    /// Returns a reference to the EFI Runtime Services Table.
    pub fn runtime_services(&self) -> &RuntimeServices {
        debug_assert!(
            !self.runtime_services.is_null(),
            "runtime services pointer is null"
        );
        unsafe { &*self.runtime_services }
    }

    /// Returns a slice of configuration table entries. Each entry pairs a vendor GUID with a
    /// firmware-specific table pointer.
    pub fn configuration_tables(&self) -> &[ConfigurationTable] {
        unsafe {
            core::slice::from_raw_parts(self.configuration_table, self.number_of_table_entries)
        }
    }
}
