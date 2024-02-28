use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
};
use songbird::input::YoutubeDl;

use crate::HttpKey;
use crate::commands::music::join;

#[command]
pub async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            msg.reply(ctx, "Please provide a URL to a song or video")
                .await?;
            return Ok(());
        }
    };

    let guild_id = msg.guild_id.unwrap();

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    // if not currently in voice channel, try to join
    if let None = manager.get(guild_id) {
        join::join(ctx, msg, args)
            .await
            .expect("Voice channel connection failed.");
    }

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let src = YoutubeDl::new(http_client, url);

        // enqueue using songbird built-in queue
        let _ = handler.enqueue_input(src.clone().into()).await;

        let queue_len = handler.queue().len();
        if queue_len > 1 {
            msg.reply(ctx,  format!("Queued song at position {}!", queue_len - 1)).await?;
        } else {
            msg.reply(ctx, "Playing song").await?;
        }
    } else {
        msg.reply(ctx, "Not in a voice channel to play in").await?;
    }

    Ok(())
}
