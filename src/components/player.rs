use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::Duration;

use poise::serenity_prelude as serenity;
use reqwest::Client as HttpClient;
use serenity::all::{
    ButtonStyle, Cache, ChannelId, ChannelType, ComponentInteraction, CreateActionRow,
    CreateButton, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse, CreateMessage,
    EditMessage, GuildId, Http, MessageId, User,
};
use serenity::async_trait;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::input::YoutubeDl;
use songbird::tracks::{PlayMode, Track};
use songbird::{Call, Songbird};
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use url::Url;

use crate::{Context, Result};

/// Limit the displayed number of upcoming songs to avoid overflowing the message
const UP_NEXT_LIMIT: usize = 5;

/// Metadata used to update the player message
#[derive(Debug)]
pub struct QueuedTrack {
    pub title: String,
    pub url: String,
    pub thumbnail: String,
    pub requester_name: String,
    pub requester_icon: String,
}

/// Registry of live players, one per guild
pub type Players = Arc<Mutex<HashMap<GuildId, Arc<Player>>>>;

pub struct Player {
    guild_id: GuildId,
    call: Arc<Mutex<Call>>,
    message: Mutex<Option<(ChannelId, MessageId)>>,
    manager: Arc<Songbird>,
    players: Players,
    http: Arc<Http>,
    cache: Arc<Cache>,
}

impl Player {
    pub async fn get(ctx: Context<'_>) -> Option<Arc<Player>> {
        let guild_id = ctx.guild_id()?;
        let players = ctx.data().players.lock().await;
        players.get(&guild_id).cloned()
    }

    /// Join the player to the author's voice channel
    pub async fn join(ctx: Context<'_>) -> Option<Arc<Player>> {
        let guild_id = ctx.guild_id()?;
        let author = ctx.author();
        let serenity_ctx = ctx.serenity_context();
        let manager = songbird::get(serenity_ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        let channels = guild_id.channels(&serenity_ctx.http).await.ok()?;
        let voice_channel_id = channels
            .values()
            .filter(|channel| channel.kind == ChannelType::Voice)
            .find(|channel| match channel.members(&serenity_ctx.cache) {
                Ok(members) => members.iter().any(|member| member.user.id == author.id),
                Err(_) => false,
            })?
            .id;

        let existing = ctx.data().players.lock().await.get(&guild_id).cloned();
        if let Some(player) = existing {
            let current_channel = player.call.lock().await.current_channel();
            if current_channel != Some(voice_channel_id.into()) {
                manager.join(guild_id, voice_channel_id).await.ok()?;
            }
            return Some(player);
        }

        let call = match manager.join(guild_id, voice_channel_id).await {
            Ok(call) => call,
            Err(err) => {
                error!("guild={guild_id} failed to join voice channel: {err}");
                return None;
            }
        };
        info!(
            "guild={} user(name=\"{}\",id={}) connected bot to voicechannel={}",
            guild_id, &author.name, &author.id, voice_channel_id
        );

        let player = Arc::new(Player {
            guild_id,
            call: call.clone(),
            message: Mutex::new(None),
            manager,
            players: ctx.data().players.clone(),
            http: serenity_ctx.http.clone(),
            cache: serenity_ctx.cache.clone(),
        });

        // Weak pointer to avoid dependency cycle
        let weak = Arc::downgrade(&player);
        {
            let mut handler = call.lock().await;
            handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
            handler.add_global_event(
                TrackEvent::Play.into(),
                RefreshPlayer {
                    player: weak.clone(),
                    ended: false,
                },
            );
            handler.add_global_event(
                TrackEvent::End.into(),
                RefreshPlayer {
                    player: weak.clone(),
                    ended: true,
                },
            );
            handler.add_global_event(
                Event::Periodic(Duration::from_secs(60), None),
                AutoLeave(weak),
            );
        }

        ctx.data()
            .players
            .lock()
            .await
            .insert(guild_id, player.clone());
        Some(player)
    }

    /// Resolve the posted track and add to queue if possible
    pub async fn enqueue(
        &self,
        http_client: HttpClient,
        query: &str,
        requester: &User,
    ) -> Option<Arc<QueuedTrack>> {
        let source = match Url::parse(query) {
            Ok(url) => YoutubeDl::new(http_client, url.to_string()),
            Err(_) => YoutubeDl::new_search(http_client, query.to_string()),
        };
        let mut input: songbird::input::Input = source.into();

        let metadata = match input.aux_metadata().await {
            Ok(metadata) => metadata,
            Err(err) => {
                error!("could not find metadata for query={query} error={err}");
                return None;
            }
        };

        let track = Arc::new(QueuedTrack {
            title: metadata.title.unwrap_or_else(|| "Unknown".to_string()),
            url: metadata.source_url.unwrap_or_default(),
            thumbnail: metadata.thumbnail.unwrap_or_default(),
            requester_name: requester
                .global_name
                .clone()
                .unwrap_or_else(|| requester.name.clone()),
            requester_icon: requester.avatar_url().unwrap_or_default(),
        });

        info!(
            "guild={} user(name=\"{}\",id={}) queued url=({})",
            self.guild_id, &requester.name, &requester.id, track.url
        );

        self.call
            .lock()
            .await
            .enqueue(Track::new_with_data(input, track.clone()))
            .await;
        Some(track)
    }

    pub async fn queue(&self) -> Vec<Arc<QueuedTrack>> {
        self.tracks(None).await
    }

    async fn tracks(&self, exclude: Option<u128>) -> Vec<Arc<QueuedTrack>> {
        self.call
            .lock()
            .await
            .queue()
            .current_queue()
            .iter()
            .filter(|handle| exclude != Some(handle.uuid().as_u128()))
            .map(|handle| handle.data::<QueuedTrack>())
            .collect()
    }

    pub async fn is_paused(&self) -> bool {
        let current = self.call.lock().await.queue().current();
        match current {
            Some(handle) => {
                matches!(handle.get_info().await, Ok(state) if state.playing == PlayMode::Pause)
            }
            None => false,
        }
    }

    pub async fn skip(&self) {
        if let Err(err) = self.call.lock().await.queue().skip() {
            warn!("guild={} failed to skip: {}", self.guild_id, err);
        }
    }

    pub async fn pause(&self) {
        if let Err(err) = self.call.lock().await.queue().pause() {
            warn!("guild={} failed to pause: {}", self.guild_id, err);
        }
    }

    pub async fn resume(&self) {
        if let Err(err) = self.call.lock().await.queue().resume() {
            warn!("guild={} failed to resume: {}", self.guild_id, err);
        }
    }

    pub async fn clear(&self) {
        self.call.lock().await.queue().stop();
    }

    pub async fn leave(&self) {
        let message = self.message.lock().await.take();
        if let Some((channel_id, message_id)) = message {
            channel_id.delete_message(&self.http, message_id).await.ok();
        }
        self.players.lock().await.remove(&self.guild_id);
        match self.manager.remove(self.guild_id).await {
            Ok(()) => info!("guild={} left voice channel", self.guild_id),
            Err(err) => error!("guild={} failed to leave: {}", self.guild_id, err),
        }
    }

    /// Posts a new player message and deletes the old one
    pub async fn post_to(&self, channel_id: ChannelId) {
        self.post(channel_id, None).await;
    }

    /// Deletes the player message and posts it again, intended to constantly keep
    /// the message visible when queue updates occur
    pub async fn repost(&self, exclude: Option<u128>) {
        let current = *self.message.lock().await;
        if let Some((channel_id, _)) = current {
            self.post(channel_id, exclude).await;
        }
    }

    /// Edits the player message without a full swap
    pub async fn refresh(&self, exclude: Option<u128>) {
        let Some((channel_id, message_id)) = *self.message.lock().await else {
            return;
        };
        let (embed, components) = self.render(exclude).await;
        if let Err(err) = channel_id
            .edit_message(
                &self.http,
                message_id,
                EditMessage::new().embed(embed).components(components),
            )
            .await
        {
            warn!("guild={} player message gone: {}", self.guild_id, err);
            self.post(channel_id, exclude).await;
        }
    }

    async fn post(&self, channel_id: ChannelId, exclude: Option<u128>) {
        let (embed, components) = self.render(exclude).await;

        let mut message = self.message.lock().await;
        if let Some((old_channel, old_message)) = message.take() {
            old_channel
                .delete_message(&self.http, old_message)
                .await
                .ok();
        }
        match channel_id
            .send_message(
                &self.http,
                CreateMessage::new().embed(embed).components(components),
            )
            .await
        {
            Ok(posted) => *message = Some((channel_id, posted.id)),
            Err(err) => warn!("guild={} failed to post player: {}", self.guild_id, err),
        }
    }

    /// Generates an embed to show the current state of the player queue, also has basic controls
    async fn render(&self, exclude: Option<u128>) -> (CreateEmbed, Vec<CreateActionRow>) {
        let tracks = self.tracks(exclude).await;
        let paused = self.is_paused().await;

        let embed = match tracks.first() {
            Some(current) => {
                let mut embed = CreateEmbed::default()
                    .title(format!(
                        "{} {}",
                        if paused { "⏸️" } else { "▶️" },
                        current.title
                    ))
                    .footer(
                        CreateEmbedFooter::new(format!("Requested by {}", current.requester_name))
                            .icon_url(&current.requester_icon),
                    );
                if !current.url.is_empty() {
                    embed = embed.url(&current.url);
                }
                if !current.thumbnail.is_empty() {
                    embed = embed.thumbnail(&current.thumbnail);
                }
                if let Some(upcoming) = up_next(&tracks) {
                    embed = embed.field("Up next", upcoming, false);
                }
                embed
            }
            None => CreateEmbed::default().title("Nothing playing"),
        };

        let idle = tracks.is_empty();
        let buttons = CreateActionRow::Buttons(vec![
            button("player:pause", "Pause", ButtonStyle::Secondary, idle),
            button("player:play", "Play", ButtonStyle::Secondary, idle),
            button("player:skip", "Skip", ButtonStyle::Secondary, idle),
            button("player:clear", "Clear", ButtonStyle::Danger, idle),
        ]);

        (embed, vec![buttons])
    }
}

fn button(id: &str, label: &str, style: ButtonStyle, disabled: bool) -> CreateButton {
    CreateButton::new(id)
        .label(label)
        .style(style)
        .disabled(disabled)
}

fn up_next(tracks: &[Arc<QueuedTrack>]) -> Option<String> {
    let upcoming = tracks.get(1..)?;
    if upcoming.is_empty() {
        return None;
    }

    let mut listing = String::new();
    for (index, track) in upcoming.iter().take(UP_NEXT_LIMIT).enumerate() {
        if track.url.is_empty() {
            listing.push_str(&format!("{}. {}\n", index + 1, track.title));
        } else {
            listing.push_str(&format!(
                "{}. [{}]({})\n",
                index + 1,
                track.title,
                track.url
            ));
        }
    }
    if upcoming.len() > UP_NEXT_LIMIT {
        listing.push_str(&format!("+{} more", upcoming.len() - UP_NEXT_LIMIT));
    }
    Some(listing)
}

/// Embed for displaying info about newly queued tracks
pub fn queued_embed(track: &QueuedTrack) -> CreateEmbed {
    let mut embed = CreateEmbed::default().title(&track.title).footer(
        CreateEmbedFooter::new(format!("Queued by {}", track.requester_name))
            .icon_url(&track.requester_icon),
    );
    if !track.url.is_empty() {
        embed = embed.url(&track.url);
    }
    if !track.thumbnail.is_empty() {
        embed = embed.thumbnail(&track.thumbnail);
    }
    embed
}

/// Handles the player embed's buttons.
pub async fn handle_component(
    ctx: &serenity::Context,
    interaction: &ComponentInteraction,
    players: &Players,
) -> Result<()> {
    // Acknowledge the interaction then build the response
    interaction
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await?;

    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };
    let Some(player) = players.lock().await.get(&guild_id).cloned() else {
        return Ok(());
    };

    match interaction.data.custom_id.as_str() {
        "player:pause" => player.pause().await,
        "player:play" => player.resume().await,
        "player:skip" => player.skip().await,
        "player:clear" => player.clear().await,
        other => warn!("unknown player button: {other}"),
    }
    player.refresh(None).await;
    Ok(())
}

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                error!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }
        None
    }
}

/// Reposts the player when a song starts, redraws it in place when one finishes.
struct RefreshPlayer {
    player: Weak<Player>,
    ended: bool,
}

#[async_trait]
impl VoiceEventHandler for RefreshPlayer {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let player = self.player.upgrade()?;
        if !self.ended {
            // Repost the player message
            player.repost(None).await;
            return None;
        }
        let exclude = match ctx {
            EventContext::Track(track_list) => track_list
                .first()
                .map(|(_, handle)| handle.uuid().as_u128()),
            _ => None,
        };
        player.refresh(exclude).await;
        None
    }
}

/// Disconnects once the bot is alone
struct AutoLeave(Weak<Player>);

#[async_trait]
impl VoiceEventHandler for AutoLeave {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let player = self.0.upgrade()?;
        let voice_channel_id = player.call.lock().await.current_channel()?;
        let voice_channel_id = ChannelId::new(voice_channel_id.0.get());

        if let Ok(channel) = player.http.get_channel(voice_channel_id).await
            && let Some(guild_channel) = channel.guild()
            && let Ok(members) = guild_channel.members(&player.cache)
            && members.len() <= 1
        {
            player.leave().await;
        }
        None
    }
}
