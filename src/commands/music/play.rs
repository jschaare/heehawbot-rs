use crate::{commands::music::join, CommandResult, Context, HttpKey};

use poise::CreateReply;
use serenity::all::{CreateEmbed, CreateEmbedFooter};
use songbird::input::YoutubeDl;
use tracing::info;
use url::Url;

#[poise::command(slash_command, prefix_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "url or search query"]
    #[rest]
    query: String,
) -> CommandResult {
    let author = ctx.author();
    let serenity_ctx = ctx.serenity_context();
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("Can only use `/play` inside a guild").await?;
            return Ok(());
        }
    };

    let http_client = {
        let data = serenity_ctx.data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(serenity_ctx)
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

        let response = ctx
            .send(
                CreateReply::default().embed(
                    CreateEmbed::default()
                        .title("Searching...")
                        .field("Query", &query, false),
                ),
            )
            .await?;

        let src = if let Ok(url) = Url::parse(&query) {
            YoutubeDl::new(http_client, url.to_string())
        } else {
            YoutubeDl::new_search(http_client, query)
        };

        let mut src: songbird::input::Input = src.clone().into();

        // extract metadata about song
        let aux_metadata = match src.aux_metadata().await {
            Ok(metadata) => metadata,
            Err(_e) => {
                ctx.say("Unable to play your song, oops...").await?;
                return Ok(());
            }
        };
        let title = match aux_metadata.title {
            Some(t) => t,
            None => "Unknown".to_string(),
        };
        let source_url = match aux_metadata.source_url {
            Some(url) => url,
            None => "".to_string(),
        };
        let thumbnail_url = match aux_metadata.thumbnail {
            Some(thumbnail) => thumbnail,
            None => "".to_string(),
        };
        let author_name = match &author.global_name {
            Some(name) => name,
            None => &author.name,
        };
        let author_icon_url = match author.avatar_url() {
            Some(url) => url,
            None => "".to_string(),
        };

        info!(
            "guild={} user(name=\"{}\",id={}) queued url=({})",
            guild_id, &author.name, &author.id, source_url
        );

        // enqueue using songbird built-in queue
        handler.enqueue_input(src).await;

        response
            .edit(
                ctx,
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
