use crate::{CommandResult, Context};

#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn skip(ctx: Context<'_>) -> CommandResult {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("Can only use `/skip` inside a guild").await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if queue.len() == 0 {
            ctx.say("No songs to skip").await?;
            return Ok(());
        }
        queue.skip().expect("failed to skip");

        ctx.say("Skipped song").await?;
    } else {
        ctx.say("Not in a voice channel to skip in").await?;
    }
    Ok(())
}
