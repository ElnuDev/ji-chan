use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use serde_json::json;
use std::env;

use crate::utils::*;
use crate::ShardManagerContainer;

#[command]
#[owners_only]
async fn sleep(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    if let Some(manager) = data.get::<ShardManagerContainer>() {
        msg.reply(ctx, "Good night!").await?;
        manager.lock().await.shutdown_all().await;
    } else {
        msg.reply(ctx, "There was a problem getting the shard manager")
            .await?;

        return Ok(());
    }

    Ok(())
}

#[command]
#[owners_only]
#[allow(non_snake_case)]
async fn setSubmissionChannel(ctx: &Context, msg: &Message) -> CommandResult {
    let mut guild_data = get_guild_data();
    let guild = msg.guild_id.unwrap().as_u64().to_string();
    let channel = msg.channel_id.as_u64().to_string();
    if guild_data.contains_key(&guild) {
        let mut current_guild_data = guild_data[&guild].as_object().unwrap().clone();
        if current_guild_data.contains_key("submissionChannel") {
            current_guild_data["submissionChannel"] = channel.into();
        } else {
            current_guild_data.insert(String::from("submissionChannel"), channel.into());
        }
        guild_data[&guild] = current_guild_data.into();
    } else {
        guild_data.insert(guild, json!({ "submissionChannel": channel }));
    }
    set_guild_data(guild_data);
    msg.reply(
        &ctx.http,
        format!(
            "Submission channel for **{}** set to <#{}>.",
            msg.guild(&ctx).unwrap().name,
            msg.channel_id
        ),
    )
    .await?;
    Ok(())
}

#[command]
#[owners_only]
#[allow(non_snake_case)]
async fn setSuggestionChannel(ctx: &Context, msg: &Message) -> CommandResult {
    let channel = msg.channel_id.as_u64().to_string();
    let mut guild_data = get_guild_data();
    guild_data["suggestionChannel"] = channel.into();
    set_guild_data(guild_data);
    msg.reply(
        &ctx.http,
        format!("Submission channel set to <#{}>.", msg.channel_id),
    )
    .await?;
    Ok(())
}

#[command]
#[owners_only]
#[allow(non_snake_case)]
async fn setAnnouncementRole(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let role;
    match args.single::<u64>() {
        Ok(id) => role = id.to_string(),
        Err(_) => {
            msg.reply(&ctx.http, "Please provide an announcement role ID.")
                .await?;
            return Ok(());
        }
    }
    let mut guild_data = get_guild_data();
    let guild = msg.guild_id.unwrap().as_u64().to_string();
    if guild_data.contains_key(&guild) {
        let mut current_guild_data = guild_data[&guild].as_object().unwrap().clone();
        if current_guild_data.contains_key("announcementRole") {
            current_guild_data["announcementRole"] = role.into();
        } else {
            current_guild_data.insert(String::from("announcementRole"), role.into());
        }
        guild_data[&guild] = current_guild_data.into();
    } else {
        guild_data.insert(guild, json!({ "announcementRole": role }));
    }
    set_guild_data(guild_data);
    msg.reply(&ctx.http, "Announcement role set.").await?;
    Ok(())
}

#[command]
#[owners_only]
async fn announce(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    send(ctx, msg, args.rest(), true, false).await
}

#[command]
#[owners_only]
#[allow(non_snake_case)]
async fn announceChallenge(ctx: &Context, msg: &Message) -> CommandResult {
    let challenge_number = get_challenge_number();
    let message = format!("Welcome to the **{n}{th}** weekly **Tegaki Tuesday** (手書きの火曜日) handwriting challenge! :pen_fountain: The prompt is available in both Japanese and English on the website at <https://tegakituesday.com/{n}>.

You can make submissions in both languages, but please submit in your target language first. Submissions can be submitted by uploading the image to this channel along with the `{p}submit` command. By submitting, you agree to having your work posted to the website under the Attribution-ShareAlike 4.0 Unported (CC BY-SA 4.0) license, attributed to your Discord account. (<https://creativecommons.org/licenses/by-sa/4.0>).",
        n = challenge_number,
        th = match challenge_number % 10 {
            1 => "ˢᵗ",
            2 => "ⁿᵈ",
            3 => "ʳᵈ",
            _ => "ᵗʰ"
        },
        p = env::var("PREFIX").unwrap()
    );
    send(ctx, msg, &message, true, false).await
}

#[command]
#[owners_only]
#[allow(non_snake_case)]
async fn rebuildSite(ctx: &Context, msg: &Message) -> CommandResult {
    rebuild_site();
    msg.reply(&ctx.http, "Started site rebuild process!")
        .await?;
    Ok(())
}
