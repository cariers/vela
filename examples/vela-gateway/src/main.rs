use std::{collections::HashMap, pin::Pin};

use futures::StreamExt;
use vela_core::{authenticate::JwtAuthenticator, jwt};
use vela_protobuf::connect::Info;
use volans::{
    Transport,
    core::{PeerId, Url},
    muxing, plaintext, request,
    swarm::{self, NetworkIncomingBehavior, NetworkOutgoingBehavior, client, server},
    ws,
};

#[derive(Default, Debug, Clone, Copy)]
pub struct TokioExecutor;

impl swarm::Executor for TokioExecutor {
    fn exec(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        tokio::spawn(future);
    }
}

#[derive(NetworkIncomingBehavior)]
struct GatewayInboundBehavior {
    ping: volans::ping::inbound::Behavior,
    connect: vela_connect::server::Behavior<JwtAuthenticator>,
}

#[derive(NetworkOutgoingBehavior)]
struct GatewayOutboundBehavior {
    ping: volans::ping::outbound::Behavior,
    connect: vela_connect::client::Behavior,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    // init tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .init();

    tracing::info!("Starting TCP Echo Example");

    let addr = Url::parse("ws://0.0.0.0:8088")?;

    let key: [u8; 32] = rand::random();
    let local_key = plaintext::ed25519::SigningKey::from_bytes(&key);
    let local_peer_id = PeerId::from_bytes(key);

    let identify_upgrade = plaintext::Config::new(local_key.verifying_key());

    let muxing_upgrade = muxing::Config::new();

    let transport = ws::Config::new()
        .upgrade()
        .authenticate(identify_upgrade)
        .multiplex(muxing_upgrade)
        .boxed();

    let jwt = JwtAuthenticator::new(jwt::DecodingKey::from_secret(b"test"));

    let connect = vela_connect::server::Behavior::new(
        Info {
            name: "Vela Gateway".to_string(),
            version: "0.1.0".to_string(),
        },
        jwt,
        request::Config::default(),
    );

    let behavior = GatewayInboundBehavior {
        ping: volans::ping::inbound::Behavior::default(),
        connect,
    };

    let mut swarm = swarm::server::Swarm::new(
        transport,
        behavior,
        local_peer_id,
        swarm::connection::PoolConfig::new(Box::new(TokioExecutor)),
    );

    let _ = swarm.listen_on(addr.clone())?;

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        if let Err(e) = start_client().await {
            tracing::error!("Server error: {:?}", e);
        }
    });

    while let Some(event) = swarm.next().await {
        match event {
            server::SwarmEvent::Behavior(GatewayInboundBehaviorEvent::Connect(event)) => {
                tracing::info!("Server Connect event: {:?}", event);
            }
            server::SwarmEvent::Behavior(GatewayInboundBehaviorEvent::Ping(_)) => {}
            _ => tracing::info!("Server Swarm event: {:?}", event),
        }
    }
    Ok(())
}

async fn start_client() -> anyhow::Result<()> {
    tracing::info!("Starting TCP Demo Client");

    let addr = Url::parse("ws://0.0.0.0:8088")?;

    let key: [u8; 32] = rand::random();
    let local_key = plaintext::ed25519::SigningKey::from_bytes(&key);
    let local_peer_id = PeerId::from_bytes(key);

    let identify_upgrade = plaintext::Config::new(local_key.verifying_key());

    let muxing_upgrade = muxing::Config::new();

    let transport = ws::Config::new()
        .upgrade()
        .authenticate(identify_upgrade)
        .multiplex(muxing_upgrade)
        .boxed();

    let connect = vela_connect::client::Behavior::new(
        Info {
            name: "Vela Gateway".to_string(),
            version: "0.1.0".to_string(),
        },
        request::Config::default(),
    );

    let behavior = GatewayOutboundBehavior {
        ping: volans::ping::outbound::Behavior::default(),
        connect,
    };

    let mut swarm = swarm::client::Swarm::new(
        transport,
        behavior,
        local_peer_id,
        swarm::connection::PoolConfig::new(Box::new(TokioExecutor)),
    );

    let _ = swarm.dial(swarm::DialOpts::new(Some(addr), None)).unwrap();

    let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiJzc19hYmNkIiwic3ViIjoicHlfMTIzNDU2IiwiaWF0IjoxNTE2MjM5MDIyLCJleHAiOjIxNDY5NTkwMjJ9.tyGVTHlbgMKFNQNjvMOV32MtQuzIVLxcBddjwVX78ME".to_string();

    while let Some(event) = swarm.next().await {
        match event {
            client::SwarmEvent::ConnectionEstablished { peer_id, addr, .. } => {
                tracing::info!("Client connected to {} at {}", peer_id, addr);
                swarm
                    .behavior_mut()
                    .connect
                    .send_authentication(peer_id, token.clone());
            }
            client::SwarmEvent::Behavior(GatewayOutboundBehaviorEvent::Connect(event)) => {
                tracing::info!("Client Connect event: {:?}", event);
            }
            client::SwarmEvent::Behavior(GatewayOutboundBehaviorEvent::Ping(_)) => {}
            _ => tracing::info!("Client Swarm event: {:?}", event),
        }
    }
    Ok(())
}
