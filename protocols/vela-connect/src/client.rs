use std::{
    io,
    task::{Context, Poll},
};

use vela_core::authenticate::AuthError;
use vela_protobuf::connect::Info;
use vela_request::{Config, Request, client};
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
    type Event = Event;
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
        loop {
            match self.inner.poll(cx) {
                Poll::Ready(BehaviorEvent::Behavior(event)) => match event {
                    client::Event::Failure {
                        peer_id,
                        connection_id,
                        request_id: _,
                        cause,
                    } => {
                        tracing::error!("身份验证错误 {}: {:?}", peer_id, cause);
                        return Poll::Ready(BehaviorEvent::CloseConnection {
                            peer_id: peer_id,
                            connection: volans::swarm::behavior::CloseConnection::One(
                                connection_id,
                            ),
                        });
                    }
                    client::Event::Response {
                        peer_id,
                        connection_id,
                        request_id: _,
                        response,
                    } => match response.into_payload() {
                        Ok(info) => {
                            return Poll::Ready(BehaviorEvent::Behavior(Event::Authenticated {
                                peer_id: peer_id,
                                connection_id: connection_id,
                                info: info,
                            }));
                        }
                        Err(status) => {
                            tracing::warn!("身份验证失败 {}: {:?}", peer_id, status);
                            return Poll::Ready(BehaviorEvent::CloseConnection {
                                peer_id: peer_id,
                                connection: volans::swarm::behavior::CloseConnection::One(
                                    connection_id,
                                ),
                            });
                        }
                    },
                },
                Poll::Ready(BehaviorEvent::HandlerAction {
                    peer_id,
                    handler,
                    action,
                }) => {
                    return Poll::Ready(BehaviorEvent::HandlerAction {
                        peer_id,
                        handler,
                        action,
                    });
                }
                Poll::Ready(BehaviorEvent::CloseConnection {
                    peer_id,
                    connection,
                }) => {
                    return Poll::Ready(BehaviorEvent::CloseConnection {
                        peer_id,
                        connection,
                    });
                }
                Poll::Pending => {}
                _ => unreachable!("Unexpected event"),
            }
            return Poll::Pending;
        }
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

/// 行为事件枚举，表示认证过程中可能发生的事件。
#[derive(Debug)]
pub enum Event {
    /// 认证成功。
    Authenticated {
        /// 对端节点 ID。
        peer_id: PeerId,
        /// 连接 ID。
        connection_id: ConnectionId,
        /// 玩家 ID。
        info: Info,
    },
    /// 认证失败。
    Unauthenticated {
        /// 对端节点 ID。
        peer_id: PeerId,
        /// 连接 ID。
        connection_id: ConnectionId,
        /// 认证失败的原因。
        cause: AuthError,
    },
    /// 认证过程中发生错误。
    AuthenticateFailure {
        /// 对端节点 ID。
        peer_id: PeerId,
        /// 连接 ID。
        connection_id: ConnectionId,
        /// 错误信息。
        error: io::Error,
    },
}
