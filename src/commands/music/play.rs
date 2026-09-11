use crate::components::player::{Player, queued_embed};
use crate::{CommandResult, Context, HttpKey};

use poise::CreateReply;
use serenity::all::CreateEmbed;

#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "url or search query"]
    #[rest]
    query: String,
) -> CommandResult {
    let Some(player) = Player::join(ctx).await else {
        ctx.send(
            CreateReply::default()
                .content("You are not in a voice channel, please join one.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let http_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let response = ctx
        .send(
            CreateReply::default().embed(
                CreateEmbed::default()
                    .title("Searching...")
                    .field("Query", &query, false),
            ),
        )
        .await?;

    let Some(track) = player.enqueue(http_client, &query, ctx.author()).await else {
        response
            .edit(
                ctx,
                CreateReply::default()
                    .embed(CreateEmbed::default().title("Unable to play your song, oops...")),
            )
            .await?;
        return Ok(());
    };

    response
        .edit(ctx, CreateReply::default().embed(queued_embed(&track)))
        .await?;

    player.post_to(ctx.channel_id()).await;
    Ok(())
}
