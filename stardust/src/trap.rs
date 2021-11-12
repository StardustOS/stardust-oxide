//! Trap handlers

use xen::{
    dbg,
    trap::{set_trap_table, TrapInfo},
    xen_sys::FLAT_KERNEL_CS,
};

/// Registers the trap handlers
pub fn init() {
    set_trap_table(&TRAP_TABLE);
}

static TRAP_TABLE: [TrapInfo; 18] = [
    TrapInfo {
        vector: 0,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: divide_error as *const (),
    },
    TrapInfo {
        vector: 1,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: debug as *const (),
    },
    TrapInfo {
        vector: 3,
        flags: 3,
        cs: FLAT_KERNEL_CS as u16,
        address: int3 as *const (),
    },
    TrapInfo {
        vector: 4,
        flags: 3,
        cs: FLAT_KERNEL_CS as u16,
        address: overflow as *const (),
    },
    TrapInfo {
        vector: 5,
        flags: 3,
        cs: FLAT_KERNEL_CS as u16,
        address: bounds as *const (),
    },
    TrapInfo {
        vector: 6,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: invalid_op as *const (),
    },
    TrapInfo {
        vector: 7,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: device_not_available as *const (),
    },
    TrapInfo {
        vector: 9,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: coprocessor_segment_overrun as *const (),
    },
    TrapInfo {
        vector: 10,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: invalid_TSS as *const (),
    },
    TrapInfo {
        vector: 11,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: segment_not_present as *const (),
    },
    TrapInfo {
        vector: 12,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: stack_segment as *const (),
    },
    TrapInfo {
        vector: 13,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: general_protection as *const (),
    },
    TrapInfo {
        vector: 14,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: page_fault as *const (),
    },
    TrapInfo {
        vector: 15,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: spurious_interrupt_bug as *const (),
    },
    TrapInfo {
        vector: 16,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: coprocessor_error as *const (),
    },
    TrapInfo {
        vector: 17,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: alignment_check as *const (),
    },
    TrapInfo {
        vector: 19,
        flags: 0,
        cs: FLAT_KERNEL_CS as u16,
        address: simd_coprocessor_error as *const (),
    },
    TrapInfo {
        vector: 0,
        flags: 0,
        cs: 0,
        address: 0 as *const (),
    },
];

extern "C" {
    fn divide_error();
    fn debug();
    fn int3();
    fn overflow();
    fn bounds();
    fn invalid_op();
    fn device_not_available();
    fn coprocessor_segment_overrun();
    fn invalid_TSS();
    fn segment_not_present();
    fn stack_segment();
    fn general_protection();
    fn page_fault();
    fn spurious_interrupt_bug();
    fn coprocessor_error();
    fn alignment_check();
    fn simd_coprocessor_error();
}

#[no_mangle]
/// Handler for divide error trap
pub extern "C" fn do_divide_error() {
    dbg!()
}

#[no_mangle]
/// Handler for debug trap
pub extern "C" fn do_debug() {
    dbg!()
}

#[no_mangle]
/// Handler for int3 trap
pub extern "C" fn do_int3() {
    dbg!()
}

#[no_mangle]
/// Handler for overflow trap
pub extern "C" fn do_overflow() {
    dbg!()
}

#[no_mangle]
/// Handler for bounds trap
pub extern "C" fn do_bounds() {
    dbg!()
}

#[no_mangle]
/// Handler for invalid operation trap
pub extern "C" fn do_invalid_op() {
    dbg!()
}

#[no_mangle]
/// Handler for device not available trap
pub extern "C" fn do_device_not_available() {
    dbg!()
}

#[no_mangle]
/// Handler for coprocessor segment overrun trap
pub extern "C" fn do_coprocessor_segment_overrun() {
    dbg!()
}

#[no_mangle]
/// Handler for invalid TSS trap
pub extern "C" fn do_invalid_TSS() {
    dbg!()
}

#[no_mangle]
/// Handler for segment not present trap
pub extern "C" fn do_segment_not_present() {
    dbg!()
}

#[no_mangle]
/// Handler for do stack segment trap
pub extern "C" fn do_stack_segment() {
    dbg!()
}

#[no_mangle]
/// Handler for general protection trap
pub extern "C" fn do_general_protection() {
    dbg!()
}

#[no_mangle]
/// Handler for page fault trap
pub extern "C" fn do_page_fault() {
    dbg!()
}

#[no_mangle]
/// Handler for dspurious interrupt trap
pub extern "C" fn do_spurious_interrupt_bug() {
    dbg!()
}

#[no_mangle]
/// Handler for coprocessor error trap
pub extern "C" fn do_coprocessor_error() {
    dbg!()
}

#[no_mangle]
/// Handler for alignment check trap
pub extern "C" fn do_alignment_check() {
    dbg!()
}

#[no_mangle]
/// Handler for SIMD coprocessor trap
pub extern "C" fn do_simd_coprocessor_error() {
    dbg!()
}

#[no_mangle]
/// Handler for hypervisor callback trap
pub extern "C" fn do_hypervisor_callback() {
    dbg!()
}
