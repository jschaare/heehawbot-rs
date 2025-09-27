use crate::{CommandResult, Context, HttpKey};
use poise::CreateReply;
use serde::{Deserialize, Serialize};
use serenity::all::CreateEmbed;

#[derive(Debug, Serialize)]
struct ChallengeOpenRequest {
    rated: bool,
    variant: String,
    #[serde(rename = "clock.limit")]
    clock_limit: u32,
    #[serde(rename = "clock.increment")]
    clock_increment: u32,
}

#[derive(Debug, Deserialize)]
struct ChallengeOpenResponse {
    url: String,
    #[serde(rename = "urlBlack")]
    url_black: String,
    #[serde(rename = "urlWhite")]
    url_white: String,
}

#[poise::command(slash_command, prefix_command)]
pub async fn play_chess(ctx: Context<'_>) -> CommandResult {
    let serenity_ctx = ctx.serenity_context();

    let http_client = {
        let data = serenity_ctx.data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let challenge_open_request = ChallengeOpenRequest {
        rated: false,
        variant: "standard".to_string(),
        clock_limit: 600,
        clock_increment: 0,
    };

    let res = http_client
        .post("https://lichess.org/api/challenge/open")
        .json(&challenge_open_request)
        .send()
        .await
        .expect("Couldn't send message to lichess");

    let challenge_open_response = &res
        .json::<ChallengeOpenResponse>()
        .await
        .expect("Could not retrieve response from lichess");

    ctx.send(
        CreateReply::default().embed(
            CreateEmbed::default()
                .title("Lichess Game Created")
                .url(&challenge_open_response.url)
                .field("Join as White", &challenge_open_response.url_white, false)
                .field("Join as Black", &challenge_open_response.url_black, false),
        ),
    )
    .await?;

    Ok(())
}
