use std::env;

use reqwest::Client as HttpClient;
use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    framework::{
        standard::{macros::group, Configuration},
        StandardFramework,
    },
    model::gateway::Ready,
    prelude::{GatewayIntents, TypeMapKey},
};
use songbird::SerenityInit;

mod commands;
use commands::{
    music::{join::JOIN_COMMAND, leave::LEAVE_COMMAND, play::PLAY_COMMAND, skip::SKIP_COMMAND},
    ping::PING_COMMAND,
};

#[group]
#[commands(ping)]
struct General;

#[group]
#[commands(join, leave, play, skip)]
struct Music;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    let framework = StandardFramework::new()
        .group(&GENERAL_GROUP)
        .group(&MUSIC_GROUP);
    framework.configure(Configuration::new().prefix("!"));

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
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
