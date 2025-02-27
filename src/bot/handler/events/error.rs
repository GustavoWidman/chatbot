use std::sync::Arc;

use poise::CreateReply;
use serenity::all::{
    ChannelId, CreateActionRow, CreateButton, CreateEmbed, CreateMessage, Http, Message,
    MessageReference,
};

use crate::bot::handler::framework::Context;

use super::super::Handler;

impl Handler {
    /// attempts to relay an error into a discord channel, still logging it
    /// if fails, still logs and logs the failure of the failure (lol)
    pub async fn on_error(error: HandlerError<'_>) {
        let HandlerError { error, location } = error;

        log::error!("handling error:\n\n{error:?}\n");

        let embed = CreateEmbed::default()
            .color(0xFF6961)
            .title("Chatbot encountered an error")
            .description(format!("```{}```", error.to_string()));
        let button = CreateButton::new("delete_error")
            .label("")
            .emoji('🗑')
            .style(serenity::all::ButtonStyle::Danger);

        match location {
            ErrorLocation::Context(ctx) => {
                if let Err(why) = ctx
                    .send(
                        CreateReply::default()
                            .embed(embed)
                            .ephemeral(true)
                            .components(vec![CreateActionRow::Buttons(vec![button])]),
                    )
                    .await
                {
                    log::error!("error during propagation of error to user: {why:?}");
                }
            }
            ErrorLocation::Message((http, message)) => {
                if let Err(why) = message
                    .channel_id
                    .send_message(
                        http,
                        CreateMessage::new()
                            .button(button)
                            .reference_message(&message)
                            .embed(embed),
                    )
                    .await
                {
                    log::error!("error during propagation of error to user: {why:?}");
                }
            }
            ErrorLocation::Channel((http, channel_id, reference)) => {
                let mut message = CreateMessage::new().embed(embed).button(button);
                if let Some(ref_msg) = reference {
                    message = message.reference_message(ref_msg);
                }
                if let Err(why) = channel_id.send_message(http, message).await {
                    log::error!("error during propagation of error to user: {why:?}");
                }
            }
        }
    }
}

pub enum ErrorLocation<'a> {
    Context(Context<'a>),
    Message((Arc<Http>, Message)),
    Channel((Arc<Http>, ChannelId, Option<MessageReference>)),
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
        let (http, channel_id) = self;
        ErrorLocation::Channel((http, channel_id, None))
    }
}

impl Into<ErrorLocation<'static>> for (Arc<Http>, ChannelId, MessageReference) {
    fn into(self) -> ErrorLocation<'static> {
        let (http, channel_id, reference) = self;
        ErrorLocation::Channel((http, channel_id, Some(reference)))
    }
}

impl Into<ErrorLocation<'static>> for (Arc<Http>, ChannelId, Option<MessageReference>) {
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
