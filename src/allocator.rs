use core::{alloc::GlobalAlloc, ptr::null_mut};

use crate::{system_table, tables::MemoryType};

#[repr(transparent)]
pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let boot_services = system_table().boot_services();

        boot_services
            .allocate_pool(MemoryType::LOADER_DATA, layout.size())
            .unwrap_or(null_mut())
            .cast()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: core::alloc::Layout) {
        let boot_services = system_table().boot_services();

        let _ = boot_services.free_pool(ptr.cast());
    }
}
