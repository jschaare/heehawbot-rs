use crate::{CommandResult, Context, components::player::Player};

#[poise::command(slash_command, prefix_command, guild_only, ephemeral)]
pub async fn join(ctx: Context<'_>) -> CommandResult {
    let join_msg = match Player::join(ctx).await {
        Some(_) => "Joined voice channel!",
        None => "You are not in a voice channel, please join one.",
    };
    ctx.say(join_msg).await?;
    Ok(())
}
