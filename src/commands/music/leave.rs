use serenity::{
    client::Context,
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
};

#[command]
#[only_in(guilds)]
pub async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if manager.get(guild_id).is_some() {
        manager.remove(guild_id).await.expect("Failed to leave");
        msg.reply(ctx, "Left voice channel").await?;
    }

    Ok(())
}
