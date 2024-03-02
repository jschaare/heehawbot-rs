use crate::{commands::music::join, CommandResult, Context, HttpKey};

use poise::CreateReply;
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

    {
        // if not currently in voice channel, try to join
        join::join_channel(ctx).await?;
    }

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let src = if !url.starts_with("http") {
            // just search for all the args
            ctx.send(
                CreateReply::default()
                    .content(format!("Searching for \"**{}**\"!", url))
                    .ephemeral(true),
            )
            .await?;
            YoutubeDl::new_search(http_client, url)
        } else {
            YoutubeDl::new(http_client, url)
        };

        let mut src: songbird::input::Input = src.clone().into();

        // extract metadata about song
        let aux_metadata = match src.aux_metadata().await {
            Ok(metadata) => metadata,
            Err(_e) => return Ok(()),
        };
        let title = match aux_metadata.title.clone() {
            Some(t) => t,
            None => "Unknown".to_string(),
        };
        let source_url = match aux_metadata.source_url.clone() {
            Some(url) => url,
            None => "".to_string(),
        };

        // enqueue using songbird built-in queue
        handler.enqueue_input(src).await;

        let queue_len = handler.queue().len();
        if queue_len > 1 {
            ctx.say(format!(
                "Queued song **{}** at position {}!\n{}",
                title,
                queue_len - 1,
                source_url,
            ))
            .await?;
        } else {
            ctx.say(format!("Playing song **{}**\n{}", title, source_url))
                .await?;
        }
    } else {
        ctx.say("Not in a voice channel to play in").await?;
    }

    Ok(())
}
