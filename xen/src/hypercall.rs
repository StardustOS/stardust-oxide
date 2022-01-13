//! Platform-agnostic hypercall interface

use {
    core::{convert::TryInto, result::Result},
    displaydoc::Display,
};

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

/// Hypercall Error
#[derive(Debug, Display)]
pub enum Error {
    /// Operation not permitted
    PERM,
    /// No such file or directory
    NOENT,
    /// No such process
    SRCH,
    /// Interrupted system call. Internal only, should never be exposed to the guest.
    INTR,
    /// I/O error
    IO,
    /// No such device or address
    NXIO,
    /// Arg list too long
    TOOBIG,
    /// Exec format error
    NOEXEC,
    /// Bad file number
    BADF,
    /// No child processes
    CHILD,
    /// Operation would block
    WOULDBLOCK,
    /// Out of memory
    NOMEM,
    /// Permission denied
    ACCES,
    /// Bad address
    FAULT,
    /// Device or resource busy
    BUSY,
    /// File exists
    EXIST,
    /// Cross-device link
    XDEV,
    /// No such device
    NODEV,
    /// Not a directory
    NOTDIR,
    /// Is a directory
    ISDIR,
    /// Invalid argument
    INVAL,
    /// File table overflow
    NFILE,
    /// Too many open files
    MFILE,
    /// No space left on device
    NOSPC,
    /// Read-only file system
    ROFS,
    /// Too many links
    MLINK,
    /// Math argument out of domain of func
    DOM,
    /// Math result not representable
    RANGE,
    /// Resource deadlock would occur
    DEADLOCK,
    /// File name too long
    NAMETOOLONG,
    /// No record locks available
    NOLCK,
    /// Function not implemented
    NOSYS,
    /// Directory not empty
    NOTEMPTY,
    /// No data available
    NODATA,
    /// Timer expired
    TIME,
    /// Not a data message
    BADMSG,
    /// Value too large for defined data type
    OVERFLOW,
    /// Illegal byte sequence
    ILSEQ,
    /// Interrupted system call should be restarted. Internal only, should never be exposed to the guest.
    RESTART,
    /// Socket operation on non-socket
    NOTSOCK,
    /// Message too large.
    MSGSIZE,
    /// Operation not supported on transport endpoint
    OPNOTSUPP,
    /// Address already in use
    ADDRINUSE,
    /// Cannot assign requested address
    ADDRNOTAVAIL,
    /// No buffer space available
    NOBUFS,
    /// Transport endpoint is already connected
    ISCONN,
    /// Transport endpoint is not connected
    NOTCONN,
    /// Connection timed out
    TIMEDOUT,
    /// Connection refused
    CONNREFUSED,
    /// Unknown error
    Unknown(i64),
}

impl From<i64> for Error {
    fn from(errno: i64) -> Self {
        match errno {
            0.. => panic!("ERRNO must be negative"),
            -1 => Self::PERM,
            -2 => Self::NOENT,
            -3 => Self::SRCH,
            -4 => Self::INTR,
            -5 => Self::IO,
            -6 => Self::NXIO,
            -7 => Self::TOOBIG,
            -8 => Self::NOEXEC,
            -9 => Self::BADF,
            -10 => Self::CHILD,
            -11 => Self::WOULDBLOCK,
            -12 => Self::NOMEM,
            -13 => Self::ACCES,
            -14 => Self::FAULT,
            -16 => Self::BUSY,
            -17 => Self::EXIST,
            -18 => Self::XDEV,
            -19 => Self::NODEV,
            -20 => Self::NOTDIR,
            -21 => Self::ISDIR,
            -22 => Self::INVAL,
            -23 => Self::NFILE,
            -24 => Self::MFILE,
            -28 => Self::NOSPC,
            -30 => Self::ROFS,
            -31 => Self::MLINK,
            -33 => Self::DOM,
            -34 => Self::RANGE,
            -35 => Self::DEADLOCK,
            -36 => Self::NAMETOOLONG,
            -37 => Self::NOLCK,
            -38 => Self::NOSYS,
            -39 => Self::NOTEMPTY,
            -61 => Self::NODATA,
            -62 => Self::TIME,
            -74 => Self::BADMSG,
            -75 => Self::OVERFLOW,
            -84 => Self::ILSEQ,
            -85 => Self::RESTART,
            -88 => Self::NOTSOCK,
            -90 => Self::MSGSIZE,
            -95 => Self::OPNOTSUPP,
            -98 => Self::ADDRINUSE,
            -99 => Self::ADDRNOTAVAIL,
            -105 => Self::NOBUFS,
            -106 => Self::ISCONN,
            -107 => Self::NOTCONN,
            -110 => Self::TIMEDOUT,
            -111 => Self::CONNREFUSED,
            n => Self::Unknown(n),
        }
    }
}

/// Converts an error number result from a hypercall to a Result
pub(crate) fn errno_to_result(errno: i64) -> Result<u64, Error> {
    if errno >= 0 {
        Ok(errno.try_into().expect("Failed to convert i64 to u64"))
    } else {
        Err(Error::from(errno))
    }
}
