use {
    fast_async_mutex::mutex::{Mutex, MutexOwnedGuard, MutexOwnedGuardFuture},
    futures::FutureExt,
    std::{
        sync::Arc,
        task::{Context, Poll},
    },
};

/// A mutex that's easier to use when writing low-level [`Future`][std::future::Future]s,
/// [`Stream`][futures::Stream]s and [`Sink`][futures::Sink]s.
///
/// This mutex automatically wraps it's data in an `Arc`, so that it can be cheaply cloned. It also
/// keeps track of whether it is attempting to acquire the lock or not, so it is easy to
/// continuously poll it to obtain the lock.
#[derive(Debug)]
pub struct MutexStateMachine<T> {
    data: Arc<Mutex<T>>,
    lock_future: Option<MutexOwnedGuardFuture<T>>,
}

impl<T> MutexStateMachine<T> {
    /// Create a new mutex to protect the specified `data`.
    pub fn new(data: T) -> Self {
        MutexStateMachine {
            data: Arc::new(Mutex::new(data)),
            lock_future: None,
        }
    }

    /// Attempt to acquire the lock.
    ///
    /// If the attempt fails, returns [`Poll::Pending`] and schedules the current task to wake up
    /// when the lock becomes available so that this method can be called again. Once it is
    /// acquired, a [`MutexOwnedGuard`] is returned.
    pub fn poll_lock(&mut self, context: &mut Context<'_>) -> Poll<MutexOwnedGuard<T>> {
        let data = &self.data;
        let lock_future = self.lock_future.get_or_insert_with(|| data.lock_owned());

        let poll_result = lock_future.poll_unpin(context);

        if poll_result.is_ready() {
            self.lock_future = None;
        }

        poll_result
    }
}

/// Clone this mutex, allowing more than one instance to try to acquire the lock.
///
/// The cloned instance is in a completely separate state, as if `poll_lock` had never been called.
/// This means that it is not initially attempting to acquire the lock.
impl<T> Clone for MutexStateMachine<T> {
    fn clone(&self) -> Self {
        MutexStateMachine {
            data: self.data.clone(),
            lock_future: None,
        }
    }
}
