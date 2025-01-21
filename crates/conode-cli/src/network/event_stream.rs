use crate::Message;
use conode_types::sync::SyncEvent;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio_stream::Stream;

/// An [`EventStream`] for polling/receiving [`SyncEvent`].
pub struct EventStream {
    receiver: tokio::sync::watch::Receiver<SyncEvent>,
    changed: Option<
        Pin<Box<dyn Future<Output = Result<(), tokio::sync::watch::error::RecvError>> + Send>>,
    >,
}

impl EventStream {
    /// Create a new EventStream.
    pub fn new(receiver: tokio::sync::watch::Receiver<SyncEvent>) -> Self {
        Self {
            receiver,
            changed: None,
        }
    }
}

impl Stream for EventStream {
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(fut) = self.changed.as_mut() {
            match fut.as_mut().poll(cx) {
                Poll::Ready(Ok(())) => {
                    let event = (*self.receiver.borrow_and_update()).clone();
                    self.changed = None;
                    Poll::Ready(Some(Message::SyncEvent(event)))
                }
                Poll::Ready(Err(_)) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        } else {
            let mut receiver = self.receiver.clone();
            self.changed = Some(Box::pin(async move { receiver.changed().await }));
            Poll::Pending
        }
    }
}
