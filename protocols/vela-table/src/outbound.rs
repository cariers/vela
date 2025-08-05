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
