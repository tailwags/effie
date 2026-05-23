use core::{
    alloc::{GlobalAlloc, Layout},
    ffi::c_void,
    ptr::null_mut,
};

use crate::{
    system_table,
    tables::{BootServices, MemoryType, PhysicalAddress},
};

const PAGE_SIZE: usize = 4096;

#[repr(transparent)]
pub struct Allocator;

const fn page_alloc_eligible(layout: &Layout) -> bool {
    layout.align() == PAGE_SIZE && layout.size() % PAGE_SIZE == 0
}

// AllocatePool guarantees 8-byte alignment. For align > 8, over-allocate by `align`
// bytes and slide to the first aligned address, storing the original pool pointer
// in the word immediately before it so dealloc can free the right block.
fn alloc_aligned(boot_services: &BootServices, size: usize, align: usize) -> *mut u8 {
    let raw: *mut u8 = match boot_services.allocate_pool(MemoryType::LOADER_DATA, size + align) {
        Ok(p) => p.cast(),
        Err(_) => return null_mut(),
    };

    // allocate_pool gives 8-byte alignment; for align >= 16 (the minimum power-of-2
    // above 8) align_offset on that base is always a nonzero multiple of 8, so there
    // is always room for the stored pointer before `aligned`. If already aligned we
    // advance by a full `align` to maintain that guarantee.
    let offset = match raw.align_offset(align) {
        0 => align,
        n => n,
    };

    unsafe {
        let aligned = raw.add(offset);
        aligned.cast::<*mut u8>().sub(1).write(raw);
        aligned
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let boot_services = system_table().boot_services();

        if page_alloc_eligible(&layout) {
            return boot_services
                .allocate_any_pages(MemoryType::LOADER_DATA, layout.size() / PAGE_SIZE)
                .map(|PhysicalAddress(addr)| addr as usize as *mut u8)
                .unwrap_or(null_mut());
        }

        if layout.align() <= 8 {
            boot_services
                .allocate_pool(MemoryType::LOADER_DATA, layout.size())
                .unwrap_or(null_mut())
                .cast()
        } else {
            alloc_aligned(boot_services, layout.size(), layout.align())
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let boot_services = system_table().boot_services();

        if page_alloc_eligible(&layout) {
            let result = unsafe {
                boot_services.free_pages(PhysicalAddress(ptr as u64), layout.size() / PAGE_SIZE)
            };
            debug_assert!(result.is_ok(), "free_pages failed: {:?}", result.err());
            return;
        }

        let raw: *mut c_void = if layout.align() <= 8 {
            ptr.cast()
        } else {
            unsafe { ptr.cast::<*mut u8>().sub(1).read().cast() }
        };

        let result = unsafe { boot_services.free_pool(raw) };
        debug_assert!(result.is_ok(), "free_pool failed: {:?}", result.err());
    }
}
