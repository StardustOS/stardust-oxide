//! Platform-agnostic hypercall interface

/// Software trap from a domain to the hypervisor used to request privileged operations
#[macro_export]
macro_rules! hypercall {
    ($offset:expr) => {
        $crate::platform::hypercall::hypercall0($offset)
    };
    ($offset:expr, $arg0:expr) => {
        $crate::platform::hypercall::hypercall1($offset, u64::from($arg0))
    };
    ($offset:expr, $arg0:expr, $arg1:expr) => {
        $crate::platform::hypercall::hypercall2($offset, u64::from($arg0), u64::from($arg1))
    };
    ($offset:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {
        $crate::platform::hypercall::hypercall3(
            $offset,
            u64::from($arg0),
            u64::from($arg1),
            u64::from($arg2),
        )
    };
    ($offset:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        $crate::platform::hypercall::hypercall4(
            $offset,
            u64::from($arg0),
            u64::from($arg1),
            u64::from($arg2),
            u64::from($arg3),
        )
    };
    ($offset:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {
        $crate::platform::hypercall::hypercall5(
            $offset,
            u64::from($arg0),
            u64::from($arg1),
            u64::from($arg2),
            u64::from($arg3),
            u64::from($arg4),
        )
    };
}
