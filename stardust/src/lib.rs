#![no_std]

use xen::xen_sys::start_info_t;

mod console;

extern "C" {
    fn console_init(start_info: *mut start_info_t);
}

#[derive(Debug)]
struct Foo<'a> {
    a: i64,
    b: u8,
    c: &'a str,
}

#[no_mangle]
pub extern "C" fn start_kernel(start_info: *mut start_info_t) {
    unsafe {
        console_init(start_info);
    };

    println!();
    println!("Initialising...                                    ");
    println!("   _____ _____ _____ _____ _____ __ __ _____ _____ ");
    println!("  |   __|_   _|  _  |  _  |     |  |  |   __|_   _|");
    println!("  |__   | | | |     |    _| |   |     |__   | | |  ");
    println!("  |_____| |_| |__|__|__|__|_____|_____|_____| |_|  ");
    println!("                             █▀█ ▀▄▀ █ █▀▄ █▀▀     ");
    println!("                             █▄█ █ █ █ █▄▀ ██▄     ");

    println!("test");
    println!();

    dbg!(Foo {
        a: -14351253,
        b: 0xFE,
        c: "test!:)"
    });

    panic_if_5(5);

    loop {}
}

/// Panics if input == 5 otherwise returns
fn panic_if_5(input: usize) {
    if input == 5 {
        panic!("input was 5");
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
