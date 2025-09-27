use crate::{commands::music::join, CommandResult, Context, HttpKey};

use poise::CreateReply;
use reqwest::Client;
use serenity::all::{CreateEmbed, CreateEmbedFooter};
use songbird::input::{Compose, Input, LiveInput, YoutubeDl};
use tracing::{error, info};
use url::Url;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

async fn get_soundclound_stream(url: &str) -> Result<String, Error> {
    let output = tokio::process::Command::new("yt-dlp")
        .args([
            "--no-playlist",
            "-f",
            "http_mp3_1_0/http_mp3/best[protocol=http]",
            "--get-url",
            "--extractor-args",
            "soundcloud:force_api_v2",
            url,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "failed to extract url: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let stream_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    info!("extracted : {}", stream_url);
    Ok(stream_url)
}

pub fn is_soundcloud(u: &Url) -> bool {
    match u.domain() {
        Some(domain) => {
            error!(domain);
            domain.contains("soundcloud.com")
                || domain.contains("snd.sc")
                || domain.contains("sndcdn.com")
        }
        None => false,
    }
}

async fn get_input(query: String, http_client: Client) -> Result<Input, Error> {
    if let Ok(url) = Url::parse(&query) {
        if is_soundcloud(&url) {
            let stream_url = get_soundclound_stream(url.to_string().as_str()).await?;
            // let mut source = YoutubeDl::new_search(http_client, url.to_string()).user_args(vec![
            let mut ytdl = YoutubeDl::new(http_client, stream_url).user_args(vec![
                "--no-playlist".into(),
                "--socket-timeout".into(),
                "60".into(),
                "--retries".into(),
                "5".into(),
            ]);
            let audio = ytdl.create_async().await.map_err(Error::from)?;
            return Ok(Input::Live(LiveInput::Raw(audio), Some(Box::new(ytdl))));
        } else {
            let mut ytdl = YoutubeDl::new(http_client, url.to_string());
            return Ok(Input::Live(
                LiveInput::Raw(ytdl.create_async().await.map_err(Error::from)?),
                Some(Box::new(ytdl)),
            ));
        }
    }

    let mut ytdl = YoutubeDl::new_search(http_client, query).user_args(vec![
        "--no-playlist".into(),
        "-f".into(),
        "http_mp3/best[protocol!=m3u8][protocol!=hls]/bestaudio/best".into(),
        "--extractor-args".into(),
        "soundcloud:force_api_v2".into(),
        "--no-check-certificates".into(),
        "--prefer-free-formats".into(),
        "--socket-timeout".into(),
        "30".into(),
        "--retries".into(),
        "3".into(),
    ]);
    let audio = ytdl.create_async().await.map_err(Error::from)?;
    Ok(Input::Live(LiveInput::Raw(audio), Some(Box::new(ytdl))))
}

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

        let mut src = match get_input(query, http_client).await {
            Ok(src) => src,
            Err(e) => {
                error!(error = %e);
                ctx.say("Unable to play your song, oops...").await?;
                return Ok(());
            }
        };

        // extract metadata about song
        let aux_metadata = match src.aux_metadata().await {
            Ok(metadata) => metadata,
            Err(e) => {
                error!(error = %e);
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
