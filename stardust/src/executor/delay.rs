use {
    core::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    },
    xen::platform::time::get_system_time,
};

/// Future for delaying asynchronous execution for the supplied Duration
///
/// This is a naive implementation and will cause executor to busy wait until the duration passes.
/// However does not prevent other tasks from being run so has some demonstration and debug usefulness.
pub struct Delay {
    // timestamp after which the delay expires
    expiration_timestamp: u64,
}

impl Delay {
    /// Creates a new future that will await for the supplied duration
    ///
    /// Supplied duration in nanoseconds must be less than 584.6 years.
    pub fn new(duration: Duration) -> Self {
        Self {
            expiration_timestamp: get_system_time() + (duration.as_nanos() as u64),
        }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        if get_system_time() < self.expiration_timestamp {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
