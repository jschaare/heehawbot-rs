use crate::{CommandResult, Context, HttpKey};

use songbird::input::YoutubeDl;

#[poise::command(slash_command, prefix_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "url or search query"]
    #[rest]
    url: String,
) -> CommandResult {
    let guild_id = ctx.guild_id().unwrap();

    let http_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    // if not currently in voice channel, try to join
    // TODO: broken
    // if let None = manager.get(guild_id) {
    //     join::join();
    // }

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let src = if !url.starts_with("http") {
            // just search for all the args
            ctx.say(format!("Searching for \"{}\"!", url)).await?;
            YoutubeDl::new_search(http_client, url)
        } else {
            YoutubeDl::new(http_client, url)
        };

        // enqueue using songbird built-in queue
        let _ = handler.enqueue_input(src.clone().into()).await;

        let queue_len = handler.queue().len();
        if queue_len > 1 {
            ctx.say(format!("Queued song at position {}!", queue_len - 1))
                .await?;
        } else {
            ctx.say("Playing song").await?;
        }
    } else {
        ctx.say("Not in a voice channel to play in").await?;
    }

    Ok(())
}
