//! Utility functions

use core::arch::asm;

/// Synchronised bit set
pub unsafe fn synch_set_bit(nr: u64, addr: *mut u64) {
    asm!("lock", "bts [{}], {}", in(reg) addr, in(reg) nr);
}

/// Synchronised bit clear
pub unsafe fn synch_clear_bit(nr: u64, addr: *mut u64) {
    asm!("lock", "btr [{}], {}", in(reg) addr, in(reg) nr);
}
