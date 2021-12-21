//! Stardust Oxide

#![no_std]
#![feature(alloc_error_handler)]
#![deny(missing_docs)]

extern crate alloc;

use {
    core::{slice, str},
    log::{debug, error},
    xen::{
        console::Writer,
        init_info, println,
        scheduler::{schedule_operation, Command, ShutdownReason},
        sections::{edata, end, erodata, etext, text_start},
        xen_sys::start_info_t,
    },
};

pub mod logger;
pub mod mm;
pub mod trap;

#[cfg(feature = "test")]
mod test;

/// Launches the kernel with the supplied reference to the start_info structure.
pub fn launch(start_info: *mut start_info_t) {
    init_info(start_info);

    // SAFETY: safe to dereference raw pointer as it is valid when provided by Xen
    let start_info = unsafe { &*start_info };

    Writer::init(start_info);
    logger::init();

    println!();
    println!("   _____ _____ _____ _____ _____ __ __ _____ _____ ");
    println!("  |   __|_   _|  _  |  _  |     |  |  |   __|_   _|");
    println!("  |__   | | | |     |    _| |   |     |__   | | |  ");
    println!("  |_____| |_| |__|__|__|__|_____|_____|_____| |_|  ");
    println!("                             █▀█ ▀▄▀ █ █▀▄ █▀▀     ");
    println!("                             █▄█ █ █ █ █▄▀ ██▄     ");
    println!();
    print_start_info(start_info);

    trap::init();
    mm::init(start_info);

    #[cfg(feature = "test")]
    test::tests();

    unimplemented!("initialisation and idle loop")
}

fn print_start_info(start_info: &start_info_t) {
    let magic_str = str::from_utf8(unsafe {
        slice::from_raw_parts(start_info.magic.as_ptr() as *const u8, 32)
    })
    .unwrap();

    debug!("   platform: {}", magic_str);
    debug!("  domain ID: {}", xen::xenstore::domain_id());
    debug!("   nr_pages: {}", start_info.nr_pages);
    debug!("shared_info: {:#X}", start_info.shared_info);
    debug!("    pt_base: {:#X}", start_info.pt_base);
    debug!("   mfn_list: {:#X}", start_info.mfn_list);
    debug!("  mod_start: {:#X}", start_info.mod_start);
    debug!("    mod_len: {}", start_info.mod_len);
    debug!("      _text: {:#X}", text_start());
    debug!("     _etext: {:#X}", etext());
    debug!("   _erodata: {:#X}", erodata());
    debug!("     _edata: {:#X}", edata());
    debug!("       _end: {:#X}", end());
    debug!("");
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);

    Writer::flush();

    schedule_operation(Command::Shutdown(ShutdownReason::Crash));

    loop {}
}
