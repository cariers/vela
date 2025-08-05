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
        InboundStreamHandler, InboundUpgradeSend, NetworkBehavior, NetworkIncomingBehavior,
        StreamProtocol, SubstreamProtocol, THandlerAction, THandlerEvent,
    },
};

use crate::{Event, protocol};

type PongFuture = BoxFuture<'static, Result<protocol::ConnectInfo, io::Error>>;

pub struct Handler {
    info: protocol::ServerInfo,
    inbound: Option<PongFuture>,
    connected: bool,
}

impl Handler {
    pub fn new(info: protocol::ServerInfo) -> Self {
        Self {
            info,
            inbound: None,
            connected: false,
        }
    }
}

impl ConnectionHandler for Handler {
    type Action = Infallible;

    type Event = protocol::ConnectInfo;

    fn handle_action(&mut self, _action: Self::Action) {
        unreachable!("Connect handler does not support actions");
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ConnectionHandlerEvent<Self::Event>> {
        if self.connected {
            return Poll::Pending;
        }
        loop {
            if let Some(future) = self.inbound.as_mut() {
                match future.poll_unpin(cx) {
                    Poll::Ready(Ok(connect_info)) => {
                        tracing::info!("Received connection info: {:?}", connect_info);
                        self.inbound = None;
                        self.connected = true;
                        // 发送连接信息到上层
                        return Poll::Ready(ConnectionHandlerEvent::Notify(connect_info));
                    }
                    Poll::Ready(Err(e)) => {
                        self.inbound = None;
                        tracing::error!("Failed to read connection info: {:?}", e);
                        return Poll::Ready(ConnectionHandlerEvent::CloseConnection);
                    }
                    Poll::Pending => {}
                }
            }

            return Poll::Pending;
        }
    }
}

impl InboundStreamHandler for Handler {
    type InboundUpgrade = ReadyUpgrade<StreamProtocol>;

    type InboundUserData = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundUpgrade, Self::InboundUserData> {
        SubstreamProtocol::new(ReadyUpgrade::new(protocol::PROTOCOL_NAME), ())
    }

    fn on_fully_negotiated(
        &mut self,
        _user_data: Self::InboundUserData,
        stream: <Self::InboundUpgrade as InboundUpgradeSend>::Output,
    ) {
        self.inbound = Some(protocol::read_connection_info(stream, self.info.clone()).boxed());
    }

    fn on_upgrade_error(
        &mut self,
        _user_data: Self::InboundUserData,
        error: <Self::InboundUpgrade as InboundUpgradeSend>::Error,
    ) {
        tracing::error!(
            "Connect handler failed to upgrade inbound connection: {:?}",
            error
        );
        self.inbound = None;
    }
}

pub struct Behavior {
    info: protocol::ServerInfo,
    events: VecDeque<Event<protocol::ConnectInfo>>,
    none_event_waker: Option<Waker>,
}

impl Behavior {
    pub fn new(info: protocol::ServerInfo) -> Self {
        Self {
            info,
            events: VecDeque::new(),
            none_event_waker: None,
        }
    }
}

impl NetworkBehavior for Behavior {
    type ConnectionHandler = Handler;
    type Event = Event<protocol::ConnectInfo>;

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

impl NetworkIncomingBehavior for Behavior {
    /// 处理已建立的连接
    fn handle_established_connection(
        &mut self,
        _id: ConnectionId,
        peer_id: PeerId,
        _local_addr: &Url,
        _remote_addr: &Url,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        tracing::trace!("Connect handler established for peer: {}", peer_id);
        Ok(Handler::new(self.info.clone()))
    }
}
