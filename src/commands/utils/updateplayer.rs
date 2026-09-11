use crate::{CommandResult, Context};

#[poise::command(slash_command, prefix_command, owners_only, ephemeral)]
pub async fn updateplayer(ctx: Context<'_>) -> CommandResult {
    ctx.defer_ephemeral().await?;
    let msg = match tokio::process::Command::new("yt-dlp")
        .arg("-U")
        .output()
        .await
    {
        Ok(out) => format!(
            "```\n{}{}\n```",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(e) => format!("Failed to run yt-dlp: {e}"),
    };
    ctx.say(msg).await?;

    Ok(())
}
