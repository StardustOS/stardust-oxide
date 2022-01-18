//! Interface for Xen Grant Tables
//!
//! "The grant table mechanism [..] allows memory pages to be transferred or shared between virtual machines"

use {crate::hypercall, displaydoc::Display};

pub mod operations;

/// Grant table error
#[derive(Display, Debug)]
pub enum Error {
    /// Error from the hypercall in a grant table operation
    Hypercall(hypercall::Error),
    /// Error returned in status field of operation argument
    Operation(GrantStatusError),
}

impl From<hypercall::Error> for Error {
    fn from(e: hypercall::Error) -> Self {
        Self::Hypercall(e)
    }
}

impl From<GrantStatusError> for Error {
    fn from(e: GrantStatusError) -> Self {
        Self::Operation(e)
    }
}

/// Errors returned in the status field of grant table operation argument structs
#[derive(Debug, Display)]
pub enum GrantStatusError {
    /// General undefined error.
    GeneralError,
    /// Unrecognsed domain id.
    BadDomain,
    /// Unrecognised or inappropriate gntref.
    BadGntRef,
    /// Unrecognised or inappropriate handle.
    BadHandle,
    /// Inappropriate virtual address to map.
    BadVirtAddr,
    /// Inappropriate device address to unmap.
    BadDevAddr,
    /// Out of space in I/O MMU.
    NoDeviceSpace,
    /// Not enough privilege for operation.
    PermissionDenied,
    /// Specified page was invalid for op.
    BadPage,
    /// copy arguments cross page boundary.
    BadCopyArg,
    /// transfer page address too large.
    AddressTooBig,
    /// Operation not done; try again.
    Eagain,
    /// Out of space (handles etc).
    NoSpace,
}

impl From<i16> for GrantStatusError {
    fn from(status: i16) -> Self {
        match status {
            -1 => Self::GeneralError,
            -2 => Self::BadDomain,
            -3 => Self::BadGntRef,
            -4 => Self::BadHandle,
            -5 => Self::BadVirtAddr,
            -6 => Self::BadDevAddr,
            -7 => Self::NoDeviceSpace,
            -8 => Self::PermissionDenied,
            -9 => Self::BadPage,
            -10 => Self::BadCopyArg,
            -11 => Self::AddressTooBig,
            -12 => Self::Eagain,
            -13 => Self::NoSpace,
            _ => panic!("unknown status"),
        }
    }
}
