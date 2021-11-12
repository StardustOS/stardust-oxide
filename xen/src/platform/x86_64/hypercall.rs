//! x86_64 hypercall functions

use cty::c_long;

#[repr(C)]
struct HypercallEntry([u8; 32]);

extern "C" {
    static hypercall_page: [HypercallEntry; 128];
}

/// Makes hypercall with 0 arguments
///
/// # Safety
///
/// `offset` must be a valid offset into `hypercall_page`
pub unsafe fn hypercall0(offset: u32) -> c_long {
    let mut res: c_long;
    let entry = hypercall_page.as_ptr().offset(offset as isize);

    asm!("call {}",in(reg) entry,

    // All caller-saved registers must be marked as clobberred
    out("rax") res, out("rcx") _, out("rdx") _, out("rsi") _,
    out("r8") _, out("r9") _, out("r10") _, out("r11") _,
    out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
    out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
    out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
    out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _);

    res
}

/// Makes hypercall with 1 argument
///
/// # Safety
///
/// `offset` must be a valid offset into `hypercall_page`
pub unsafe fn hypercall1(offset: u32, arg0: u64) -> c_long {
    let mut res: c_long;
    let entry = hypercall_page.as_ptr().offset(offset as isize);

    asm!("call {}",in(reg) entry,

    inout("rdi") arg0 => _,

    // all caller-saved registers must be marked as clobberred
    out("rax") res, out("rcx") _, out("rdx") _, out("rsi") _,
    out("r8") _, out("r9") _, out("r10") _, out("r11") _,
    out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
    out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
    out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
    out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _);

    res
}

/// Makes hypercall with 2 arguments
///
/// # Safety
///
/// `offset` must be a valid offset into `hypercall_page`
pub unsafe fn hypercall2(offset: u32, arg0: u64, arg1: u64) -> c_long {
    let mut res: c_long;
    let entry = hypercall_page.as_ptr().offset(offset as isize);

    asm!("call {}",in(reg) entry,

    inout("rdi") arg0 => _,
    inout("rsi") arg1 => _,

    // all caller-saved registers must be marked as clobberred
    out("rax") res, out("rcx") _, out("rdx") _,
    out("r8") _, out("r9") _, out("r10") _, out("r11") _,
    out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
    out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
    out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
    out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _);

    res
}

/// Makes hypercall with 3 arguments
///
/// # Safety
///
/// `offset` must be a valid offset into `hypercall_page`
pub unsafe fn hypercall3(offset: u32, arg0: u64, arg1: u64, arg2: u64) -> c_long {
    let mut res: c_long;
    let entry = hypercall_page.as_ptr().offset(offset as isize);

    asm!("call {}",in(reg) entry,

    inout("rdi") arg0 => _,
    inout("rsi") arg1 => _,
    inout("rdx") arg2 => _,

    // all caller-saved registers must be marked as clobberred
    out("rax") res, out("rcx") _,
    out("r8") _, out("r9") _, out("r10") _, out("r11") _,
    out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
    out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
    out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
    out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _);

    res
}

/// Makes hypercall with 4 arguments
///
/// # Safety
///
/// `offset` must be a valid offset into `hypercall_page`
pub unsafe fn hypercall4(offset: u32, arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> c_long {
    let mut res: c_long;
    let entry = hypercall_page.as_ptr().offset(offset as isize);

    asm!("call {}",in(reg) entry,

    inout("rdi") arg0 => _,
    inout("rsi") arg1 => _,
    inout("rdx") arg2 => _,
    inout("r10") arg3 => _,

    // all caller-saved registers must be marked as clobberred
    out("rax") res, out("rcx") _,
    out("r8") _, out("r9") _,  out("r11") _,
    out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
    out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
    out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
    out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _);

    res
}

/// Makes hypercall with 5 arguments
///
/// # Safety
///
/// `offset` must be a valid offset into `hypercall_page`
pub unsafe fn hypercall5(
    offset: u32,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
) -> c_long {
    let mut res: c_long;
    let entry = hypercall_page.as_ptr().offset(offset as isize);

    asm!("call {}",in(reg) entry,

    inout("rdi") arg0 => _,
    inout("rsi") arg1 => _,
    inout("rdx") arg2 => _,
    inout("r10") arg3 => _,
    inout("r9") arg4 => _,

    // all caller-saved registers must be marked as clobberred
    out("rax") res, out("rcx") _,
    out("r8") _, out("r11") _,
    out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
    out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
    out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
    out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _);

    res
}
