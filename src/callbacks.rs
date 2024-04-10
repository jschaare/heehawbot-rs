use super::error::BotError;
use super::{Data, Result};

use poise::{
    serenity_prelude::{Context as SerenityContext, Ready},
    Framework,
};

pub async fn on_ready<'a>(
    ctx: &SerenityContext,
    ready: &Ready,
    framework: &Framework<Data, BotError>,
) -> Result<Data> {
    println!("{} is connected!", ready.user.name);

    #[cfg(not(debug_assertions))]
    {
        poise::builtins::register_globally(ctx, framework.options().commands.as_slice())
            .await
            .expect("Failed to register commands globally");
    }

    #[cfg(debug_assertions)]
    {
        poise::builtins::register_in_guild(
            ctx,
            framework.options().commands.as_slice(),
            std::env::var("DEV_SERVER_ID")
                .expect("`DEV_SERVER_ID` not found in dev build")
                .parse::<u64>()
                .expect("Invalid value for `DEV_SERVER_ID`")
                .into(),
        )
        .await
        .expect("Failed to register commands in dev guild");
    }

    Ok(Data {})
}
