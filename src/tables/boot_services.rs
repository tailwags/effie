use core::{ffi::c_void, mem::MaybeUninit, ptr::null_mut};

use alloc::vec::Vec;

use crate::{
    Event, Guid, Handle, HasProtocol, Protocol, Result, Status, protocols::DevicePath,
    tables::TableHeader,
};

use super::Signature;

// FIXME: use wrapper structs for ty
#[repr(C)]
pub struct BootServices {
    hdr: TableHeader,
    raise_tpl: unsafe extern "efiapi" fn(new_tpl: Tpl) -> Tpl,
    restore_tpl: unsafe extern "efiapi" fn(old_tpl: Tpl),
    allocate_pages: unsafe extern "efiapi" fn(
        allocate_type: AllocateType,
        memory_type: MemoryType,
        pages: usize,
        memory: *mut PhysicalAddress,
    ) -> Status,
    free_pages: unsafe extern "efiapi" fn(memory: PhysicalAddress, pages: usize) -> Status,
    get_memory_map: unsafe extern "efiapi" fn(
        memory_map_size: *mut usize,
        memory_map: *mut MemoryDescriptor,
        map_key: *mut usize,
        descriptor_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> Status,
    allocate_pool: unsafe extern "efiapi" fn(
        pool_type: MemoryType,
        size: usize,
        buffer: *mut *mut c_void,
    ) -> Status,
    free_pool: unsafe extern "efiapi" fn(buffer: *const c_void) -> Status,
    create_event: unsafe extern "efiapi" fn(
        ty: u32,
        notify_tpl: Tpl,
        notify_function: Option<EventNotify>,
        notify_context: *mut c_void,
        event: *mut Event,
    ) -> Status,
    /// FIXME: implement EFI_TIMER_DELAY
    set_timer: unsafe extern "efiapi" fn(efi_event: Event, ty: u32, trigger_time: u64) -> Status,
    wait_for_event: unsafe extern "efiapi" fn(
        number_of_events: usize,
        event: *const Event,
        index: *mut usize,
    ) -> Status,
    signal_event: unsafe extern "efiapi" fn(event: Event) -> Status,
    close_event: unsafe extern "efiapi" fn(event: Event) -> Status,
    check_event: unsafe extern "efiapi" fn(event: Event) -> Status,
    /// FIXME: implement EFI_INTERFACE_TYPE
    install_protocol_interface: unsafe extern "efiapi" fn(
        handle: *mut Handle,
        protocol: *const Guid,
        interface_type: u32,
        interface: *const c_void,
    ) -> Status,
    reinstall_protocol_interface: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        old_interface: *const c_void,
        new_interface: *const c_void,
    ) -> Status,
    uninstall_protocol_interface: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        interface: *const c_void,
    ) -> Status,
    handle_protocol: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        interface: *mut *mut c_void,
    ) -> Status,
    reserved: *mut c_void,
    register_protocol_notify: unsafe extern "efiapi" fn(
        protocol: *const Guid,
        event: Event,
        registration: *mut *const c_void,
    ) -> Status,
    locate_handle: unsafe extern "efiapi" fn(
        search_type: u32,
        protocol: *const Guid,
        search_key: *const c_void,
        buffer_size: *mut usize,
        buffer: *mut Handle,
    ) -> Status,
    locate_device_path: unsafe extern "efiapi" fn(
        protocol: *const Guid,
        device_path: *mut *const DevicePath,
        device: *mut Handle,
    ) -> Status,
    install_configuration_table:
        unsafe extern "efiapi" fn(guid: *const Guid, table: *const c_void) -> Status,
    load_image: unsafe extern "efiapi" fn(
        boot_policy: bool,
        parent_image_handle: Handle,
        device_path: *const DevicePath,
        source_buffer: *const c_void,
        source_size: usize,
        image_handle: *mut Handle,
    ) -> Status,
    start_image: unsafe extern "efiapi" fn(
        image_handle: Handle,
        exit_data_size: *mut usize,
        exit_data: *mut *const u16,
    ) -> Status,
    exit: unsafe extern "efiapi" fn(
        image_handle: Handle,
        exit_status: Status,
        exit_data_size: usize,
        exit_data: *const u16,
    ) -> !,
    unload_image: unsafe extern "efiapi" fn(image_handle: Handle) -> Status,
    exit_boot_services: unsafe extern "efiapi" fn(image_handle: Handle, map_key: usize) -> Status,
    get_next_monotonic_count: unsafe extern "efiapi" fn(count: *mut u64) -> Status,
    stall: unsafe extern "efiapi" fn(microseconds: usize) -> Status,
    set_watchdog_timer: unsafe extern "efiapi" fn(
        timeout: usize,
        watchdog_code: u64,
        data_size: usize,
        watchdog_data: *const u16,
    ) -> Status,
    connect_controller: unsafe extern "efiapi" fn(
        controller_handle: Handle,
        driver_image_handle: *const Handle,
        remaining_device_path: *const DevicePath,
        recursive: bool,
    ) -> Status,
    disconnect_controller: unsafe extern "efiapi" fn(
        controller_handle: Handle,
        driver_image_handle: Handle,
        child_handle: Handle,
    ) -> Status,
    open_protocol: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        interface: *mut *mut c_void,
        agent_handle: Handle,
        controller_handle: Handle,
        attributes: OpenProtocolAttributes,
    ) -> Status,
    close_protocol: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        agent_handle: Handle,
        controller_handle: Handle,
    ) -> Status,
    open_protocol_information: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol: *const Guid,
        entry_buffer: *mut *const OpenProtocolInformationEntry,
        entry_count: *mut usize,
    ) -> Status,
    protocols_per_handle: unsafe extern "efiapi" fn(
        handle: Handle,
        protocol_buffer: *mut *mut *const Guid,
        protocol_buffer_count: *mut usize,
    ) -> Status,
    locate_handle_buffer: unsafe extern "efiapi" fn(
        search_type: u32,
        protocol: *const Guid,
        search_key: *const c_void,
        no_handles: *mut usize,
        buffer: *mut *mut Handle,
    ) -> Status,
    locate_protocol: unsafe extern "efiapi" fn(
        protocol: *const Guid,
        registration: *const c_void,
        interface: *mut *mut c_void,
    ) -> Status,
    install_multiple_protocol_interfaces:
        unsafe extern "efiapi" fn(handle: *mut Handle, ...) -> Status,
    uninstall_multiple_protocol_interfaces:
        unsafe extern "efiapi" fn(handle: Handle, ...) -> Status,
    calculate_crc32:
        unsafe extern "efiapi" fn(data: *const c_void, data_size: usize, crc32: *mut u32) -> Status,
    copy_mem:
        unsafe extern "efiapi" fn(destination: *mut c_void, source: *const c_void, length: usize),

    set_mem: unsafe extern "efiapi" fn(buffer: *mut c_void, size: usize, value: u8),
    create_event_ex: unsafe extern "efiapi" fn(
        ty: u32,
        notify_tpl: Tpl,
        notify_function: Option<EventNotify>,
        notify_context: *const c_void,
        event_group: *const Guid,
        event: *mut Event,
    ) -> Status,
}

#[repr(transparent)]
pub struct Tpl(usize);

impl Tpl {
    pub const APPLICATION: Self = Self(4);
    pub const CALLBACK: Self = Self(8);
    pub const NOTIFY: Self = Self(16);
    pub const HIGH_LEVEL: Self = Self(31);
}

#[repr(transparent)]
pub struct MemoryType(u32);

impl MemoryType {
    pub const RESERVED_MEMORY_TYPE: Self = Self(0);
    pub const LOADER_CODE: Self = Self(1);
    pub const LOADER_DATA: Self = Self(2);
    pub const BOOT_SERVICES_CODE: Self = Self(3);
    pub const BOOT_SERVICES_DATA: Self = Self(4);
    pub const RUNTIME_SERVICES_CODE: Self = Self(5);
    pub const RUNTIME_SERVICES_DATA: Self = Self(6);
    pub const CONVENTIONAL_MEMORY: Self = Self(7);
    pub const UNUSABLE_MEMORY: Self = Self(8);
    pub const ACPIRECLAIM_MEMORY: Self = Self(9);
    pub const ACPIMEMORY_NVS: Self = Self(10);
    pub const MEMORY_MAPPED_IO: Self = Self(11);
    pub const MEMORY_MAPPED_IOPORT_SPACE: Self = Self(12);
    pub const PAL_CODE: Self = Self(13);
    pub const PERSISTENT_MEMORY: Self = Self(14);
    pub const UNACCEPTED_MEMORY_TYPE: Self = Self(15);
    pub const MAX_MEMORY_TYPE: Self = Self(16);
}

#[repr(transparent)]
pub struct AllocateType(u32);

impl AllocateType {
    pub const ALLOCATE_ANY_PAGES: Self = Self(0);
    pub const ALLOCATE_MAX_ADDRESS: Self = Self(1);
    pub const ALLOCATE_ADDRESS: Self = Self(2);
    pub const MAX_ALLOCATE_TYPE: Self = Self(3);
}

#[repr(transparent)]
pub struct PhysicalAddress(pub u64);

impl From<u64> for PhysicalAddress {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
pub struct VirtualAddress(u64);

#[repr(C)]
pub struct MemoryDescriptor {
    pub ty: u32,
    pub physical_start: PhysicalAddress,
    pub virtual_start: VirtualAddress,
    pub number_of_pages: u64,
    pub attribute: u64,
}

pub struct MemoryMap {
    /// Raw byte buffer filled by GetMemoryMap.
    pub buffer: Vec<u8>,
    /// Key required for ExitBootServices.
    pub map_key: usize,
    /// Actual per-descriptor stride in bytes; may exceed size_of::<MemoryDescriptor>().
    pub descriptor_size: usize,
    pub descriptor_version: u32,
}

impl MemoryMap {
    pub fn iter(&self) -> impl Iterator<Item = &MemoryDescriptor> {
        self.buffer
            .chunks(self.descriptor_size)
            .map(|chunk| unsafe { &*(chunk.as_ptr() as *const MemoryDescriptor) })
    }
}

pub type EventNotify = unsafe extern "efiapi" fn(event: Event, context: *mut c_void);

#[repr(transparent)]
pub struct OpenProtocolAttributes(u32);

impl OpenProtocolAttributes {
    pub const BY_HANDLE_PROTOCOL: Self = Self(0x00000001);
    pub const GET_PROTOCOL: Self = Self(0x00000002);
    pub const TEST_PROTOCOL: Self = Self(0x00000004);
    pub const BY_CHILD_CONTROLLER: Self = Self(0x00000008);
    pub const BY_DRIVER: Self = Self(0x00000010);
    pub const BY_DRIVER_EXCLUSIVE: Self = Self(Self::BY_DRIVER.0 | Self::EXCLUSIVE.0);
    pub const EXCLUSIVE: Self = Self(0x00000020);
}

#[repr(C)]
pub struct OpenProtocolInformationEntry {
    pub agent_handle: Handle,
    pub controller_handle: Handle,
    pub attributes: OpenProtocolAttributes,
    pub open_count: u32,
}

impl BootServices {
    pub fn signature(&self) -> Signature {
        self.hdr.signature
    }

    // FIXME: check for errors
    // TODO: consider using a newtype wrapper that frees on drop
    pub fn allocate_pool(&self, memory_type: MemoryType, size: usize) -> Result<*mut c_void> {
        let mut buffer = null_mut();

        unsafe { (self.allocate_pool)(memory_type, size, &mut buffer) }
            .into_result()
            .map(|_| buffer)
    }

    /// # Safety
    ///
    /// The caller must ensure the pointer was allocated by allocate_pool
    pub unsafe fn free_pool(&self, buffer: *const c_void) -> Result {
        unsafe { (self.free_pool)(buffer) }.into_result()
    }

    /// # Safety
    ///
    /// `memory` must be a physical address obtained from `allocate_any_pages` or
    /// `allocate_pages_at_address`, and `pages` must match the count from that call.
    pub unsafe fn free_pages(&self, memory: PhysicalAddress, pages: usize) -> Result {
        unsafe { (self.free_pages)(memory, pages) }.into_result()
    }

    pub fn allocate_pages_at_address(
        &self,
        memory_type: MemoryType,
        pages: usize,
        address: PhysicalAddress,
    ) -> Result {
        let mut memory = address;

        unsafe {
            (self.allocate_pages)(
                AllocateType::ALLOCATE_ADDRESS,
                memory_type,
                pages,
                &mut memory,
            )
        }
        .into_result()
    }

    pub fn allocate_any_pages(
        &self,
        memory_type: MemoryType,
        pages: usize,
    ) -> Result<PhysicalAddress> {
        let mut address = PhysicalAddress::from(0);

        unsafe {
            (self.allocate_pages)(
                AllocateType::ALLOCATE_ANY_PAGES,
                memory_type,
                pages,
                &mut address,
            )
        }
        .into_result()
        .map(|_| address)
    }

    // The returned &mut P points to a firmware protocol object, not data inside
    // BootServices. UEFI is single-threaded and open_protocol hands out exclusive
    // logical ownership of the protocol interface.
    pub fn open_protocol<P: HasProtocol>(
        &self,
        handle: &Handle,
        agent: &Handle,
    ) -> Result<Protocol<P>> {
        let mut protocol = MaybeUninit::<*mut P>::uninit();

        let status = unsafe {
            (self.open_protocol)(
                *handle,
                &P::GUID,
                protocol.as_mut_ptr().cast(),
                *agent,
                Handle::null(),
                OpenProtocolAttributes::BY_HANDLE_PROTOCOL,
            )
        };

        status.into_result()?;

        Protocol::new(unsafe { protocol.assume_init() }, *handle, *agent)
    }

    pub fn locate_protocol<P: crate::HasProtocol>(&self) -> Result<Protocol<P>> {
        let mut protocol = MaybeUninit::<*mut P>::uninit();

        let status =
            unsafe { (self.locate_protocol)(&P::GUID, null_mut(), protocol.as_mut_ptr().cast()) };

        status.into_result()?;

        Protocol::new_unscoped(unsafe { protocol.assume_init() })
    }

    pub fn close_protocol<P: HasProtocol>(&self, handle: &Handle, agent: &Handle) -> Result {
        unsafe { (self.close_protocol)(*handle, &P::GUID, *agent, Handle::null()) }.into_result()
    }

    pub fn start_image(&self, image_handle: Handle) -> Result {
        unsafe { (self.start_image)(image_handle, null_mut(), null_mut()) }.into_result()
    }

    pub fn get_memory_map(&self) -> Result<MemoryMap> {
        todo!(
            "call get_memory_map raw fn with size=0 to get required size; \
             allocate Vec<u8> of that size plus two descriptor_size worth of slack \
             (allocating the Vec itself changes the map); call get_memory_map again \
             to fill the buffer; return MemoryMap with buffer, map_key, descriptor_size, \
             descriptor_version. Return Err if the status is not BUFFER_TOO_SMALL on the \
             first probe call."
        )
    }

    /// Exits boot services. After this call the global allocator is permanently dead.
    ///
    /// # Safety
    ///
    /// - `image_handle` must be the handle passed to `efi_main`.
    /// - `map_key` must have been obtained from `get_memory_map` with no intervening
    ///   boot-services calls (including allocations) between that call and this one.
    ///   Any such call invalidates the key and this function will return
    ///   `EFI_INVALID_PARAMETER`; retry the whole `get_memory_map` +
    ///   `exit_boot_services` sequence in that case.
    /// - On success, all boot services (including the global allocator) are permanently
    ///   unavailable. The caller must not use any `BootServices` reference, `Box`, `Vec`,
    ///   or other heap-backed type after a successful call.
    pub unsafe fn exit_boot_services(&self, image_handle: Handle, map_key: usize) -> Result {
        unsafe { (self.exit_boot_services)(image_handle, map_key) }.into_result()
    }

    /// Blocks until one of the events in `events` is signaled.
    /// Returns the index of the first event that fired.
    pub fn wait_for_event(&self, events: &[Event]) -> Result<usize> {
        let mut index = 0usize;
        let status = unsafe { (self.wait_for_event)(events.len(), events.as_ptr(), &mut index) };

        status.into_result().map(|_| index)
    }

    /// Stalls execution for the given number of microseconds.
    pub fn stall(&self, microseconds: usize) -> Result {
        unsafe { (self.stall)(microseconds) }.into_result()
    }
}
