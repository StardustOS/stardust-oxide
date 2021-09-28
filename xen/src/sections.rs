//! Helper functions for section addresses

/// Returns the address of the start of the `.text` section
#[inline]
pub fn text_start() -> usize {
    extern "C" {
        static mut _text: u8;
    }

    unsafe { &_text as *const u8 as usize }
}

/// Returns the address of the start of the `.etext` section
#[inline]
pub fn etext() -> usize {
    extern "C" {
        static mut _etext: u8;
    }

    unsafe { &_etext as *const u8 as usize }
}

/// Returns the address of the start of the `.edata` section
#[inline]
pub fn erodata() -> usize {
    extern "C" {
        static mut _erodata: u8;
    }

    unsafe { &_erodata as *const u8 as usize }
}

/// Returns the address of the start of the `.edata` section
#[inline]
pub fn edata() -> usize {
    extern "C" {
        static mut _edata: u8;
    }

    unsafe { &_edata as *const u8 as usize }
}

/// Returns the address of the start of the `.end` section
#[inline]
pub fn end() -> usize {
    extern "C" {
        static mut _end: u8;
    }

    unsafe { &_end as *const u8 as usize }
}
