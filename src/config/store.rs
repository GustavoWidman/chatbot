use anyhow::bail;
use serenity::prelude::TypeMapKey;

use super::structure::{ChatBotConfigInner, ChatBotConfigTOML};
use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};

#[derive(Debug)]
pub struct ChatBotConfig {
    pub path: PathBuf,
    cached: ChatBotConfigTOML,
}

impl ChatBotConfig {
    pub fn read(path: PathBuf) -> Result<Self, anyhow::Error> {
        let path = match path.is_dir() {
            true => path.join("config.toml"),
            false => path,
        };

        if !path.exists() {
            return Ok(Self::new(path)?);
        }

        if !path.is_file() {
            bail!(
                "Given path exists and is not a file... either change the path or delete the file."
            );
        }

        let config_str = std::fs::read_to_string(&path)?;

        Ok(Self {
            path,
            cached: toml::from_str(&config_str)?,
        })
    }

    pub fn update(&mut self) -> bool {
        let new = Self::read(self.path.clone()).unwrap();

        match self.cached.config == new.cached.config {
            true => false,
            false => {
                self.cached = new.cached;
                true
            }
        }
    }

    fn new(path: PathBuf) -> Result<Self, anyhow::Error> {
        std::fs::create_dir_all(path.parent().unwrap())?;

        let config = Self {
            path,
            cached: ChatBotConfigTOML::default(),
        };

        config.save()?;

        Ok(config)
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        std::fs::write(&self.path, toml::to_string(&self.cached)?)?;

        Ok(())
    }

    pub async fn async_save(&self) -> Result<(), anyhow::Error> {
        tokio::fs::write(&self.path, toml::to_string(&self.cached)?).await?;

        Ok(())
    }
}

impl Deref for ChatBotConfig {
    type Target = ChatBotConfigInner;

    fn deref(&self) -> &Self::Target {
        &self.cached.config
    }
}

impl DerefMut for ChatBotConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cached.config
    }
}

impl Clone for ChatBotConfig {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            cached: self.cached.clone(),
        }
    }
}

impl PartialEq for ChatBotConfig {
    fn eq(&self, other: &Self) -> bool {
        self.cached.config == other.cached.config
    }
}

impl TypeMapKey for ChatBotConfig {
    type Value = ChatBotConfig;
}
