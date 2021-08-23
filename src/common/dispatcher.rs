use {
    crate::util::MutexStateMachine,
    async_oneshot::Sender,
    fast_async_mutex::mutex::MutexOwnedGuard,
    futures::{ready, Sink},
    std::{
        collections::HashMap,
        hash::Hash,
        pin::Pin,
        task::{Context, Poll},
    },
};

/// A dispatcher of received responses.
///
/// The responses are routed according to an ID type. Pending requests are added by using the
/// [`Dispatcher`] as a [`Sink`] of tuples of an ID and a [`Sender`] endpoint of a `Response`. The
/// [`Receiver`][async_oneshot::Receiver] endpoint will be resolved when the corresponding
/// `Response` is dispatched.
///
/// When the [`Dispatcher`] is used as a [`Sink`] of tuples of an ID and a `Response`, the
/// `Response` is sent to the endpoint for the respective ID.
///
/// In order to use the [`Dispatcher`] as both types of [`Sink`]s, it can be cheaply cloned, and
/// one instance can be used for each case, since the data is stored and shared through an
/// [`Arc`][std::sync::Arc] internally.
#[derive(Debug)]
pub struct Dispatcher<Id, Response> {
    pending_requests: MutexStateMachine<HashMap<Id, Sender<Response>>>,
    guard: Option<MutexOwnedGuard<HashMap<Id, Sender<Response>>>>,
}

impl<Id, Response> Dispatcher<Id, Response> {
    pub fn new() -> Self {
        Dispatcher {
            pending_requests: MutexStateMachine::new(HashMap::new()),
            guard: None,
        }
    }
}

impl<Id, Response> Clone for Dispatcher<Id, Response> {
    fn clone(&self) -> Self {
        Dispatcher {
            pending_requests: self.pending_requests.clone(),
            guard: None,
        }
    }
}

impl<Id, Response> Sink<(Id, Sender<Response>)> for Dispatcher<Id, Response>
where
    Id: Eq + Hash,
{
    type Error = ();

    fn poll_ready(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let mut this = self.as_mut();

        if this.guard.is_none() {
            let guard = ready!(this.pending_requests.poll_lock(context));

            this.guard = Some(guard);
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        item: (Id, Sender<Response>),
    ) -> Result<(), Self::Error> {
        let (id, sender) = item;

        self.as_mut()
            .guard
            .take()
            .expect("Attempt to send item without holding the pending requests lock")
            .insert(id, sender);

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl<Id, Response> Sink<(Id, Response)> for Dispatcher<Id, Response>
where
    Id: Eq + Hash,
{
    type Error = ();

    fn poll_ready(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let mut this = self.as_mut();

        if this.guard.is_none() {
            let guard = ready!(this.pending_requests.poll_lock(context));

            this.guard = Some(guard);
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: (Id, Response)) -> Result<(), Self::Error> {
        let (id, response) = item;

        let maybe_sender = self
            .as_mut()
            .guard
            .take()
            .expect("Attempt to send item without holding the pending requests lock")
            .remove(&id);

        if let Some(mut sender) = maybe_sender {
            let _ = sender.send(response);
        }

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
