pub mod callbacks;
pub mod commands;
pub mod error;

use poise::serenity_prelude as serenity;
use reqwest::Client as HttpClient;
use serenity::{
    client::Client,
    prelude::{GatewayIntents, TypeMapKey},
};
use songbird::SerenityInit;
use std::env;

use error::BotError;

pub struct Data {}
pub type Context<'a> = poise::Context<'a, Data, BotError>;
pub type Result<T> = std::result::Result<T, BotError>;
pub type CommandResult = Result<()>;

struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::utils::age::age(),
                commands::utils::ping::ping(),
                commands::music::join::join(),
                commands::music::leave::leave(),
                commands::music::play::play(),
                commands::music::skip::skip(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                case_insensitive_commands: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, ready, framework| Box::pin(callbacks::on_ready(ctx, ready, framework)))
        .build();

    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new())
        .await
        .expect("Error creating serenity client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| println!("Client ended: {:?}", why));
    });

    tokio::signal::ctrl_c()
        .await
        .map_err(|why| println!("Failed to handle Ctrl-C signal: {:?}", why))
        .ok();
    println!("Received Ctrl-C, shutting down.");
}
