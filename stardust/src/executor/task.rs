//! Executable asynchronous task

use {
    alloc::boxed::Box,
    core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
};

/// Wrapper around a pinned, boxed future
pub struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    /// Create a new task from a future
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            future: Box::pin(future),
        }
    }

    pub fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}
