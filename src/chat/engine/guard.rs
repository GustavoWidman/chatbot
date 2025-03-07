use serenity::all::{Http, UserId};
use std::collections::HashMap;
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::{bot::Data, utils::macros::config};

use super::ChatEngine;

/// Wraps an engine reference together with its write guard.
pub struct EngineGuard<'a> {
    // Keep the guard so the reference remains valid.
    _guard: RwLockReadGuard<'a, HashMap<UserId, RwLock<ChatEngine>>>,
    user: UserId,
}

impl<'a> EngineGuard<'a> {
    pub async fn lock(data: &'a Data, user: UserId, http: &Http) -> anyhow::Result<Self> {
        let user_map = data.user_map.read().await;
        let contains = user_map.contains_key(&user);
        drop(user_map);
        match contains {
            true => (),
            false => {
                let mut user_map = data.user_map.write().await;
                let config = config!(data);
                let engine = ChatEngine::new(config, user.clone(), http).await?;

                user_map.insert(user, RwLock::new(engine));
            }
        };

        let user_map = data.user_map.read().await;
        Ok(Self {
            _guard: user_map,
            user,
        })
    }

    pub async fn engine(&self) -> &RwLock<ChatEngine> {
        self._guard.get(&self.user).unwrap()
    }
}
