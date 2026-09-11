use crate::{CommandResult, Context, components::player::Player};

#[poise::command(slash_command, prefix_command, guild_only, ephemeral)]
pub async fn resume(ctx: Context<'_>) -> CommandResult {
    match Player::get(ctx).await {
        Some(player) => {
            player.resume().await;
            player.refresh(None).await;
            ctx.say("Resumed").await?;
        }
        None => {
            ctx.say("Nothing is playing").await?;
        }
    }
    Ok(())
}
