use std::{sync::Arc, time::Duration};

use crate::{CommandResult, Context};

use poise::CreateReply;
use serenity::all::{Cache, ChannelId, ChannelType, GuildId, Http};
use serenity::async_trait;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::Songbird;
use tracing::{error, info};

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
                        match self.manager.remove(self.guild_id).await {
                            Ok(()) => info!(
                                "guild={} channel={} left automatically",
                                self.guild_id, self.voice_channel_id
                            ),
                            Err(err) => error!("failed to leave automatically: {}", err),
                        }
                    }
                }
            }
        }
        None
    }
}

pub async fn join_channel(ctx: Context<'_>) -> bool {
    let author = ctx.author();
    let serenity_ctx = ctx.serenity_context();
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            return false;
        }
    };
    let guild = match guild_id.channels(&serenity_ctx.http).await {
        Ok(channels) => channels,
        Err(_) => {
            return false;
        }
    };

    let voice_channel = guild
        .values()
        .filter(|channel| channel.kind == ChannelType::Voice)
        .find(|channel| {
            let members = match channel.members(&serenity_ctx.cache) {
                Ok(members) => members,
                Err(_) => {
                    return false;
                }
            };
            members
                .iter()
                .any(|member| member.user.id == author.id)
        });

    let voice_channel_id = match voice_channel {
        Some(channel) => channel.id,
        None => {
            return false;
        }
    };

    let manager = songbird::get(serenity_ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    match manager.join(guild_id, voice_channel_id).await {
        Ok(call) => {
            info!(
                "guild={} user(name=\"{}\",id={}) connected bot to voicechannel={}",
                guild_id, &author.name, &author.id, voice_channel_id
            );

            let mut handler = call.lock().await;
            handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
            handler.add_global_event(
                Event::Periodic(Duration::from_secs(60), None),
                AutoLeaveHandler {
                    manager,
                    guild_id,
                    voice_channel_id,
                    http: serenity_ctx.http.clone(),
                    cache: serenity_ctx.cache.clone(),
                },
            );
            true
        }
        Err(_) => false
    }
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
