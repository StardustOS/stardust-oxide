#![no_std]
#![no_main]
#![test_runner(crate::test_runner)]

use xen::xen_sys::start_info_t;

#[no_mangle]
pub extern "C" fn start_kernel(start_info: *mut start_info_t) {
    // we have liftoff
    stardust::launch(start_info)
}
