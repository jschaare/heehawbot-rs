use crate::{commands::music::join, CommandResult, Context, HttpKey};

use poise::CreateReply;
use serenity::all::{CreateEmbed, CreateEmbedFooter};
use songbird::input::YoutubeDl;

#[poise::command(slash_command, prefix_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "url or search query"]
    #[rest]
    query: String,
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
    if !join::join_channel(ctx).await {
        ctx.send(
            CreateReply::default()
                .content("You are not in a voice channel, please join one.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let src = if !query.starts_with("http") {
            // just search for all the args
            ctx.send(
                CreateReply::default()
                    .content(format!("Searching for \"**{}**\"!", query))
                    .ephemeral(true),
            )
            .await?;
            YoutubeDl::new_search(http_client, query)
        } else {
            YoutubeDl::new(http_client, query)
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
        let thumbnail_url = match aux_metadata.thumbnail {
            Some(thumbnail) => thumbnail,
            None => "".to_string(),
        };
        let author = ctx.author();
        let author_name = match &author.global_name {
            Some(name) => name,
            None => &author.name,
        };
        let author_icon_url = match author.avatar_url() {
            Some(url) => url,
            None => "".to_string(),
        };

        // enqueue using songbird built-in queue
        handler.enqueue_input(src).await;

        ctx.send(
            CreateReply::default().embed(
                CreateEmbed::default()
                    .title(title)
                    .url(source_url)
                    .thumbnail(thumbnail_url)
                    .footer(
                        CreateEmbedFooter::new(format!("Queued by {author_name}"))
                            .icon_url(author_icon_url),
                    ),
            ),
        )
        .await?;
    } else {
        ctx.say("Unable to play your song, oops...").await?;
    }

    Ok(())
}
