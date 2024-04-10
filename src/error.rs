use poise::serenity_prelude::{self as serenity, prelude::SerenityError};
use std::sync::Arc;
use tracing::warn;

use crate::Context;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BotError {
    SerenityError(Arc<SerenityError>),
}

impl From<serenity::Error> for BotError {
    fn from(value: serenity::Error) -> Self {
        BotError::SerenityError(Arc::new(value))
    }
}

impl BotError {
    pub async fn handle(&self, ctx: Context<'_>) {
        match self {
            BotError::SerenityError(err) => {
                warn!("Discord Error: {}", err);
                ctx.say("Error with the command, file an issue.").await.ok();
            }
        }
    }
}

impl std::fmt::Display for BotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotError::SerenityError(err) => write!(f, "Serenity error: {}", err),
        }
    }
}
