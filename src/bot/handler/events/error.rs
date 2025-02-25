use std::sync::Arc;

use poise::CreateReply;
use serenity::all::{ChannelId, CreateMessage, Http, Message};

use crate::bot::handler::framework::Context;

use super::super::Handler;

impl Handler {
    /// attempts to relay an error into a discord channel, still logging it
    /// if fails, still logs and logs the failure of the failure (lol)
    pub async fn on_error(error: HandlerError<'_>) {
        let HandlerError { error, location } = error;

        log::error!("handling error:\n{error:?}");

        match location {
            ErrorLocation::Context(ctx) => {
                if let Err(why) = ctx
                    .send(
                        CreateReply::default()
                            .content(error.to_string())
                            .ephemeral(true),
                    )
                    .await
                {
                    log::error!("error during propagation of error to user: {why:?}");
                }
            }
            ErrorLocation::Message((http, message)) => {
                if let Err(why) = message.reply(http, error.to_string()).await {
                    log::error!("error during propagation of error to user: {why:?}");
                }
            }
            ErrorLocation::Channel((http, channel_id)) => {
                if let Err(why) = channel_id
                    .send_message(http, CreateMessage::new().content(error.to_string()))
                    .await
                {
                    log::error!("error during propagation of error to user: {why:?}");
                }
            }
        }

        todo!()
    }
}

pub enum ErrorLocation<'a> {
    Context(Context<'a>),
    Message((Arc<Http>, Message)),
    Channel((Arc<Http>, ChannelId)),
}

impl<'a> Into<ErrorLocation<'a>> for Context<'a> {
    fn into(self) -> ErrorLocation<'a> {
        ErrorLocation::Context(self)
    }
}

impl Into<ErrorLocation<'static>> for (Arc<Http>, Message) {
    fn into(self) -> ErrorLocation<'static> {
        ErrorLocation::Message(self)
    }
}

impl Into<ErrorLocation<'static>> for (Arc<Http>, ChannelId) {
    fn into(self) -> ErrorLocation<'static> {
        ErrorLocation::Channel(self)
    }
}

pub struct HandlerError<'a> {
    error: anyhow::Error,
    location: ErrorLocation<'a>,
}

pub enum HandlerResult<'a, T> {
    Ok(T),
    Err(HandlerError<'a>),
}

impl<'a, T> HandlerResult<'a, T> {
    pub fn ok(value: T) -> Self {
        Self::Ok(value)
    }

    pub fn err(error: impl Into<anyhow::Error>, location: impl Into<ErrorLocation<'a>>) -> Self {
        Self::Err(HandlerError {
            error: error.into(),
            location: location.into(),
        })
    }
}
