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

struct PDA {
    irqcount: i32,
    irqstackptr: *mut u8,
}

static mut PDA: PDA = PDA {
    irqcount: -1,
    irqstackptr: core::ptr::null_mut(),
};
static mut IRQSTACK: [u8; 2 * 32768] = [0; 2 * 32768];

unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;

    asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low, in("edx") high,
        options(nostack, preserves_flags),
    );
}

/// Initialise events
pub fn init_events() {
    unsafe {
        // asm!("movl 0, %fs", options(nostack, preserves_flags));
        // asm!("movl 0, %gs", options(nostack, preserves_flags));
        write_msr(0xc0000101, &mut PDA as *mut _ as u64);
        PDA.irqcount = -1;
        PDA.irqstackptr = (((IRQSTACK.as_mut_ptr() as u64) + 2 * 32768) & !(32768 - 1)) as *mut _;
    }
}
