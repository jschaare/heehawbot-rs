use crate::{CommandResult, Context, components::player::Player};

#[poise::command(slash_command, prefix_command, guild_only, ephemeral)]
pub async fn clear(ctx: Context<'_>) -> CommandResult {
    match Player::get(ctx).await {
        Some(player) => {
            player.clear().await;
            player.refresh(None).await;
            ctx.say("Cleared the queue").await?;
        }
        None => {
            ctx.say("Nothing is queued").await?;
        }
    }
    Ok(())
}
