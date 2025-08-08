pub mod client;
pub mod server;

use volans::swarm::StreamProtocol;

pub const PROTOCOL_NAME: StreamProtocol = StreamProtocol::new("/v1/connect");
