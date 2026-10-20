//! Time since [`Builder::init`](crate::Builder::init), from the CPU's counter
//! rather than UEFI `GetTime()`, which often ticks once a second and is gone
//! after ExitBootServices. Once started it needs no firmware.

use core::sync::atomic::{AtomicU64, Ordering};

static START: AtomicU64 = AtomicU64::new(0);
/// Counter ticks per second; 0 if unknown.
static FREQUENCY: AtomicU64 = AtomicU64::new(0);

/// Records the start time and the counter frequency. Needs boot services.
pub fn start() {
    START.store(counter(), Ordering::Relaxed);
    FREQUENCY.store(frequency(), Ordering::Relaxed);
}

/// Milliseconds since [`start`], or `None` if the frequency is unknown.
pub fn elapsed_ms() -> Option<u64> {
    let frequency = FREQUENCY.load(Ordering::Relaxed);
    if frequency == 0 {
        return None;
    }
    let ticks = counter().wrapping_sub(START.load(Ordering::Relaxed));
    Some(ticks / frequency * 1000 + ticks % frequency * 1000 / frequency)
}

#[cfg(target_arch = "aarch64")]
fn counter() -> u64 {
    let ticks;
    // SAFETY: reading the virtual counter has no side effects, and UEFI
    // runs at an exception level where it is accessible.
    unsafe { core::arch::asm!("isb", "mrs {}, cntvct_el0", out(reg) ticks) };
    ticks
}

#[cfg(target_arch = "aarch64")]
fn frequency() -> u64 {
    let frequency: u64;
    // SAFETY: as for `counter`; firmware sets CNTFRQ_EL0 at boot.
    unsafe { core::arch::asm!("mrs {}, cntfrq_el0", out(reg) frequency) };
    frequency
}

#[cfg(target_arch = "x86_64")]
fn counter() -> u64 {
    // SAFETY: RDTSC is available on every x86_64 CPU.
    unsafe { core::arch::x86_64::_rdtsc() }
}

/// The TSC rate is not architectural, so time it against a 10 ms `Stall()`.
#[cfg(target_arch = "x86_64")]
fn frequency() -> u64 {
    const STALL_US: u64 = 10_000;
    let boot_services = effie::system_table().boot_services();
    let before = counter();
    match boot_services.stall(STALL_US as usize) {
        Ok(()) => counter().wrapping_sub(before) * (1_000_000 / STALL_US),
        Err(_) => 0,
    }
}

/// No known counter: the frequency stays unknown and lines show `?.???`.
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
fn counter() -> u64 {
    0
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
fn frequency() -> u64 {
    0
}
