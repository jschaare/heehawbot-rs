use crate::{CommandResult, Context};

#[poise::command(prefix_command, slash_command)]
pub async fn ping(ctx: Context<'_>) -> CommandResult {
    ctx.say("pong").await?;

    Ok(())
}
