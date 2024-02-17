use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
};
use songbird::input::YoutubeDl;

use crate::HttpKey;

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
    msg.reply(ctx, format!("received {}", url)).await?;

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

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let src = YoutubeDl::new(http_client, url);
        let _ = handler.play_input(src.clone().into());

        msg.reply(ctx, "Playing song").await?;
    } else {
        msg.reply(ctx, "Not in a voice channel to play in").await?;
    }

    Ok(())
}
