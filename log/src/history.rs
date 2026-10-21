//! A ring buffer holding the last bytes of output.
//!
//! UEFI boot services run on a single processor, so `push` calls never
//! overlap. `take` hands the buffer out for good: `push` stops writing to it,
//! so the returned slices are never aliased by a `&mut`. `copy` only borrows
//! the buffer while copying into memory it allocated beforehand, so nothing
//! can log (and `push`) in between.

use alloc::{boxed::Box, vec, vec::Vec};
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

/// The leaked buffer; null before `init` and after `take`.
static DATA: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());
static CAPACITY: AtomicUsize = AtomicUsize::new(0);
/// Bytes written so far; the next byte goes to `total % CAPACITY`.
static TOTAL: AtomicUsize = AtomicUsize::new(0);

pub fn init(capacity: usize) {
    let data = Box::leak(vec![0u8; capacity].into_boxed_slice());
    DATA.store(data.as_mut_ptr(), Ordering::Relaxed);
    CAPACITY.store(capacity, Ordering::Relaxed);
}

pub fn push(bytes: &[u8]) {
    let ptr = DATA.load(Ordering::Relaxed);
    if ptr.is_null() {
        return;
    }
    // SAFETY: `init` leaked `CAPACITY` bytes at `ptr`, and see the module
    // comment for why nothing else references them.
    let data = unsafe { core::slice::from_raw_parts_mut(ptr, CAPACITY.load(Ordering::Relaxed)) };
    let total = TOTAL.load(Ordering::Relaxed);
    TOTAL.store(total + bytes.len(), Ordering::Relaxed);

    let skip = bytes.len().saturating_sub(data.len());
    let bytes = &bytes[skip..];
    let start = (total + skip) % data.len();
    let (head, tail) = bytes.split_at(bytes.len().min(data.len() - start));
    data[start..start + head.len()].copy_from_slice(head);
    data[..tail.len()].copy_from_slice(tail);
}

pub fn take() -> (&'static [u8], &'static [u8]) {
    let ptr = DATA.swap(core::ptr::null_mut(), Ordering::Relaxed);
    if ptr.is_null() {
        return (&[], &[]);
    }
    // SAFETY: `init` leaked `CAPACITY` bytes at `ptr`, and after the swap
    // `push` never writes to them again.
    let data = unsafe { core::slice::from_raw_parts(ptr, CAPACITY.load(Ordering::Relaxed)) };
    split(data)
}

pub fn copy() -> Vec<u8> {
    // Allocated first: the allocator may log, which pushes.
    let mut out = Vec::with_capacity(CAPACITY.load(Ordering::Relaxed));
    let ptr = DATA.load(Ordering::Relaxed);
    if ptr.is_null() {
        return out;
    }
    // SAFETY: `init` leaked `CAPACITY` bytes at `ptr`, and nothing below
    // allocates or logs, so `push` cannot run while `data` is alive.
    let data = unsafe { core::slice::from_raw_parts(ptr, CAPACITY.load(Ordering::Relaxed)) };
    let (older, newer) = split(data);
    out.extend_from_slice(older);
    out.extend_from_slice(newer);
    out
}

/// Returns the buffer's contents as `(older, newer)`, starting at a whole line.
fn split(data: &[u8]) -> (&[u8], &[u8]) {
    let total = TOTAL.load(Ordering::Relaxed);
    if total <= data.len() {
        return (&data[..total], &[]);
    }
    // The buffer has wrapped, so the oldest line is cut; skip it.
    let (newer, older) = data.split_at(total % data.len());
    match older.iter().position(|&b| b == b'\n') {
        Some(i) => (&older[i + 1..], newer),
        None => {
            let i = newer.iter().position(|&b| b == b'\n').map_or(0, |i| i + 1);
            (&[], &newer[i..])
        }
    }
}
