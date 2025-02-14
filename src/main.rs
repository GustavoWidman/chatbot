use std::path::PathBuf;

use chat::ChatMessage;
use config::store::ChatBotConfig;

mod archive;
mod bot;
mod chat;
mod config;
mod utils;

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
//     let config = ChatBotConfig::read(PathBuf::from("config.toml")).unwrap();

//     let client = chat::client::ChatClient::new(&config.llm, 1120638385124556870.into());

//     client.store(vec![
//         CompletionMessage {
//             role: "user".to_string(),
//             content: "hey, whats up mei?".to_string(),
//         },
//         CompletionMessage {
//             role: "assistant".to_string(),
//             content: "nothing much gus, just coding. you?".to_string(),
//         },
//         CompletionMessage {
//             role: "user".to_string(),
//             content: "same... did i ever tell you about the whole time i almost got expelled?".to_string(),
//         },
//         CompletionMessage {
//             role: "assistant".to_string(),
//             content: "really? i had no idea gus... what happened?".to_string(),
//         },
//         CompletionMessage {
//             role: "user".to_string(),
//             content: "so basically, there's this whole check-in system right? you know it? the one where we have to go on the college's website to get the presence for that class, or else it's counted as a missed class?".to_string(),
//         },
//         CompletionMessage {
//             role: "assistant".to_string(),
//             content: "yea, i know, what's up with it?".to_string(),
//         },
//         CompletionMessage {
//             role: "user".to_string(),
//             content: "so what i did, was i automated the system by planting a raspberry pi on campus, and then i wrote a script that would check in every class, that way i wouldn't have to go to the website, and it would be automated, so i could just sit back and relax, and not worry about it.".to_string(),
//         },
//         CompletionMessage {
//             role: "assistant".to_string(),
//             content: "that's a great idea, but i can already see what happened.".to_string(),
//         },
//         CompletionMessage {
//             role: "user".to_string(),
//             content: "yea... problem is, it wasn't a teacher that found it, but rather a student that i had shared this information with, he reported me to the ethics committee and i almost got expelled because of it, i got a 6 month suspension and a 1 year ban from any and all clubs, including the cybersecurity club, which i was president".to_string(),
//         },
//         CompletionMessage {
//             role: "assistant".to_string(),
//             content: "oh wow....".to_string(),
//         },
//     ], "gus".to_string(), "mei".to_string()).await.unwrap();

//     // println!(
//     //     "search results: {:?}",
//     //     client.storage.search("expelled", 0.0).unwrap()
//     // );
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
