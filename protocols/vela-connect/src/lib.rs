pub mod client;
pub mod server;

use volans::swarm::StreamProtocol;

pub const PROTOCOL_NAME: StreamProtocol = StreamProtocol::new("/v1/connect");

pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/vela.connect.rs"));
}
