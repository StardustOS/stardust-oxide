//! Futures executor for cooperative multitasking

use {
    alloc::collections::VecDeque,
    core::{
        future::Future,
        task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    },
    task::Task,
};

mod delay;
mod task;

pub use delay::Delay;

/// Basic executor for async tasks
pub struct Executor {
    tasks: VecDeque<Task>,
}

impl Executor {
    /// Create new empty Executor
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
        }
    }

    /// Spawn a new task on the executor
    pub fn spawn(&mut self, fut: impl Future<Output = ()> + 'static) {
        self.tasks.push_back(Task::new(fut))
    }

    /// Run the executor, polling tasks repeatedly
    pub fn run(&mut self) {
        while let Some(mut task) = self.tasks.pop_front() {
            let waker = new_waker();
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {}
                Poll::Pending => self.tasks.push_back(task),
            }
        }
    }
}

/// Create a new dummy RawWaker
fn new_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        new_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(0 as *const (), vtable)
}

/// Create a new dummy Waker
fn new_waker() -> Waker {
    unsafe { Waker::from_raw(new_raw_waker()) }
}
