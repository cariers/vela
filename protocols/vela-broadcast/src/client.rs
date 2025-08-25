mod handler;

use std::{
    collections::VecDeque,
    fmt, io,
    task::{Context, Poll},
};

use volans::{
    codec::Decoder,
    core::{Multiaddr, PeerId},
    swarm::{
        BehaviorEvent, ConnectionDenied, ConnectionId, NetworkBehavior, NetworkOutgoingBehavior,
        StreamProtocol, THandlerAction, THandlerEvent,
    },
};

#[derive(Debug)]
pub struct Behavior<TCodec>
where
    TCodec: Decoder + Clone + Send + 'static,
    TCodec::Item: fmt::Debug + Send,
    TCodec::Error: fmt::Debug + Send,
    TCodec::Error: Into<io::Error>,
{
    protocol: StreamProtocol,
    codec: TCodec,
    pending_events: VecDeque<BroadcastEvent<TCodec::Item>>,
}

impl<TCodec> Behavior<TCodec>
where
    TCodec: Decoder + Clone + Send + 'static,
    TCodec::Item: fmt::Debug + Send,
    TCodec::Error: fmt::Debug + Send,
    TCodec::Error: Into<io::Error>,
{
    pub fn new(protocol: StreamProtocol, codec: TCodec) -> Self {
        Self {
            protocol,
            codec,
            pending_events: VecDeque::new(),
        }
    }
}

impl<TCodec> NetworkBehavior for Behavior<TCodec>
where
    TCodec: Decoder + Clone + Send + 'static,
    TCodec::Item: fmt::Debug + Send,
    TCodec::Error: fmt::Debug + Send,
    TCodec::Error: Into<io::Error>,
{
    type Event = BroadcastEvent<TCodec::Item>;
    type ConnectionHandler = handler::Handler<TCodec>;

    fn on_connection_handler_event(
        &mut self,
        connection_id: ConnectionId,
        peer_id: PeerId,
        event: THandlerEvent<Self>,
    ) {
        let event = match event {
            handler::Event::Closed => BroadcastEvent::Closed {
                peer_id,
                connection_id,
            },
            handler::Event::Unsupported => BroadcastEvent::Unsupported {
                peer_id,
                connection_id,
                protocol: self.protocol.clone(),
            },
            handler::Event::Message(message) => BroadcastEvent::Message {
                peer_id,
                connection_id,
                message,
            },
            handler::Event::IoError(error) => BroadcastEvent::IoError {
                peer_id,
                connection_id,
                error,
            },
        };
        self.pending_events.push_back(event);
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<BehaviorEvent<Self::Event, THandlerAction<Self>>> {
        loop {
            if let Some(event) = self.pending_events.pop_front() {
                return Poll::Ready(BehaviorEvent::Behavior(event));
            }
            return Poll::Pending;
        }
    }
}

impl<TCodec> NetworkOutgoingBehavior for Behavior<TCodec>
where
    TCodec: Decoder + Clone + Send + 'static,
    TCodec::Item: fmt::Debug + Send,
    TCodec::Error: fmt::Debug + Send,
    TCodec::Error: Into<io::Error>,
{
    fn handle_established_connection(
        &mut self,
        _id: ConnectionId,
        _peer_id: PeerId,
        _addr: &Multiaddr,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        Ok(handler::Handler::new(
            self.protocol.clone(),
            self.codec.clone(),
        ))
    }
}

#[derive(Debug)]
pub enum BroadcastEvent<T> {
    Message {
        peer_id: PeerId,
        connection_id: ConnectionId,
        message: T,
    },
    Closed {
        peer_id: PeerId,
        connection_id: ConnectionId,
    },
    Unsupported {
        peer_id: PeerId,
        connection_id: ConnectionId,
        protocol: StreamProtocol,
    },
    IoError {
        peer_id: PeerId,
        connection_id: ConnectionId,
        error: io::Error,
    },
}
