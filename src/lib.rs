#![no_std]
#![crate_type = "staticlib"]

extern "C" {
    fn console_write(message: *const i8) -> u32;
    fn console_flush();
}

/// Panics if input == 5 otherwise returns
#[no_mangle]
pub extern "C" fn panic_if_5(input: usize) {
    if input == 5 {
        panic!();
    }
}

/// Prints a message from Rust
#[no_mangle]
pub extern "C" fn hello() {
    let data = "Hello from Rust! 🦀\n\r\0";

    unsafe { console_write((&*data).as_ptr() as *const i8) };
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        console_write((&*"** EXAMPLE PANIC **\n\r\0").as_ptr() as *const i8);
        console_flush();
    };

    loop {}
}
