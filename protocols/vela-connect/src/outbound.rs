use std::{
    collections::VecDeque,
    convert::Infallible,
    io,
    task::{Context, Poll, Waker},
};

use futures::{FutureExt, future::BoxFuture};
use volans::{
    core::{PeerId, Url, upgrade::ReadyUpgrade},
    swarm::{
        BehaviorEvent, ConnectionDenied, ConnectionHandler, ConnectionHandlerEvent, ConnectionId,
        NetworkBehavior, NetworkOutgoingBehavior, OutboundStreamHandler, OutboundUpgradeSend,
        StreamProtocol, StreamUpgradeError, SubstreamProtocol, THandlerAction, THandlerEvent,
    },
};

use crate::{Event, protocol};

type PongFuture = BoxFuture<'static, Result<protocol::ServerInfo, io::Error>>;

pub struct Handler {
    info: protocol::ConnectInfo,
    outbound: Option<PongFuture>,
    connecting: bool,
    connected: bool,
}

impl Handler {
    pub fn new(info: protocol::ConnectInfo) -> Self {
        Self {
            info,
            outbound: None,
            connecting: false,
            connected: false,
        }
    }
}

impl ConnectionHandler for Handler {
    type Action = Infallible;
    type Event = protocol::ServerInfo;

    fn handle_action(&mut self, _action: Self::Action) {
        unreachable!("Connect handler does not support actions");
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ConnectionHandlerEvent<Self::Event>> {
        loop {
            if let Some(future) = self.outbound.as_mut() {
                match future.poll_unpin(cx) {
                    Poll::Ready(Ok(info)) => {
                        self.outbound = None;
                        self.connected = true;
                        return Poll::Ready(ConnectionHandlerEvent::Notify(info));
                    }
                    Poll::Ready(Err(e)) => {
                        tracing::error!("Failed to read server info: {:?}", e);
                        self.outbound = None;
                        return Poll::Ready(ConnectionHandlerEvent::CloseConnection);
                    }
                    Poll::Pending => {}
                }
            }
            return Poll::Pending;
        }
    }
}

impl OutboundStreamHandler for Handler {
    type OutboundUpgrade = ReadyUpgrade<StreamProtocol>;
    type OutboundUserData = ();

    fn on_fully_negotiated(
        &mut self,
        _user_data: Self::OutboundUserData,
        stream: <Self::OutboundUpgrade as OutboundUpgradeSend>::Output,
    ) {
        self.connecting = false;
        self.outbound = Some(protocol::read_server_info(stream, self.info.clone()).boxed());
    }

    fn on_upgrade_error(
        &mut self,
        _user_data: Self::OutboundUserData,
        error: StreamUpgradeError<<Self::OutboundUpgrade as OutboundUpgradeSend>::Error>,
    ) {
        tracing::info!("Connect upgrade error: {:?}", error);
        self.connecting = false;
    }

    fn poll_outbound_request(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<SubstreamProtocol<Self::OutboundUpgrade, Self::OutboundUserData>> {
        if !self.connected && self.outbound.is_none() && !self.connecting {
            let protocol = SubstreamProtocol::new(ReadyUpgrade::new(protocol::PROTOCOL_NAME), ());
            self.connecting = true;
            return Poll::Ready(protocol);
        }
        Poll::Pending
    }
}

pub struct Behavior {
    info: protocol::ConnectInfo,
    events: VecDeque<Event<protocol::ServerInfo>>,
    none_event_waker: Option<Waker>,
}

impl Behavior {
    pub fn new(info: protocol::ConnectInfo) -> Self {
        Self {
            info,
            events: VecDeque::new(),
            none_event_waker: None,
        }
    }
}

impl NetworkBehavior for Behavior {
    type ConnectionHandler = Handler;
    type Event = Event<protocol::ServerInfo>;

    fn on_connection_handler_event(
        &mut self,
        id: ConnectionId,
        peer_id: PeerId,
        event: THandlerEvent<Self>,
    ) {
        self.events.push_front(Event {
            peer_id,
            connection: id,
            info: event,
        });
        if let Some(waker) = self.none_event_waker.take() {
            waker.wake();
        }
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<BehaviorEvent<Self::Event, THandlerAction<Self>>> {
        if let Some(event) = self.events.pop_back() {
            return Poll::Ready(BehaviorEvent::Behavior(event));
        }
        self.none_event_waker = Some(_cx.waker().clone());
        Poll::Pending
    }
}

impl NetworkOutgoingBehavior for Behavior {
    fn handle_established_connection(
        &mut self,
        id: ConnectionId,
        peer_id: PeerId,
        addr: &Url,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        tracing::trace!(
            "Connect handler established for peer: {}, {}, {}",
            id,
            peer_id,
            addr
        );
        Ok(Handler::new(self.info.clone()))
    }
}
