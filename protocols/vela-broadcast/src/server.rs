use std::{
    fmt,
    task::{Context, Poll},
};

use async_broadcast::{Receiver, Sender};
use volans::{
    codec::Encoder,
    core::{Multiaddr, PeerId},
    swarm::{
        BehaviorEvent, ConnectionDenied, ConnectionId, NetworkBehavior, NetworkIncomingBehavior,
        StreamProtocol, THandlerAction,
    },
};

mod handler;

pub struct Behavior<TCodec>
where
    TCodec: Encoder + Clone + Send + 'static,
    TCodec::Item<'static>: Send + 'static,
    TCodec::Error: fmt::Debug + Send,
{
    protocol: StreamProtocol,
    sender: Sender<TCodec::Item<'static>>,
    receiver: Receiver<TCodec::Item<'static>>,
    codec: TCodec,
}

impl<TCodec> Behavior<TCodec>
where
    TCodec: Encoder + Clone + Send + 'static,
    TCodec::Item<'static>: Send + Clone + 'static,
    TCodec::Error: fmt::Debug + Send,
{
    pub fn new(protocol: StreamProtocol, codec: TCodec, buffer: usize) -> Self {
        let (sender, receiver) = async_broadcast::broadcast(buffer);
        Self {
            sender,
            receiver,
            protocol,
            codec,
        }
    }

    pub fn broadcaster(&self) -> Broadcaster<TCodec::Item<'static>> {
        Broadcaster {
            sender: self.sender.clone(),
        }
    }
}

impl<TCodec> NetworkBehavior for Behavior<TCodec>
where
    TCodec: Encoder + Clone + Send + 'static,
    TCodec::Item<'static>: fmt::Debug + Send + Clone + 'static,
    TCodec::Error: fmt::Debug + Send,
{
    type Event = ();
    type ConnectionHandler = handler::Handler<TCodec>;

    fn on_connection_handler_event(
        &mut self,
        _id: volans::swarm::ConnectionId,
        _peer_id: volans::core::PeerId,
        _event: volans::swarm::THandlerEvent<Self>,
    ) {
        unreachable!("This behavior does not generate events");
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<BehaviorEvent<Self::Event, THandlerAction<Self>>> {
        Poll::Pending
    }
}

impl<TCodec> NetworkIncomingBehavior for Behavior<TCodec>
where
    TCodec: Encoder + Clone + Send + 'static,
    TCodec::Item<'static>: fmt::Debug + Send + Clone + 'static,
    TCodec::Error: fmt::Debug + Send,
{
    /// 处理已建立的连接
    fn handle_established_connection(
        &mut self,
        _id: ConnectionId,
        _peer_id: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        Ok(handler::Handler::new(
            self.protocol.clone(),
            self.receiver.clone(),
            self.codec.clone(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct Broadcaster<T>
where
    T: Send + 'static,
{
    sender: Sender<T>,
}

impl<T> Broadcaster<T>
where
    T: Clone + Send + 'static,
{
    pub fn try_broadcast(&self, item: T) -> Result<(), T> {
        self.sender
            .try_broadcast(item)
            .map(|_| ())
            .map_err(|e| e.into_inner())
    }
}
