use std::{
    collections::{HashMap, VecDeque},
    io,
    task::{Context, Poll},
    time::Duration,
};

use futures::FutureExt;
use futures_bounded::{Delay, FuturesMap};
use vela_core::{
    authenticate::{AuthError, Authenticator},
    ids::{PlayerId, SessionId},
};
use vela_protobuf::{common::Code, connect::Info};
use vela_request::{Config, Request, RequestId, Responder, server};
use volans::{
    core::{PeerId, Url},
    swarm::{
        BehaviorEvent, ConnectionDenied, ConnectionId, ListenerEvent, NetworkBehavior,
        NetworkIncomingBehavior, THandlerAction, THandlerEvent,
        error::{ConnectionError, ListenError},
    },
};

use crate::PROTOCOL_NAME;

pub struct Behavior<TAuthenticator> {
    info: Info,
    authenticator: TAuthenticator,
    inner: server::Behavior<Info, Info>,
    pending_authentication: HashMap<RequestId, Authentication>,
    authenticating: FuturesMap<RequestId, Result<(SessionId, PlayerId), AuthError>>,
    pending_event: VecDeque<Event>,
}

struct Authentication {
    peer_id: PeerId,
    connection_id: ConnectionId,
    info: Info,
    responder: Responder<Info>,
}

impl<TAuthenticator> Behavior<TAuthenticator>
where
    TAuthenticator: Authenticator<String> + Clone + Send + 'static,
{
    pub fn new(info: Info, authenticator: TAuthenticator, config: Config) -> Self {
        Self {
            info,
            authenticator,
            inner: server::Behavior::new(vec![PROTOCOL_NAME], config),
            pending_authentication: HashMap::new(),
            authenticating: FuturesMap::new(|| Delay::futures_timer(Duration::from_secs(10)), 1000),
            pending_event: VecDeque::new(),
        }
    }

    fn on_authenticating(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        request_id: RequestId,
        request: Request<Info>,
        responder: Responder<Info>,
    ) {
        let token = request.get_metadata("x-token").cloned();
        if let Some(token) = token {
            let authenticator = self.authenticator.clone();
            let check_token = token.clone();
            let fut = async move { authenticator.authenticate(check_token).await };
            if self
                .authenticating
                .try_push(request_id, fut.boxed())
                .is_err()
            {
                let _ = responder.err_response(Code::Internal.into());
                self.pending_event.push_back(Event::Unauthenticated {
                    peer_id,
                    connection_id,
                    cause: AuthError::Io(io::Error::new(
                        io::ErrorKind::Other,
                        "Authentication failed, task limit reached",
                    )),
                });
            } else {
                let info = request.into_payload();
                self.pending_authentication.insert(
                    request_id,
                    Authentication {
                        peer_id,
                        connection_id,
                        info: info.clone(),
                        responder,
                    },
                );
                self.pending_event.push_back(Event::Authenticating {
                    peer_id,
                    connection_id,
                    info,
                    token,
                });
            }
        } else {
            let _ = responder.err_response(Code::Unauthenticated.into());
            self.pending_event.push_back(Event::Unauthenticated {
                peer_id,
                connection_id,
                cause: AuthError::EmptyToken,
            });
        }
    }

    fn on_authenticated(
        &mut self,
        request_id: RequestId,
        result: Result<(SessionId, PlayerId), AuthError>,
    ) {
        if let Some(Authentication {
            peer_id,
            connection_id,
            info,
            responder,
        }) = self.pending_authentication.remove(&request_id)
        {
            match result {
                Ok((session_id, player_id)) => {
                    let _ = responder.ok_response(self.info.clone());
                    self.pending_event.push_back(Event::Authenticated {
                        peer_id,
                        connection_id,
                        player_id,
                        session_id,
                        info,
                    });
                }
                Err(cause) => {
                    tracing::warn!("Authentication failed for {}: {:?}", peer_id, cause);
                    let code = match &cause {
                        AuthError::InvalidToken(_)
                        | AuthError::ExpiredToken
                        | AuthError::Unauthorized => Code::Unauthenticated,
                        _ => Code::Internal,
                    };
                    let _ = responder.err_response(code.into());
                    self.pending_event.push_back(Event::Unauthenticated {
                        peer_id,
                        connection_id,
                        cause,
                    });
                }
            }
        } else {
            tracing::warn!(
                "Authentication request {} not found in pending authentication",
                request_id
            );
        }
    }

    fn on_request_event(&mut self, event: server::Event<Info, Info>) {
        match event {
            server::Event::Request {
                peer_id,
                connection_id,
                request_id,
                request,
                responder,
            } => {
                self.on_authenticating(peer_id, connection_id, request_id, request, responder);
            }
            server::Event::Failure {
                peer_id,
                connection_id,
                request_id,
                cause,
            } => {
                self.pending_event.push_back(Event::AuthenticateFailure {
                    peer_id,
                    connection_id,
                    request_id,
                    error: cause.into(),
                });
            }
            server::Event::ResponseSent {
                peer_id,
                connection_id,
                request_id,
            } => {
                tracing::trace!(
                    "Authentication response sent for request {} from {} on connection {}",
                    request_id,
                    peer_id,
                    connection_id
                );
            }
        }
    }
}

impl<TAuthenticator> NetworkBehavior for Behavior<TAuthenticator>
where
    TAuthenticator: Authenticator<String> + Clone + Send + 'static,
{
    type Event = Event;
    type ConnectionHandler = server::Handler<Info, Info>;

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
            match self.authenticating.poll_unpin(cx) {
                Poll::Ready((request_id, Ok(result))) => {
                    self.on_authenticated(request_id, result);
                }
                Poll::Ready((request_id, Err(_))) => {
                    self.on_authenticated(request_id, Err(AuthError::Timeout));
                }
                Poll::Pending => {}
            }
            if let Some(event) = self.pending_event.pop_front() {
                return Poll::Ready(BehaviorEvent::Behavior(event));
            }

            match self.inner.poll(cx) {
                Poll::Ready(BehaviorEvent::Behavior(event)) => {
                    self.on_request_event(event);
                    continue;
                }
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

impl<TAuthenticator> NetworkIncomingBehavior for Behavior<TAuthenticator>
where
    TAuthenticator: Authenticator<String> + Clone + Send + 'static,
{
    /// 处理已建立的连接
    fn handle_established_connection(
        &mut self,
        id: ConnectionId,
        peer_id: PeerId,
        local_addr: &Url,
        remote_addr: &Url,
    ) -> Result<Self::ConnectionHandler, ConnectionDenied> {
        self.inner
            .handle_established_connection(id, peer_id, local_addr, remote_addr)
    }

    /// 连接处理器事件处理
    fn on_connection_established(
        &mut self,
        id: ConnectionId,
        peer_id: PeerId,
        local_addr: &Url,
        remote_addr: &Url,
    ) {
        self.inner
            .on_connection_established(id, peer_id, local_addr, remote_addr);
    }

    fn on_connection_closed(
        &mut self,
        id: ConnectionId,
        peer_id: PeerId,
        local_addr: &Url,
        remote_addr: &Url,
        reason: Option<&ConnectionError>,
    ) {
        self.inner
            .on_connection_closed(id, peer_id, local_addr, remote_addr, reason);
    }

    /// 监听失败事件处理
    fn on_listen_failure(
        &mut self,
        id: ConnectionId,
        peer_id: Option<PeerId>,
        local_addr: &Url,
        remote_addr: &Url,
        error: &ListenError,
    ) {
        self.inner
            .on_listen_failure(id, peer_id, local_addr, remote_addr, error);
    }

    /// 监听器事件处理
    fn on_listener_event(&mut self, event: ListenerEvent<'_>) {
        self.inner.on_listener_event(event);
    }
}

#[derive(Debug)]
pub enum Event {
    Authenticating {
        peer_id: PeerId,
        connection_id: ConnectionId,
        info: Info,
        token: String,
    },
    Authenticated {
        peer_id: PeerId,
        connection_id: ConnectionId,
        player_id: PlayerId,
        session_id: SessionId,
        info: Info,
    },
    Unauthenticated {
        peer_id: PeerId,
        connection_id: ConnectionId,
        cause: AuthError,
    },
    AuthenticateFailure {
        peer_id: PeerId,
        connection_id: ConnectionId,
        request_id: RequestId,
        error: io::Error,
    },
}
