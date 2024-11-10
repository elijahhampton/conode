use conode_protocol::event::NetworkEvent;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex as TokioMutex};

use crate::app::messages::Message;
pub struct EventStream {
    pub event_rx: Arc<TokioMutex<mpsc::Receiver<NetworkEvent>>>,
}

impl<H, I> iced_native::subscription::Recipe<H, I> for EventStream
where
    H: std::hash::Hasher,
{
    type Output = Message;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: iced_futures::BoxStream<I>,
    ) -> iced_futures::BoxStream<Self::Output> {
        let event_rx = self.event_rx;
        Box::pin(futures::stream::unfold((), move |_| {
            let event_rx = event_rx.clone();
            async move {
                let mut rx = event_rx.lock().await;
                match rx.recv().await {
                    Some(event) => Some((Message::NetworkEvent(event), ())),
                    None => None,
                }
            }
        }))
    }
}
