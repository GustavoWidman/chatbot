use futures::StreamExt;
use serenity::all::{ChannelId, CreateButton, CreateMessage, Http, MessageId};

pub fn time_to_string(time: chrono::Duration) -> String {
    match time.num_seconds() {
        0..=59 => {
            let second_suffix = if time.num_seconds() == 1 { "" } else { "s" };
            format!("{} second{}", time.num_seconds(), second_suffix)
        }
        60..=3599 => {
            let minute_suffix = if time.num_minutes() == 1 { "" } else { "s" };
            format!("{} minute{}", time.num_minutes(), minute_suffix)
        }
        3600..=86399 => {
            let hour_suffix = if time.num_hours() == 1 { "" } else { "s" };
            format!("{} hour{}", time.num_hours(), hour_suffix)
        }
        _ => {
            let day_suffix = if time.num_days() == 1 { "" } else { "s" };
            format!("{} day{}", time.num_days(), day_suffix)
        }
    }
}

pub fn chunk_string(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut remaining = s;

    while !remaining.is_empty() {
        // If the remaining text is within the limit, add it and break.
        if remaining.len() <= 2000 {
            result.push(remaining.to_string());
            break;
        }

        // Consider the first 2000 characters.
        let slice = &remaining[..2000];

        // Try to find a split point: newline, then period, then space.
        let split_point = slice
            .rfind('\n')
            .or_else(|| slice.rfind('.'))
            .or_else(|| slice.rfind(' '))
            .map(|i| i + 1) // include the delimiter in the chunk
            .unwrap_or(2000);

        // Take the chunk up to the determined split point.
        let chunk = &remaining[..split_point];
        result.push(chunk.to_string());

        // Update the remaining text.
        remaining = &remaining[split_point..];
    }

    result
}

pub struct ButtonStates {
    pub prev_disabled: bool,
    pub regen_or_next: RegenOrNext,
}

pub enum RegenOrNext {
    Regen,
    Next,
}

pub fn chunk_message(message: &str, state: ButtonStates) -> anyhow::Result<Vec<CreateMessage>> {
    let mut chunks = chunk_string(message);
    let last = chunks.pop().ok_or(anyhow::anyhow!("no chunks"))?;

    let mut messages = chunks
        .into_iter()
        .map(|chunk| CreateMessage::new().content(chunk))
        .collect::<Vec<_>>();

    let (regen_or_next_id, regen_or_next_emoji) = match state.regen_or_next {
        RegenOrNext::Next => ("next", '⏩'),
        RegenOrNext::Regen => ("regen", '♻'),
    };

    let message = CreateMessage::new()
        .content(last)
        .button(
            CreateButton::new("prev")
                .label("")
                .emoji('⏪')
                .style(serenity::all::ButtonStyle::Secondary)
                .disabled(state.prev_disabled),
        )
        .button(
            CreateButton::new(regen_or_next_id)
                .label("")
                .emoji(regen_or_next_emoji)
                .style(serenity::all::ButtonStyle::Secondary),
        )
        .button(
            CreateButton::new("edit")
                .label("")
                .emoji('✏')
                .style(serenity::all::ButtonStyle::Secondary)
                .disabled(false),
        );

    messages.push(message);

    Ok(messages)
}

pub async fn send_message_batch(
    channel: ChannelId,
    http: &Http,
    messages: Vec<CreateMessage>,
) -> anyhow::Result<Vec<MessageId>> {
    futures::stream::iter(messages)
        .then(|message| async {
            channel
                .send_message(http, message)
                .await
                .map(|msg| msg.id)
                .map_err(anyhow::Error::from)
        })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect()
}

pub async fn delete_message_batch(
    channel: ChannelId,
    http: &Http,
    message_ids: Vec<MessageId>,
) -> anyhow::Result<()> {
    futures::stream::iter(message_ids)
        .then(async |message_id| {
            channel
                .delete_message(http, message_id)
                .await
                .map(|_| ())
                .map_err(anyhow::Error::from)
        })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect()
}
