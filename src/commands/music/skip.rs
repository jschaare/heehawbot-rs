use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};

#[command]
pub async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if queue.len() == 1 {
            msg.reply(ctx, "No songs to skip").await?;
            return Ok(())
        }
        queue.skip().expect("failed to skip");

        msg.reply(ctx, "Skipped song").await?;
    } else {
        msg.reply(ctx, "Not in a voice channel to skip in").await?;
    }
    Ok(())
}
