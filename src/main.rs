use std::path::PathBuf;

use archive::retrieval;
use chat::CompletionMessage;
use config::store::ChatBotConfig;

mod archive;
mod bot;
mod chat;
mod config;

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = ChatBotConfig::read(PathBuf::from("config.toml")).unwrap();

    let bot = bot::ChatBot::new(config).await.unwrap();
    bot.run().await;
}

// #[tokio::main]
// async fn main() {
//     // memory archival test
//     let mut storage = archive::MemoryStorage::new();
//     storage.add_memory("hello world").unwrap();
//     storage.add_memory("hello again").unwrap();
//     storage.add_memory("hello again").unwrap();

//     println!(
//         "search results: {:?}",
//         storage.search("again", 0.0).unwrap()
//     );
// }

// #[tokio::main]
// async fn main() {
//     let config = ChatBotConfig::read(PathBuf::from("config.toml")).unwrap();

//     let bot = retrieval::RetrievalClient::new(&config.retrieval, 1120638385124556870.into());

//     // bot.store(
//     //     vec![CompletionMessage {
//     //         role: "user".to_string(),
//     //         content: "The user told you his shirt is blue".to_string(),
//     //     }],
//     //     1120638385124556870.into(),
//     // )
//     // .await
//     // .unwrap();

//     // bot.store(vec![
//     //     CompletionMessage {
//     //         role: "user".to_string(),
//     //         content: "hey, whats up mei?".to_string(),
//     //     },
//     //     CompletionMessage {
//     //         role: "assistant".to_string(),
//     //         content: "nothing much gus, just coding. you?".to_string(),
//     //     },
//     //     CompletionMessage {
//     //         role: "user".to_string(),
//     //         content: "same... did i ever tell you how much i like apple pie?".to_string(),
//     //     },
//     //     CompletionMessage {
//     //         role: "assistant".to_string(),
//     //         content: "really? i had no idea gus... it's such a treat isn't it?".to_string(),
//     //     },
//     // ])
//     // .await
//     // .unwrap();

//     // bot.store(vec![
//     //     CompletionMessage {
//     //         role: "user".to_string(),
//     //         content: "*i see you've woken up finally, it's about time, almost 2pm already...*\n\nhey, good morning...".to_string(),
//     //     },
//     //     CompletionMessage {
//     //         role: "assistant".to_string(),
//     //         content: "*i yawn, my eyes shutting as the light bleeds in my room, still a bit to sensitivy*\n\nhi... you doing okay? you don't look too well..".to_string(),
//     //     },
//     //     CompletionMessage {
//     //         role: "user".to_string(),
//     //         content: "*i break eye contact for a moment... you can really read me like a book, can't you..*\n\nkinda... i went to my grandfather's funeral..."
//     //             .to_string(),
//     //     },
//     //     CompletionMessage {
//     //         role: "assistant".to_string(),
//     //         content: "*my expression drops as i hear you say that*\n\nr-really? i had no idea... you should've told me... i would've been there with you...".to_string(),
//     //     },
//     // ])
//     // .await
//     // .unwrap();

//     let recalled = bot
//         .recall(vec![CompletionMessage {
//             role: "user".to_string(),
//             content: "hey... could you cook me something... im feeling kinda sad...".to_string(),
//         }])
//         .await
//         .unwrap();

//     println!("{recalled:?}");
// }
