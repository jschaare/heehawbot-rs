use crate::{CommandResult, Context};

use poise::serenity_prelude::User;

#[poise::command(slash_command, prefix_command)]
pub async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user_input: Option<User>,
) -> CommandResult {
    let user = user_input.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!(
        "{}'s account was created at {}",
        user.name,
        user.created_at()
    );
    ctx.say(response).await?;
    Ok(())
}
