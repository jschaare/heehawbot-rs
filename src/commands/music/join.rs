use std::{sync::Arc, time::Duration};

use crate::{CommandResult, Context};

use poise::CreateReply;
use serenity::all::{Cache, ChannelId, GuildId, Http};
use serenity::async_trait;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::Songbird;

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

struct AutoLeaveHandler {
    manager: Arc<Songbird>,
    guild_id: GuildId,
    voice_channel_id: ChannelId,
    http: Arc<Http>,
    cache: Arc<Cache>,
}

#[async_trait]
impl VoiceEventHandler for AutoLeaveHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        if let Ok(channel) = self.http.get_channel(self.voice_channel_id).await {
            if let Some(guild_channel) = channel.guild() {
                if let Ok(members) = guild_channel.members(&self.cache) {
                    if members.len() <= 1 {
                        if let Err(err) = self.manager.remove(self.guild_id).await {
                            println!("Failed to leave after track end: {err}");
                        }
                    }
                }
            }
        }
        None
    }
}

pub async fn join_channel(ctx: Context<'_>) -> bool {
    let guild_id = ctx.guild_id().unwrap();
    let voice_channel = ctx
        .guild()
        .unwrap()
        .voice_states
        .get(&ctx.author().id)
        .and_then(|v| v.channel_id);

    let channel_id = match voice_channel {
        Some(channel) => channel,
        None => {
            return false;
        }
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Ok(handler_lock) = manager.join(guild_id, channel_id).await {
        // Attach an event handler to see notifications of all track errors.
        let mut handler = handler_lock.lock().await;
        let handler_http = ctx.serenity_context().http.clone();
        let handler_cache = ctx.serenity_context().cache.clone();

        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        handler.add_global_event(
            Event::Periodic(Duration::from_secs(60), None),
            AutoLeaveHandler {
                manager,
                guild_id,
                voice_channel_id: channel_id,
                http: handler_http,
                cache: handler_cache,
            },
        )
    }

    return true;
}

#[poise::command(slash_command, prefix_command, ephemeral)]
pub async fn join(ctx: Context<'_>) -> CommandResult {
    let join_msg = if join_channel(ctx).await {
        "Joined voice channel!"
    } else {
        "You are not in a voice channel, please join one."
    };
    ctx.send(CreateReply::default().content(join_msg).ephemeral(true))
        .await?;
    Ok(())
}
