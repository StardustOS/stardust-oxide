#![no_std]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]

use xen::{
    console::Writer,
    dbg, println,
    scheduler::{schedule_operation, Command, ShutdownReason},
    xen_sys::start_info_t,
};

mod trap;

#[derive(Debug)]
struct Foo<'a> {
    a: i64,
    b: u8,
    c: &'a str,
}

#[no_mangle]
pub extern "C" fn start_kernel(start_info: *mut start_info_t) {
    Writer::init(unsafe { &*start_info });

    println!();
    println!("Initialising...                                    ");
    println!("   _____ _____ _____ _____ _____ __ __ _____ _____ ");
    println!("  |   __|_   _|  _  |  _  |     |  |  |   __|_   _|");
    println!("  |__   | | | |     |    _| |   |     |__   | | |  ");
    println!("  |_____| |_| |__|__|__|__|_____|_____|_____| |_|  ");
    println!("                             █▀█ ▀▄▀ █ █▀▄ █▀▀     ");
    println!("                             █▄█ █ █ █ █▄▀ ██▄     ");
    println!();

    trap::init();

    #[cfg(test)]
    test_main();

    let foo = Foo {
        a: -14351253,
        b: 0xFE,
        c: "example use of the dbg! macro😄",
    };
    dbg!(foo);

    panic_if_5(5);

    loop {}
}

fn panic_if_5(input: usize) {
    if input == 5 {
        panic!("input was 5");
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);

    schedule_operation(Command::Shutdown(ShutdownReason::Crash));

    loop {}
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());

    for test in tests {
        test();
    }

    println!("All tests passed");

    schedule_operation(Command::Shutdown(ShutdownReason::Poweroff));
}

#[cfg(test)]
mod test {
    #[test_case]
    fn example() {
        assert_eq!(2 + 2, 4);
    }
}
