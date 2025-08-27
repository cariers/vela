include!(concat!(env!("OUT_DIR"), "/vela.table.rs"));

pub use client_protocol::Message as ClientMessage;
pub use server_protocol::Message as ServerMessage;
