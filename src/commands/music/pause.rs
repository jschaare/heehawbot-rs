use crate::{CommandResult, Context, components::player::Player};

#[poise::command(slash_command, prefix_command, guild_only, ephemeral)]
pub async fn pause(ctx: Context<'_>) -> CommandResult {
    match Player::get(ctx).await {
        Some(player) => {
            player.pause().await;
            player.refresh(None).await;
            ctx.say("Paused").await?;
        }
        None => {
            ctx.say("Nothing is playing").await?;
        }
    }
    Ok(())
}
