use serenity::all::User;
use std::collections::HashMap;
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::bot::Data;

use super::ChatEngine;

/// Wraps an engine reference together with its write guard.
pub struct EngineGuard<'a> {
    // Keep the guard so the reference remains valid.
    _guard: RwLockReadGuard<'a, HashMap<User, RwLock<ChatEngine>>>,
    user: User,
}

impl<'a> EngineGuard<'a> {
    pub async fn lock(data: &'a Data, user: User) -> Self {
        let user_map = data.user_map.read().await;
        let contains = user_map.contains_key(&user);
        drop(user_map);
        match contains {
            true => (),
            false => {
                let mut user_map = data.user_map.write().await;
                data.config.write().await.update();
                let config = data.config.read().await.clone();
                let engine = ChatEngine::new(config, user.id);

                user_map.insert(user.clone(), RwLock::new(engine));
            }
        };

        let user_map = data.user_map.read().await;
        Self {
            _guard: user_map,
            user,
        }
    }

    pub async fn engine(&self) -> &RwLock<ChatEngine> {
        self._guard.get(&self.user).unwrap()
    }
}
