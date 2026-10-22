use core::{
    alloc::{GlobalAlloc, Layout},
    ffi::c_void,
    ptr::null_mut,
};

use crate::{
    Status, log, system_table,
    tables::{BootServices, MemoryType, PhysicalAddress},
};

/// UEFI page size in bytes (4 KiB).
const PAGE_SIZE: usize = 4096;

/// UEFI-backed global allocator.
///
/// Implements [`GlobalAlloc`] using UEFI boot services:
///
/// * **Page allocations** (size and alignment are multiples of 4 KiB) go through
///   [`AllocatePages`](crate::tables::BootServices::allocate_any_pages).
/// * **Small allocations** (alignment ≤ 8) use
///   [`AllocatePool`](crate::tables::BootServices::allocate_pool).
/// * **Over-aligned allocations** (alignment > 8) over-allocate and slide to the
///   correct alignment, storing the original pointer for `dealloc`.
///
/// The allocator is available only until
/// [`ExitBootServices`](crate::tables::BootServices::exit_boot_services).
/// After that point, all heap operations are unsound.
#[repr(transparent)]
pub struct Allocator;

/// Returns `true` if `layout` describes a page-sized, page-aligned allocation
/// that is eligible for `AllocatePages` rather than `AllocatePool`.
const fn page_alloc_eligible(layout: &Layout) -> bool {
    layout.align() == PAGE_SIZE && layout.size().is_multiple_of(PAGE_SIZE)
}

// Failures are reported out of line, in one place each, so every call site
// stays small.
#[cold]
#[inline(never)]
fn failed(call: &str, size: usize, err: Status) -> *mut u8 {
    log::error!("allocator: {call} failed: size={size} err={err}");
    null_mut()
}

#[cold]
#[inline(never)]
fn free_failed(call: &str, err: Status) {
    log::error!("allocator: {call} failed: {err}");
    debug_assert!(false, "{call} failed: {err:?}");
}

/// AllocatePool guarantees 8-byte alignment. For align > 8, over-allocate by `align`
/// bytes and slide to the first aligned address, storing the original pool pointer
/// in the word immediately before it so dealloc can free the right block.
fn alloc_aligned(boot_services: &BootServices, size: usize, align: usize) -> *mut u8 {
    let raw: *mut u8 = match boot_services.allocate_pool(MemoryType::LOADER_DATA, size + align) {
        Ok(p) => p.cast(),
        Err(err) => return failed("allocate_pool (over-aligned)", size, err),
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
                .unwrap_or_else(|err| failed("allocate_any_pages", layout.size(), err));
        }

        if layout.align() <= 8 {
            boot_services
                .allocate_pool(MemoryType::LOADER_DATA, layout.size())
                .map_or_else(
                    |err| failed("allocate_pool", layout.size(), err),
                    |p| p.cast(),
                )
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
            if let Err(err) = result {
                free_failed("free_pages", err);
            }
            return;
        }

        let raw: *mut c_void = if layout.align() <= 8 {
            ptr.cast()
        } else {
            unsafe { ptr.cast::<*mut u8>().sub(1).read().cast() }
        };

        let result = unsafe { boot_services.free_pool(raw) };
        if let Err(err) = result {
            free_failed("free_pool", err);
        }
    }
}
