use crate::{CommandResult, Context, components::player::Player};

/// Reposts the player embed at the bottom of this channel.
#[poise::command(slash_command, prefix_command, guild_only, ephemeral)]
pub async fn queue(ctx: Context<'_>) -> CommandResult {
    match Player::get(ctx).await {
        Some(player) => {
            player.post_to(ctx.channel_id()).await;
            ctx.say("Here you go").await?;
        }
        None => {
            ctx.say("Nothing is queued").await?;
        }
    }
    Ok(())
}
