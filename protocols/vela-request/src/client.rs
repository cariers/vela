use std::task::{Context, Poll};

use volans::{
    core::{PeerId, Url},
    request::{Config, RequestId, client},
    swarm::{
        BehaviorEvent, ConnectionDenied, ConnectionId, DialOpts, NetworkBehavior,
        NetworkOutgoingBehavior, StreamProtocol, THandlerAction, THandlerEvent,
        error::{ConnectionError, DialError},
    },
};

use crate::{Codec, Request, Response};

pub use client::Event;
pub type Handler<TRequest, TResponse> = client::Handler<Codec<TRequest, TResponse>>;

pub struct Behavior<TRequest, TResponse>
where
    TRequest: prost::Message + Default + Send + Clone + 'static,
    TResponse: prost::Message + Default + Send + Clone + 'static,
{
    inner: client::Behavior<Codec<TRequest, TResponse>>,
}

impl<TRequest, TResponse> Behavior<TRequest, TResponse>
where
    TRequest: prost::Message + Default + Send + Clone + 'static,
    TResponse: prost::Message + Default + Send + Clone + 'static,
{
    pub fn new(config: Config) -> Self {
        let codec = Codec::<TRequest, TResponse>::new();
        Self {
            inner: client::Behavior::with_codec(codec, config),
        }
    }

    pub fn send_request(
        &mut self,
        peer_id: PeerId,
        protocol: StreamProtocol,
        request: Request<TRequest>,
    ) -> RequestId {
        self.inner.send_request(peer_id, protocol, request)
    }
}

impl<TRequest, TResponse> NetworkBehavior for Behavior<TRequest, TResponse>
where
    TRequest: prost::Message + Default + Send + Clone + 'static,
    TResponse: prost::Message + Default + Send + Clone + 'static,
{
    type Event = Event<Response<TResponse>>;
    type ConnectionHandler = Handler<TRequest, TResponse>;

    fn on_connection_handler_event(
        &mut self,
        id: ConnectionId,
        peer_id: PeerId,
        event: THandlerEvent<Self>,
    ) {
        self.inner.on_connection_handler_event(id, peer_id, event);
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<BehaviorEvent<Self::Event, THandlerAction<Self>>> {
        self.inner.poll(cx)
    }
}

impl<TRequest, TResponse> NetworkOutgoingBehavior for Behavior<TRequest, TResponse>
where
    TRequest: prost::Message + Default + Send + Clone + 'static,
    TResponse: prost::Message + Default + Send + Clone + 'static,
{
    fn handle_established_connection(
        &mut self,
        id: ConnectionId,
        peer_id: PeerId,
        addr: &Url,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        self.inner.handle_established_connection(id, peer_id, addr)
    }

    fn on_connection_established(&mut self, id: ConnectionId, peer_id: PeerId, addr: &Url) {
        self.inner.on_connection_established(id, peer_id, addr);
    }

    fn on_connection_closed(
        &mut self,
        id: ConnectionId,
        peer_id: PeerId,
        addr: &Url,
        reason: Option<&ConnectionError>,
    ) {
        self.inner.on_connection_closed(id, peer_id, addr, reason);
    }

    fn on_dial_failure(
        &mut self,
        id: ConnectionId,
        peer_id: Option<PeerId>,
        addr: Option<&Url>,
        error: &DialError,
    ) {
        self.inner.on_dial_failure(id, peer_id, addr, error);
    }

    fn poll_dial(&mut self, cx: &mut Context<'_>) -> Poll<DialOpts> {
        self.inner.poll_dial(cx)
    }
}
