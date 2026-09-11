use crate::{CommandResult, Context, components::player::Player};

#[poise::command(slash_command, prefix_command, guild_only, ephemeral)]
pub async fn skip(ctx: Context<'_>) -> CommandResult {
    match Player::get(ctx).await {
        Some(player) => {
            player.skip().await;
            ctx.say("Skipped song").await?;
        }
        None => {
            ctx.say("Not in a voice channel to skip in").await?;
        }
    }
    Ok(())
}
