use tracing::info;

use crate::{CommandResult, Context};

#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn leave(ctx: Context<'_>) -> CommandResult {
    let author = ctx.author();
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if manager.get(guild_id).is_some() {
        manager.remove(guild_id).await.expect("Failed to leave");

        info!(
            "guild={} user(name=\"{}\",id={}) disconnected bot from voice channel",
            guild_id, &author.name, &author.id
        );
        ctx.say("Left voice channel").await?;
    }

    Ok(())
}
