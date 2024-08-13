use core::{
    alloc::GlobalAlloc,
    mem::size_of,
    ptr::{null_mut, write_unaligned},
};

use crate::{system_table, tables::MemoryType};

#[repr(transparent)]
pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let boot_services = system_table().boot_services();

        // let size = layout.size();
        // let align = layout.align();

        // if align > 8 {
        //     // todo!() // FIXME: deal with pointer with bigger alignment
        //     null_mut()
        // } else {
        //     if let Ok(ptr) = boot_services.allocate_pool(MemoryType::LOADER_DATA, size) {
        //         ptr.cast()
        //     } else {
        //         null_mut()
        //     }
        // }

        let mut size = if layout.align() <= 8 {
            layout.size()
        } else {
            layout.size() + (layout.align() - 8)
        };

        // We will store how many bytes that we have shifted in the beginning at the end.
        size += size_of::<usize>();

        // Do allocation.
        let mem = boot_services
            .allocate_pool(MemoryType::LOADER_DATA, size)
            .unwrap_or(null_mut());

        if mem.is_null() {
            return null_mut();
        }

        // Get number of bytes to shift so the alignment is correct.
        let misaligned = (mem as usize) % layout.align();
        let adjust = if misaligned == 0 {
            0
        } else {
            layout.align() - misaligned
        };

        // Store how many bytes have been shifted.
        let mem = mem.add(adjust);

        write_unaligned(mem.add(layout.size()) as *mut usize, adjust);

        mem.cast()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let boot_services = system_table().boot_services();

        let align = layout.align();

        if align > 8 {
            // todo!() // FIXME: deal with pointer with bigger alignment
        }

        // FIXME: can we deal with errors?
        let _ = boot_services.free_pool(ptr.cast());
    }
}
