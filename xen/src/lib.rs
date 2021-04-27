//! Interface with Xen hypervisor

#![no_std]
#![feature(asm)]
#![deny(missing_docs)]

pub use xen_sys;

//pub mod console;
pub mod platform;
pub mod sched;
