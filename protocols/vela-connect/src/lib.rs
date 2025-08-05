pub mod inbound;
pub mod outbound;
pub mod protocol;

use volans::{core::PeerId, swarm::ConnectionId};

#[derive(Debug)]
pub struct Event<TInfo> {
    pub peer_id: PeerId,
    pub connection: ConnectionId,
    pub info: TInfo,
}
