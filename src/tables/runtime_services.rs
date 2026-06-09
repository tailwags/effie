use core::{ffi::c_void, mem::MaybeUninit};

use bitflags::bitflags;

use crate::{Guid, Result, Status, WStr, types::Time};

use super::{MemoryDescriptor, TableHeader};

/// Reset type for `ResetSystem`. (UEFI specification §8.5.1)
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResetType {
    /// System-wide reset; all circuitry returns to its initial state.
    Cold = 0,
    /// System-wide initialisation; processor reset, pending cycles preserved.
    Warm = 1,
    /// Power off.
    Shutdown = 2,
    /// Platform-specific reset.
    PlatformSpecific = 3,
}

/// Real-time clock capabilities returned by `GetTime`.
///
/// (UEFI specification §8.3.1: EFI_TIME_CAPABILITIES)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TimeCapabilities {
    /// Provides the reporting resolution of the real-time clock in counts
    /// per second. For a normal PC-AT CMOS RTC device, this value would be 1
    /// Hz, or 1, to indicate that the device only reports the time to the
    /// nearest second.
    pub resolution: u32,
    /// Provides the timekeeping accuracy of the real-time clock in an error
    /// rate of 1E-6 parts per million (1E-12 relative error). For a clock
    /// with an accuracy of 50 ppm, the value in this field would be
    /// 50,000,000.
    pub accuracy: u32,
    /// A value of TRUE indicates that a time set operation clears the device's
    /// time below the Resolution reporting level. A value of FALSE indicates
    /// that the state below the Resolution level of the device is not cleared
    /// when the time is set. Normal PC-AT CMOS RTC devices set this value to
    /// FALSE.
    pub sets_to_zero: u8,
    _pad: [u8; 3],
}

bitflags! {
    /// UEFI variable attribute bitmask.
    ///
    /// Controls persistence, access, and authentication of UEFI variables.
    /// (UEFI specification §8.2)
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct VariableAttributes: u32 {
        /// Variable is stored in non-volatile storage.
        const NON_VOLATILE                             = 0x0000_0001;
        /// Variable is accessible during boot services.
        const BOOTSERVICE_ACCESS                       = 0x0000_0002;
        /// Variable is accessible during runtime (after `ExitBootServices`).
        const RUNTIME_ACCESS                           = 0x0000_0004;
        /// Variable is a hardware error record.
        const HARDWARE_ERROR_RECORD                    = 0x0000_0008;
        /// Authenticated write access (deprecated since UEFI 2.8).
        const AUTHENTICATED_WRITE_ACCESS               = 0x0000_0010;
        /// Time-based authenticated write access.
        const TIME_BASED_AUTHENTICATED_WRITE_ACCESS    = 0x0000_0020;
        /// Append the supplied data to an existing variable.
        const APPEND_WRITE                             = 0x0000_0040;
        /// Enhanced authenticated access (UEFI 2.6+).
        const ENHANCED_AUTHENTICATED_ACCESS            = 0x0000_0080;
    }
}

/// UEFI capsule header. (UEFI specification §8.5.3: EFI_CAPSULE_HEADER)
///
/// Describes a firmware or driver capsule delivered via `UpdateCapsule`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CapsuleHeader {
    /// GUID that defines the contents of the capsule.
    pub capsule_guid: Guid,
    /// Size in bytes of the capsule header. This may be larger than the size of
    /// `EFI_CAPSULE_HEADER` since `CapsuleGuid` may imply extended header entries.
    pub header_size: u32,
    /// Bitmask of [`CapsuleFlags`].
    pub flags: CapsuleFlags,
    /// Size in bytes of the entire capsule including the header.
    pub capsule_image_size: u32,
}

bitflags! {
    /// Flags for [`CapsuleHeader::flags`].
    ///
    /// (UEFI specification §8.5.3)
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct CapsuleFlags: u32 {
        /// Indicates that the firmware should process the capsule on the next
        /// system reset. Requires NVRAM storage of the scatter-gather list.
        const PERSIST_ACROSS_RESET  = 0x0001_0000;
        /// Populate the EFI System Table with the contents of this capsule on
        /// the next system reset (requires `PERSIST_ACROSS_RESET`).
        const POPULATE_SYSTEM_TABLE = 0x0002_0000;
        /// The firmware should initiate a reset after coalescing the capsule
        /// (requires `PERSIST_ACROSS_RESET`).
        const INITIATE_RESET        = 0x0004_0000;
    }
}

bitflags! {
    /// Disposition flags for [`RuntimeServices::convert_pointer`].
    ///
    /// (UEFI specification §8.4.2)
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ConvertPointerFlags: usize {
        /// If set and `*Address` is `NULL`, the call succeeds without
        /// converting the pointer. Allows optional pointers in data structures
        /// to be fixed up without special-casing every field.
        const OPTIONAL_PTR = 0x0000_0001;
    }
}

/// EFI_RUNTIME_SERVICES table. (UEFI specification §8.1: EFI_RUNTIME_SERVICES)
///
/// All services in this table remain valid after `ExitBootServices`, except
/// that [`RuntimeServices::set_virtual_address_map`] must be called **exactly
/// once** during the transition to virtual mode, and
/// [`RuntimeServices::convert_pointer`] is only valid during the
/// `SetVirtualAddressMap` callback.
///
/// The memory map passed to `SetVirtualAddressMap` must describe new virtual
/// addresses for all memory regions that have the `EFI_MEMORY_RUNTIME`
/// attribute set.
#[repr(C)]
pub struct RuntimeServices {
    /// Standard EFI table header. Signature is `RUNTSERV`.
    hdr: TableHeader,

    // ── Time Services (§8.3) ──────────────────────────────────────────────
    get_time:
        unsafe extern "efiapi" fn(time: *mut Time, capabilities: *mut TimeCapabilities) -> Status,

    set_time: unsafe extern "efiapi" fn(time: *const Time) -> Status,

    get_wakeup_time:
        unsafe extern "efiapi" fn(enabled: *mut u8, pending: *mut u8, time: *mut Time) -> Status,

    set_wakeup_time: unsafe extern "efiapi" fn(enable: u8, time: *const Time) -> Status,

    // ── Virtual Memory Services (§8.4) ───────────────────────────────────
    set_virtual_address_map: unsafe extern "efiapi" fn(
        memory_map_size: usize,
        descriptor_size: usize,
        descriptor_version: u32,
        virtual_map: *mut MemoryDescriptor,
    ) -> Status,

    convert_pointer:
        unsafe extern "efiapi" fn(debug_disposition: usize, address: *mut *mut c_void) -> Status,

    // ── Variable Services (§8.2) ──────────────────────────────────────────
    get_variable: unsafe extern "efiapi" fn(
        variable_name: *const u16,
        vendor_guid: *const Guid,
        attributes: *mut u32,
        data_size: *mut usize,
        data: *mut c_void,
    ) -> Status,

    get_next_variable_name: unsafe extern "efiapi" fn(
        variable_name_size: *mut usize,
        variable_name: *mut u16,
        vendor_guid: *mut Guid,
    ) -> Status,

    set_variable: unsafe extern "efiapi" fn(
        variable_name: *const u16,
        vendor_guid: *const Guid,
        attributes: u32,
        data_size: usize,
        data: *const c_void,
    ) -> Status,

    // ── Miscellaneous Services (§8.5) ─────────────────────────────────────
    get_next_high_monotonic_count: unsafe extern "efiapi" fn(high_count: *mut u32) -> Status,

    reset_system: unsafe extern "efiapi" fn(
        reset_type: ResetType,
        reset_status: Status,
        data_size: usize,
        reset_data: *const u8,
    ),

    // ── Capsule Services (§8.5.3, §8.5.4; added in UEFI 2.0) ────────────
    update_capsule: unsafe extern "efiapi" fn(
        capsule_header_array: *const *const CapsuleHeader,
        capsule_count: usize,
        scatter_gather_list: u64,
    ) -> Status,

    query_capsule_capabilities: unsafe extern "efiapi" fn(
        capsule_header_array: *const *const CapsuleHeader,
        capsule_count: usize,
        maximum_capsule_size: *mut u64,
        reset_type: *mut ResetType,
    ) -> Status,

    query_variable_info: unsafe extern "efiapi" fn(
        attributes: u32,
        maximum_variable_storage_size: *mut u64,
        remaining_variable_storage_size: *mut u64,
        maximum_variable_size: *mut u64,
    ) -> Status,
}

// SAFETY: RuntimeServices is a C ABI table at a stable physical address for the
// UEFI session lifetime. Effie is single-threaded; no concurrent access occurs.
unsafe impl Send for RuntimeServices {}
unsafe impl Sync for RuntimeServices {}

impl RuntimeServices {
    // ── Time Services (UEFI specification §8.3) ───────────────────────────

    /// Returns the current time and date information as set in the CMOS RTC.
    /// Optionally also returns the real-time clock device's capabilities.
    ///
    /// (UEFI specification §8.3.1: `EFI_RUNTIME_SERVICES.GetTime`)
    ///
    /// Valid before and after `ExitBootServices`.
    pub fn get_time(&self) -> Result<(Time, TimeCapabilities)> {
        let mut time = MaybeUninit::<Time>::uninit();
        let mut caps = MaybeUninit::<TimeCapabilities>::uninit();
        // SAFETY: function pointer is valid for the lifetime of the table.
        // Both output buffers are properly aligned and sized.
        let status = unsafe { (self.get_time)(time.as_mut_ptr(), caps.as_mut_ptr()) };
        status.into_result()?;
        // SAFETY: firmware initialised both outputs on EFI_SUCCESS.
        Ok(unsafe { (time.assume_init(), caps.assume_init()) })
    }

    /// Sets the current local time and date information from the fields in
    /// `time`.
    ///
    /// (UEFI specification §8.3.2: `EFI_RUNTIME_SERVICES.SetTime`)
    ///
    /// Valid before and after `ExitBootServices`.
    pub fn set_time(&self, time: &Time) -> Result {
        // SAFETY: function pointer valid; `time` reference outlives the call.
        unsafe { (self.set_time)(time) }.into_result()
    }

    /// Returns the current wakeup alarm clock setting.
    ///
    /// Returns `(enabled, pending, time)`:
    /// - `enabled` — `true` if the alarm is armed.
    /// - `pending` — `true` if the alarm signal is pending and requires
    ///   acknowledgement.
    /// - `time` — the current alarm setting.
    ///
    /// (UEFI specification §8.3.3: `EFI_RUNTIME_SERVICES.GetWakeupTime`)
    ///
    /// Valid before and after `ExitBootServices`.
    pub fn get_wakeup_time(&self) -> Result<(bool, bool, Time)> {
        let mut enabled: u8 = 0;
        let mut pending: u8 = 0;
        let mut time = MaybeUninit::<Time>::uninit();
        // SAFETY: function pointer valid; output pointers are valid references.
        let status =
            unsafe { (self.get_wakeup_time)(&mut enabled, &mut pending, time.as_mut_ptr()) };
        status.into_result()?;
        // SAFETY: firmware initialised `time` on EFI_SUCCESS.
        Ok(unsafe { (enabled != 0, pending != 0, time.assume_init()) })
    }

    /// Sets the system wakeup alarm clock time.
    ///
    /// If `enable` is `true`, the alarm is armed at `time` (which must be
    /// `Some`). If `enable` is `false`, the alarm is disabled and `time` may
    /// be `None`.
    ///
    /// (UEFI specification §8.3.4: `EFI_RUNTIME_SERVICES.SetWakeupTime`)
    ///
    /// Valid before and after `ExitBootServices`.
    pub fn set_wakeup_time(&self, enable: bool, time: Option<&Time>) -> Result {
        let time_ptr = match time {
            Some(t) => t as *const Time,
            None => core::ptr::null(),
        };
        // SAFETY: function pointer valid; `time_ptr` is either null (allowed
        // when disabling) or a valid reference that outlives the call.
        unsafe { (self.set_wakeup_time)(enable as u8, time_ptr) }.into_result()
    }

    // ── Virtual Memory Services (UEFI specification §8.4) ─────────────────

    /// Converts a runtime driver's physical memory addresses to virtual
    /// addresses after `ExitBootServices`.
    ///
    /// `descriptors` is a slice of `EFI_MEMORY_DESCRIPTOR` entries (typically
    /// a subset of the pre-EBS memory map) with `virtual_start` filled in for
    /// each region that has the `EFI_MEMORY_RUNTIME` attribute. The caller
    /// must supply `descriptor_size` and `descriptor_version` from the
    /// original `GetMemoryMap` call.
    ///
    /// May only be called **once** per boot, immediately after
    /// `ExitBootServices`, before transitioning to virtual mode.
    ///
    /// (UEFI specification §8.4.1: `EFI_RUNTIME_SERVICES.SetVirtualAddressMap`)
    ///
    /// # Safety
    ///
    /// Calling this more than once or after the system is running in virtual
    /// mode is undefined behaviour. The descriptor slice must describe valid
    /// physical-to-virtual mappings for all runtime memory regions.
    pub unsafe fn set_virtual_address_map(
        &self,
        descriptor_size: usize,
        descriptor_version: u32,
        virtual_map: &mut [MemoryDescriptor],
    ) -> Result {
        let memory_map_size = virtual_map.len() * descriptor_size;
        // SAFETY: caller guarantees the descriptor slice and mapping are valid.
        unsafe {
            (self.set_virtual_address_map)(
                memory_map_size,
                descriptor_size,
                descriptor_version,
                virtual_map.as_mut_ptr(),
            )
        }
        .into_result()
    }

    /// Determines the new virtual address that is to be used on subsequent
    /// memory accesses.
    ///
    /// `debug_disposition` controls how null pointers are handled (see
    /// [`ConvertPointerFlags::OPTIONAL_PTR`]).
    ///
    /// (UEFI specification §8.4.2: `EFI_RUNTIME_SERVICES.ConvertPointer`)
    ///
    /// # Safety
    ///
    /// Must only be called during a `SetVirtualAddressMap` notification
    /// callback. `address` must point to a pointer that refers to a physical
    /// address in one of the virtual-map descriptor regions.
    pub unsafe fn convert_pointer(
        &self,
        debug_disposition: ConvertPointerFlags,
        address: *mut *mut c_void,
    ) -> Result {
        // SAFETY: caller guarantees this is called during SetVirtualAddressMap.
        unsafe { (self.convert_pointer)(debug_disposition.bits(), address) }.into_result()
    }

    // ── Variable Services (UEFI specification §8.2) ───────────────────────

    /// Returns the value and attributes of a UEFI variable.
    ///
    /// `buf` receives the variable data. On success the returned `usize` is
    /// the number of bytes written. If the buffer is too small the call returns
    /// [`Status::BUFFER_TOO_SMALL`] and `buf` is unchanged; call
    /// [`get_variable_size`](Self::get_variable_size) to query the required
    /// size.
    ///
    /// (UEFI specification §8.2.1: `EFI_RUNTIME_SERVICES.GetVariable`)
    ///
    /// # Safety
    ///
    /// Valid before and after `ExitBootServices` for variables with
    /// `RUNTIME_ACCESS` set, but the global allocator is unavailable after EBS.
    pub unsafe fn get_variable(
        &self,
        name: &WStr,
        vendor_guid: &Guid,
        buf: &mut [u8],
    ) -> Result<(VariableAttributes, usize)> {
        let mut attributes: u32 = 0;
        let mut data_size: usize = buf.len();
        // SAFETY: all pointers are valid for the duration of the call.
        let status = unsafe {
            (self.get_variable)(
                name.as_ptr(),
                vendor_guid,
                &mut attributes,
                &mut data_size,
                buf.as_mut_ptr().cast::<c_void>(),
            )
        };
        status.into_result()?;
        Ok((VariableAttributes::from_bits_retain(attributes), data_size))
    }

    /// Returns the required buffer size in bytes for [`get_variable`](Self::get_variable).
    ///
    /// (UEFI specification §8.2.1: `EFI_RUNTIME_SERVICES.GetVariable`)
    ///
    /// # Safety
    ///
    /// See [`get_variable`](Self::get_variable).
    pub unsafe fn get_variable_size(&self, name: &WStr, vendor_guid: &Guid) -> Result<usize> {
        let mut data_size: usize = 0;
        // SAFETY: passing null data pointer with data_size=0 to query size.
        let status = unsafe {
            (self.get_variable)(
                name.as_ptr(),
                vendor_guid,
                core::ptr::null_mut(),
                &mut data_size,
                core::ptr::null_mut(),
            )
        };
        // Firmware returns BUFFER_TOO_SMALL when data pointer is null and
        // sets data_size to the required byte count.
        if status == Status::BUFFER_TOO_SMALL {
            return Ok(data_size);
        }
        status.into_result()?;
        Ok(data_size)
    }

    /// Enumerates the current variable names; used to walk all stored
    /// variables.
    ///
    /// `name_buf` is a buffer of `u16` code units (null-terminated). On the
    /// first call, set `name_buf[0] = 0` and `vendor_guid` to
    /// [`Guid::ZERO`](crate::Guid::ZERO). On each subsequent call pass the
    /// values returned by the previous call. Returns
    /// [`Status::NOT_FOUND`] when the enumeration is exhausted.
    ///
    /// On success writes the next variable name into `name_buf` and updates
    /// `vendor_guid` in-place, then returns a reference to the name.
    ///
    /// (UEFI specification §8.2.2: `EFI_RUNTIME_SERVICES.GetNextVariableName`)
    ///
    /// # Safety
    ///
    /// `name_buf` must be large enough to hold the next variable name plus a
    /// null terminator. Firmware writes directly into the buffer; if the
    /// buffer is too small the call returns [`Status::BUFFER_TOO_SMALL`] and
    /// the buffer contents are unspecified.
    pub unsafe fn get_next_variable_name<'buf>(
        &self,
        name_buf: &'buf mut [u16],
        vendor_guid: &mut Guid,
    ) -> Result<&'buf WStr> {
        let mut size = name_buf.len() * core::mem::size_of::<u16>();
        // SAFETY: caller guarantees `name_buf` is valid and large enough.
        let status =
            unsafe { (self.get_next_variable_name)(&mut size, name_buf.as_mut_ptr(), vendor_guid) };
        status.into_result()?;
        // SAFETY: on EFI_SUCCESS firmware wrote a null-terminated UTF-16
        // string into `name_buf`.
        let wstr = unsafe { WStr::from_ptr(name_buf.as_ptr()) };
        Ok(wstr)
    }

    /// Creates, updates, or deletes a UEFI variable.
    ///
    /// To delete a variable, set `data_size` to 0 (pass an empty slice).
    ///
    /// (UEFI specification §8.2.3: `EFI_RUNTIME_SERVICES.SetVariable`)
    ///
    /// Valid before and after `ExitBootServices` for variables with
    /// `RUNTIME_ACCESS` set.
    pub fn set_variable(
        &self,
        name: &WStr,
        vendor_guid: &Guid,
        attributes: VariableAttributes,
        data: &[u8],
    ) -> Result {
        // SAFETY: pointers are valid references that outlive the call.
        unsafe {
            (self.set_variable)(
                name.as_ptr(),
                vendor_guid,
                attributes.bits(),
                data.len(),
                data.as_ptr().cast::<c_void>(),
            )
        }
        .into_result()
    }

    // ── Miscellaneous Services (UEFI specification §8.5) ──────────────────

    /// Returns the next high 32 bits of the platform's monotonic counter.
    ///
    /// The counter is a 64-bit value composed of a volatile low 32-bit half
    /// (not returned here; it starts at 0 after each reset) and a persistent
    /// high 32-bit half stored in non-volatile storage. Each call increments
    /// the high count and persists it.
    ///
    /// (UEFI specification §8.5.2:
    /// `EFI_RUNTIME_SERVICES.GetNextHighMonotonicCount`)
    ///
    /// Valid before and after `ExitBootServices`.
    pub fn get_next_high_monotonic_count(&self) -> Result<u32> {
        let mut high_count: u32 = 0;
        // SAFETY: function pointer valid; high_count is a properly aligned
        // stack variable.
        unsafe { (self.get_next_high_monotonic_count)(&mut high_count) }.into_result()?;
        Ok(high_count)
    }

    /// Resets the platform. Does not return.
    ///
    /// (UEFI specification §8.5.1: `EFI_RUNTIME_SERVICES.ResetSystem`)
    ///
    /// Valid before and after `ExitBootServices`.
    pub fn reset_system(&self, reset_type: ResetType) -> ! {
        // SAFETY: function pointer is non-null for the lifetime of the table.
        unsafe {
            (self.reset_system)(reset_type, Status::SUCCESS, 0, core::ptr::null());
        }
        loop {}
    }

    /// Passes capsule(s) to the firmware. If any capsule has
    /// [`CapsuleFlags::PERSIST_ACROSS_RESET`] set, the scatter-gather list
    /// must describe a physical-address list of
    /// [`EfiCapsuleBlockDescriptor`] structures stored in
    /// `EfiRuntimeServicesData` pages; `scatter_gather_list` is the physical
    /// address of the first descriptor (or 0 if no persist flag is set).
    ///
    /// (UEFI specification §8.5.3: `EFI_RUNTIME_SERVICES.UpdateCapsule`)
    ///
    /// # Safety
    ///
    /// Each pointer in `capsule_header_array` must be valid and point to a
    /// well-formed [`CapsuleHeader`] followed by capsule data. If
    /// `PERSIST_ACROSS_RESET` is set, `scatter_gather_list` must be a valid
    /// physical address of the scatter-gather descriptor list, and all
    /// referenced pages must be `EfiRuntimeServicesData`.
    pub unsafe fn update_capsule(
        &self,
        capsule_header_array: &[*const CapsuleHeader],
        scatter_gather_list: u64,
    ) -> Result {
        // SAFETY: caller guarantees validity of all pointers and the SGL.
        unsafe {
            (self.update_capsule)(
                capsule_header_array.as_ptr(),
                capsule_header_array.len(),
                scatter_gather_list,
            )
        }
        .into_result()
    }

    /// Returns the maximum capsule size the firmware can handle and the reset
    /// type required to activate the capsule(s).
    ///
    /// (UEFI specification §8.5.4:
    /// `EFI_RUNTIME_SERVICES.QueryCapsuleCapabilities`)
    ///
    /// # Safety
    ///
    /// Each pointer in `capsule_header_array` must be valid and point to a
    /// well-formed [`CapsuleHeader`].
    pub unsafe fn query_capsule_capabilities(
        &self,
        capsule_header_array: &[*const CapsuleHeader],
    ) -> Result<(u64, ResetType)> {
        let mut maximum_capsule_size: u64 = 0;
        let mut reset_type = MaybeUninit::<ResetType>::uninit();
        // SAFETY: caller guarantees validity of all capsule header pointers.
        let status = unsafe {
            (self.query_capsule_capabilities)(
                capsule_header_array.as_ptr(),
                capsule_header_array.len(),
                &mut maximum_capsule_size,
                reset_type.as_mut_ptr(),
            )
        };
        status.into_result()?;
        // SAFETY: firmware initialised reset_type on EFI_SUCCESS.
        Ok(unsafe { (maximum_capsule_size, reset_type.assume_init()) })
    }

    /// Returns information about the maximum variable storage capacity for
    /// variables with the given `attributes`.
    ///
    /// Returns `(maximum_variable_storage_size, remaining_variable_storage_size,
    /// maximum_variable_size)`:
    /// - `maximum_variable_storage_size` — total bytes of NV storage that
    ///   can hold variables with these attributes.
    /// - `remaining_variable_storage_size` — bytes still available.
    /// - `maximum_variable_size` — largest single variable (name + data)
    ///   that can be stored.
    ///
    /// (UEFI specification §8.2.4: `EFI_RUNTIME_SERVICES.QueryVariableInfo`)
    ///
    /// Valid before and after `ExitBootServices`.
    pub fn query_variable_info(&self, attributes: VariableAttributes) -> Result<(u64, u64, u64)> {
        let mut max_storage: u64 = 0;
        let mut remaining: u64 = 0;
        let mut max_var: u64 = 0;
        // SAFETY: function pointer valid; output pointers are valid stack
        // references.
        unsafe {
            (self.query_variable_info)(
                attributes.bits(),
                &mut max_storage,
                &mut remaining,
                &mut max_var,
            )
        }
        .into_result()?;
        Ok((max_storage, remaining, max_var))
    }
}
