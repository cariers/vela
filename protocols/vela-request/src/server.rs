use std::task::{Context, Poll};

use volans::{
    core::{PeerId, Url},
    request::{Config, InboundFailure, RequestId, server},
    swarm::{
        BehaviorEvent, ConnectionDenied, ConnectionId, ListenerEvent, NetworkBehavior,
        NetworkIncomingBehavior, StreamProtocol, THandlerAction, THandlerEvent,
        error::{ConnectionError, ListenError},
    },
};

use crate::{Codec, Request, Responder};

pub type Handler<TRequest, TResponse> = server::Handler<Codec<TRequest, TResponse>>;

pub struct Behavior<TRequest, TResponse>
where
    TRequest: prost::Message + Default + Send + Clone + 'static,
    TResponse: prost::Message + Default + Send + Clone + 'static,
{
    inner: server::Behavior<Codec<TRequest, TResponse>>,
}

impl<TRequest, TResponse> Behavior<TRequest, TResponse>
where
    TRequest: prost::Message + Default + Send + Clone + 'static,
    TResponse: prost::Message + Default + Send + Clone + 'static,
{
    pub fn new<P>(protocols: P, config: Config) -> Self
    where
        P: IntoIterator<Item = StreamProtocol>,
    {
        let codec = Codec::<TRequest, TResponse>::new();
        Self {
            inner: server::Behavior::with_codec(codec, protocols, config),
        }
    }
}

impl<TRequest, TResponse> NetworkBehavior for Behavior<TRequest, TResponse>
where
    TRequest: prost::Message + Default + Send + Clone + 'static,
    TResponse: prost::Message + Default + Send + Clone + 'static,
{
    type Event = Event<TRequest, TResponse>;
    type ConnectionHandler = server::Handler<Codec<TRequest, TResponse>>;

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
        self.inner.poll(cx).map(|e| {
            e.map_event(|e| match e {
                server::Event::Request {
                    peer_id,
                    connection_id,
                    request_id,
                    request,
                    responder,
                } => Event::Request {
                    peer_id,
                    connection_id,
                    request_id,
                    request,
                    responder: Responder::new(responder),
                },
                server::Event::Failure {
                    peer_id,
                    connection_id,
                    request_id,
                    cause,
                } => Event::Failure {
                    peer_id,
                    connection_id,
                    request_id,
                    cause,
                },
                server::Event::ResponseSent {
                    peer_id,
                    connection_id,
                    request_id,
                } => Event::ResponseSent {
                    peer_id,
                    connection_id,
                    request_id,
                },
            })
        })
    }
}

impl<TRequest, TResponse> NetworkIncomingBehavior for Behavior<TRequest, TResponse>
where
    TRequest: prost::Message + Default + Send + Clone + 'static,
    TResponse: prost::Message + Default + Send + Clone + 'static,
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
pub enum Event<TRequest, TResponse> {
    Request {
        peer_id: PeerId,
        connection_id: ConnectionId,
        request_id: RequestId,
        request: Request<TRequest>,
        responder: Responder<TResponse>,
    },
    Failure {
        peer_id: PeerId,
        connection_id: ConnectionId,
        request_id: RequestId,
        cause: InboundFailure,
    },
    ResponseSent {
        peer_id: PeerId,
        connection_id: ConnectionId,
        request_id: RequestId,
    },
}
