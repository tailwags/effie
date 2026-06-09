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
    pub vendor_guid: Guid,
    /// Pointer to the firmware-provided configuration table data.
    pub vendor_table: *mut c_void,
}

// ── Well-known configuration table GUIDs ──────────────────────────────────────

/// ACPI 2.0 RSDP table GUID. Present on all ACPI 2.0+ firmware.
/// Value: {8868e871-e4f1-11d3-bc22-0080c73c8881}
const ACPI_20_TABLE_GUID: Guid = Guid::new(
    0x8868e871_u32.to_ne_bytes(),
    0xe4f1_u16.to_ne_bytes(),
    0x11d3_u16.to_ne_bytes(),
    0xbc,
    0x22,
    [0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81],
);

/// ACPI 1.0 RSDP table GUID. Fallback for firmware that only exposes ACPI 1.0.
/// Value: {eb9d2d30-2d88-11d3-9a16-0090273fc14d}
const ACPI_10_TABLE_GUID: Guid = Guid::new(
    0xeb9d2d30_u32.to_ne_bytes(),
    0x2d88_u16.to_ne_bytes(),
    0x11d3_u16.to_ne_bytes(),
    0x9a,
    0x16,
    [0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
);

/// Flattened Device Tree (FDT/DTB) table GUID. Present on firmware that exposes a DTB.
/// Value: {b1b621d5-f19c-41a5-830b-d9152c69aae0}
const FDT_TABLE_GUID: Guid = Guid::new(
    0xb1b621d5_u32.to_ne_bytes(),
    0xf19c_u16.to_ne_bytes(),
    0x41a5_u16.to_ne_bytes(),
    0x83,
    0x0b,
    [0xd9, 0x15, 0x2c, 0x69, 0xaa, 0xe0],
);

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

    /// Returns a reference to the EFI Boot Services Table, or `None` if
    /// `ExitBootServices` has already been called (boot services pointer is null).
    ///
    /// Prefer this over [`boot_services`] in contexts where panicking is
    /// undesirable — such as panic handlers — because it never panics.
    ///
    /// [`boot_services`]: Self::boot_services
    pub fn boot_services_opt(&self) -> Option<&BootServices> {
        if self.boot_services.is_null() {
            None
        } else {
            Some(unsafe { &*self.boot_services })
        }
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

    /// Searches the UEFI configuration table array for an entry with the given vendor GUID.
    ///
    /// Returns the `vendor_table` pointer of the first matching entry, or `None` if not found.
    /// Must be called before ExitBootServices on firmware that may free configuration tables
    /// during shutdown.
    pub fn find_configuration_table(&self, guid: &Guid) -> Option<*mut core::ffi::c_void> {
        self.configuration_tables()
            .iter()
            .find(|t| t.vendor_guid == *guid)
            .map(|t| t.vendor_table)
    }

    /// Scans the UEFI configuration tables for the ACPI RSDP.
    ///
    /// Prefers ACPI 2.0 (`{8868e871-...}`); falls back to ACPI 1.0 (`{eb9d2d30-...}`).
    /// Returns the physical address of the RSDP, or `None` if neither entry is present.
    /// Must be called before ExitBootServices.
    pub fn find_acpi_rsdp(&self) -> Option<u64> {
        self.find_configuration_table(&ACPI_20_TABLE_GUID)
            .or_else(|| self.find_configuration_table(&ACPI_10_TABLE_GUID))
            .map(|p| p as u64)
    }

    /// Scans the UEFI configuration tables for a Flattened Device Tree blob.
    ///
    /// Returns the physical address of the FDT, or `None` if the `{b1b621d5-...}` entry
    /// is not present. Must be called before ExitBootServices.
    pub fn find_dtb(&self) -> Option<usize> {
        self.find_configuration_table(&FDT_TABLE_GUID)
            .map(|p| p as usize)
    }
}
