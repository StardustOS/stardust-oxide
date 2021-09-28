//! Stardust Oxide

#![no_std]
#![feature(alloc_error_handler)]
#![deny(missing_docs)]

extern crate alloc;

use {
    alloc::vec::Vec,
    core::{slice, str},
    xen::{
        console::Writer,
        dbg, println,
        scheduler::{schedule_operation, Command, ShutdownReason},
        xen_sys::start_info_t,
    },
};

pub mod mm;
pub mod trap;

/// Launches the kernel with the supplied reference to the start_info structure.
pub fn launch(start_info: &start_info_t) {
    Writer::init(start_info);

    println!();
    println!("   _____ _____ _____ _____ _____ __ __ _____ _____ ");
    println!("  |   __|_   _|  _  |  _  |     |  |  |   __|_   _|");
    println!("  |__   | | | |     |    _| |   |     |__   | | |  ");
    println!("  |_____| |_| |__|__|__|__|_____|_____|_____| |_|  ");
    println!("                             █▀█ ▀▄▀ █ █▀▄ █▀▀     ");
    println!("                             █▄█ █ █ █ █▄▀ ██▄     ");
    println!();
    print_start_info(start_info);

    trap::init();
    mm::init(start_info);

    #[cfg(test)]
    test_main();

    let mut a = Vec::new();
    a.push(100i32);
    dbg!(a);

    unimplemented!("initialisation and idle loop")
}

fn print_start_info(start_info: &start_info_t) {
    let magic_str = str::from_utf8(unsafe {
        slice::from_raw_parts(start_info.magic.as_ptr() as *const u8, 32)
    })
    .unwrap();
    println!("    platform: {}", magic_str);
    println!("    nr_pages: {}", start_info.nr_pages);
    println!("    shared_info: {:#x}", start_info.shared_info);
    println!("    pt_base: {:#x}", start_info.pt_base);
    println!("    mod_start: {:#x}", start_info.mod_start);
    println!("    mod_len: {}", start_info.mod_len);
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);

    schedule_operation(Command::Shutdown(ShutdownReason::Crash));

    loop {}
}