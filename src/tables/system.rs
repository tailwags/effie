use core::ffi::c_void;

use crate::{
    Guid, Handle, WStr,
    protocols::{SimpleTextInput, SimpleTextOutput},
    tables::{BootServices, RuntimeServices, TableHeader},
};

use super::{Signature, SpecificationRevision};

/// Contains pointers to the runtime and boot services tables.
#[repr(C)]
pub struct SystemTable {
    /// TODO
    hdr: TableHeader,
    /// A pointer to a null terminated string that identifies the vendor that produces the system firmware for the platform
    firmware_vendor: *const u16,
    /// A firmware vendor specific value that identifies the revision of the system firmware for the platform.
    firmware_version: u32,
    /// The handle for the active console input device.
    console_in_handle: Handle,
    /// A pointer to the EFI_SIMPLE_TEXT_INPUT_PROTOCOL interface that is associated with ConsoleInHandle.
    con_in: *const SimpleTextInput,
    console_out_handle: Handle,
    con_out: *const SimpleTextOutput,
    standard_error_handler: Handle,
    std_err: *const SimpleTextOutput,
    runtime_services: *const RuntimeServices,
    pub(crate) boot_services: *const BootServices,
    number_of_table_entries: usize,
    configuration_table: *const ConfigurationTable,
}

#[repr(C)]
struct ConfigurationTable {
    vendor_guid: Guid,
    vendor_table: *mut c_void,
}

impl SystemTable {
    pub fn signature(&self) -> Signature {
        self.hdr.signature
    }

    pub fn revision(&self) -> SpecificationRevision {
        self.hdr.revision
    }

    pub fn firmware_vendor(&self) -> &WStr {
        unsafe { WStr::from_ptr(self.firmware_vendor) }
    }

    pub fn con_in(&self) -> &SimpleTextInput {
        unsafe { &*self.con_in }
    }

    pub fn con_out(&self) -> &SimpleTextOutput {
        unsafe { &*self.con_out }
    }

    pub fn boot_services(&self) -> &BootServices {
        debug_assert!(
            !self.boot_services.is_null(),
            "boot services unavailable (ExitBootServices already called?)"
        );
        unsafe { &*self.boot_services }
    }

    pub fn runtime_services(&self) -> &RuntimeServices {
        debug_assert!(
            !self.runtime_services.is_null(),
            "runtime services pointer is null"
        );
        unsafe { &*self.runtime_services }
    }
}
