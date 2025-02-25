use events::HandlerResult;
pub use framework::Data;
use serenity::{
    all::{Context, EventHandler, Interaction, Message, MessageUpdateEvent, Ready},
    async_trait,
};

mod buttons;
mod events;
pub mod framework;

pub struct Handler {
    pub data: Data,
}
impl Handler {
    pub fn new(data: Data) -> Self {
        Self { data }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        log::info!("{} is connected!", ready.user.name);

        ctx.set_presence(None, serenity::all::OnlineStatus::Online);
    }

    // async fn typing_start(&self, ctx: Context, event: TypingStartEvent) {
    //     let is_dm = event.guild_id.is_none();
    //
    //     if is_dm {
    //         // someone is typing in MY dms
    //         log::info!("User {} is typing in DMs", event.user_id);
    //         log::info!("Channel ID: {:?}", event.channel_id);
    //
    //         let typing = ctx.http.start_typing(event.channel_id);
    //
    //         tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    //
    //         if let Err(why) = event.channel_id.say(&ctx.http, "hey... you up too?").await {
    //             log::info!("Error sending message: {why:?}");
    //         }
    //
    //         typing.stop();
    //     }
    // }

    async fn message(&self, ctx: Context, msg: Message) {
        if let HandlerResult::Err(error) = self.on_message(ctx, msg).await {
            Self::on_error(error).await;
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let HandlerResult::Err(error) = self.on_interaction(ctx, interaction).await {
            Self::on_error(error).await;
        }
    }

    async fn message_update(
        &self,
        ctx: Context,
        old_if_available: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        if let HandlerResult::Err(error) = self.on_edit(ctx, old_if_available, new, event).await {
            Self::on_error(error).await;
        }
    }
}
