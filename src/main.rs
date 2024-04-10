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
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

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

    let pkg_name = env!("CARGO_PKG_NAME").replace('-', "_");
    let level_string = env::var(EnvFilter::DEFAULT_ENV).unwrap_or_else(|_| "info".to_string());
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into())
        .from_env()
        .expect("Expected an env filter")
        .add_directive(
            format!("{}={}", pkg_name, level_string)
                .parse()
                .expect("Expected a directive"),
        );
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();

    tracing::info!("{} is starting", pkg_name);
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
            .map_err(|why| tracing::error!("client terminated: {:?}", why));
    });

    tokio::signal::ctrl_c()
        .await
        .map_err(|why| tracing::error!("failed to handle ctrl-c signal: {:?}", why))
        .ok();
    tracing::info!("received sigtem, {} is shutting down", pkg_name);
}
