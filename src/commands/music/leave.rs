use crate::{CommandResult, Context};

#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn leave(ctx: Context<'_>) -> CommandResult {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if manager.get(guild_id).is_some() {
        manager.remove(guild_id).await.expect("Failed to leave");
        ctx.say("Left voice channel").await?;
    }

    Ok(())
}
