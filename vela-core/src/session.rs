mod local_registry;

pub use local_registry::LocalSessionRegistry;

use crate::ids::{PlayerId, SessionId};

#[async_trait::async_trait]
pub trait SessionRegistry {
    async fn lookup(&mut self, id: SessionId) -> Option<Session>;
    async fn insert(&mut self, session: Session) -> Result<(), Session>;
    async fn remove(&mut self, id: SessionId) -> bool;
    async fn single_session(&mut self, player_id: PlayerId, session_id: SessionId) -> Vec<Session>;
}

#[derive(Debug, Clone)]
pub struct Session {
    id: SessionId,
    player_id: PlayerId,
}

impl Session {
    pub fn new(id: SessionId, player_id: PlayerId) -> Self {
        Self { id, player_id }
    }

    pub fn id(&self) -> &SessionId {
        &self.id
    }

    pub fn player_id(&self) -> &PlayerId {
        &self.player_id
    }
}
