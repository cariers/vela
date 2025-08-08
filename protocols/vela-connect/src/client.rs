use std::task::{Context, Poll};

use vela_protobuf::connect::Info;
use vela_request::{Config, Request, Response, client};
use volans::{
    core::{PeerId, Url},
    request::RequestId,
    swarm::{
        BehaviorEvent, ConnectionDenied, ConnectionId, DialOpts, NetworkBehavior,
        NetworkOutgoingBehavior, THandlerAction, THandlerEvent,
        error::{ConnectionError, DialError},
    },
};

use crate::PROTOCOL_NAME;

pub struct Behavior {
    info: Info,
    inner: client::Behavior<Info, Info>,
}

impl Behavior {
    pub fn new(info: Info, config: Config) -> Self {
        Self {
            info,
            inner: client::Behavior::new(config),
        }
    }

    pub fn send_authentication(&mut self, peer_id: PeerId, token: String) -> RequestId {
        let mut request = Request::new("vela.connect.authenticate".to_string(), self.info.clone());
        request.add_metadata("x-token".to_string(), token);
        self.inner.send_request(peer_id, PROTOCOL_NAME, request)
    }
}

impl NetworkBehavior for Behavior {
    type Event = client::Event<Response<Info>>;
    type ConnectionHandler = client::Handler<Info, Info>;

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

impl NetworkOutgoingBehavior for Behavior {
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
