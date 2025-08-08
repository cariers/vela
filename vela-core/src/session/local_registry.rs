use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use parking_lot::Mutex;

use crate::{
    ids::{PlayerId, SessionId},
    session::{Session, SessionRegistry},
};

struct Shared {
    sessions: HashMap<SessionId, Session>,
    player_sessions: HashMap<PlayerId, HashSet<SessionId>>,
}

impl Shared {
    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            player_sessions: HashMap::new(),
        }
    }

    fn lookup(&self, id: SessionId) -> Option<Session> {
        self.sessions.get(&id).cloned()
    }

    fn insert(&mut self, session: Session) -> Result<(), Session> {
        if self.sessions.contains_key(&session.id) {
            return Err(session);
        }
        self.player_sessions
            .entry(session.player_id.clone())
            .or_default()
            .insert(session.id.clone());
        let _ = self.sessions.insert(session.id.clone(), session);
        Ok(())
    }

    fn remove(&mut self, id: SessionId) -> Option<Session> {
        if let Some(session) = self.sessions.remove(&id) {
            self.player_sessions
                .entry(session.player_id.clone())
                .or_default()
                .remove(&id);
            if let Some(sessions) = self.player_sessions.get_mut(&session.player_id) {
                if sessions.is_empty() {
                    self.player_sessions.remove(&session.player_id);
                }
            }
            Some(session)
        } else {
            None
        }
    }

    fn single_session(&mut self, player_id: PlayerId, session_id: SessionId) -> Vec<Session> {
        let mut sessions = Vec::new();
        if let Some(ids) = self.player_sessions.get(&player_id) {
            for id in ids {
                if *id != session_id {
                    if let Some(session) = self.sessions.remove(id) {
                        sessions.push(session.clone());
                    }
                }
            }
        }
        sessions
    }
}

pub struct LocalSessionRegistry {
    shared: Arc<Mutex<Shared>>,
}

impl LocalSessionRegistry {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Mutex::new(Shared::new())),
        }
    }
}

#[async_trait::async_trait]
impl SessionRegistry for LocalSessionRegistry {
    async fn lookup(&mut self, id: SessionId) -> Option<Session> {
        let shared = self.shared.lock();
        shared.lookup(id)
    }
    async fn insert(&mut self, session: Session) -> Result<(), Session> {
        let mut shared = self.shared.lock();
        shared.insert(session)
    }
    async fn remove(&mut self, id: SessionId) -> bool {
        let mut shared = self.shared.lock();
        shared.remove(id).is_some()
    }
    async fn single_session(&mut self, player_id: PlayerId, session_id: SessionId) -> Vec<Session> {
        let mut shared = self.shared.lock();
        shared.single_session(player_id, session_id)
    }
}
