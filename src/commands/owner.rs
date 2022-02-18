use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::env;

use crate::ShardManagerContainer;
use crate::commands::challenge::get_challenge_number;

pub fn get_guild_data() -> Map<String, Value> {
    let guild_data_path = env::var("GUILD_DATA").unwrap();
    let guild_data_json = match File::open(guild_data_path) {
        Ok(mut file) => {
            let mut json = String::new();
            file.read_to_string(&mut json).unwrap();
            json
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut file = File::create(guild_data_path).unwrap();
            file.write_all(b"{}").unwrap();
            file.flush().unwrap();
            String::from("{}")
        }
        Err(_) => panic!("Failed to open guild data file"),
    };
    let mut submission_data: Value = serde_json::from_str(&guild_data_json).unwrap();
    submission_data.as_object_mut().unwrap().clone()
}

fn set_guild_data(guild_data: Map<String, Value>) {
    let guild_data: Value = guild_data.into();
    let mut guild_data_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(GUILD_DATA_PATH)
        .unwrap();
    guild_data_file
        .write_all(
            serde_json::to_string_pretty(&guild_data)
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
}

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
        format!("Submission channel for **{}** set to <#{}>.", msg.guild(&ctx).await.unwrap().name, msg.channel_id),
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
            msg.reply(&ctx.http, "Please provide an announcement role ID.").await?;
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

async fn send(ctx: &Context, msg: &Message, message: &str, ping: bool, pin: bool) -> CommandResult {
    let guild_data = get_guild_data();
    let mut announcements_count = 0;
    for (_guild, data) in guild_data.iter() {
        let data = data.as_object().unwrap();
        if !data.contains_key("submissionChannel") {
            continue;
        }
        let channel = ChannelId(data["submissionChannel"].as_str().unwrap().parse::<u64>().unwrap());
        let mut message_to_send = String::from("");
        if ping && data.contains_key("announcementRole") {
            message_to_send.push_str(&format!("<@&{}> ", data["announcementRole"].as_str().unwrap()));
        }
        message_to_send.push_str(message);
        let sent_message = channel.send_message(&ctx.http, |e| {
            e.content(message_to_send);
            e
        }).await.unwrap();
        if pin {
            // No need to do anything on error,
            // it just means we don't have pin permissions
            match sent_message.pin(&ctx.http).await {
                Ok(_) => (),
                Err(_) => ()
            };
        }
        announcements_count += 1;
    }
    msg.reply(&ctx.http, format!("Announced to {} server{}!", announcements_count, if announcements_count == 1 { "" } else { "s" })).await?;
    Ok(())
}

#[command]
#[owners_only]
async fn announce(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    send(ctx, msg, args.rest(), true, true).await
}

#[command]
#[owners_only]
#[allow(non_snake_case)]
async fn announceChallenge(ctx: &Context, msg: &Message) -> CommandResult {
    let challenge_number = get_challenge_number();
    let message = format!("Welcome to the **{n}{th}** weekly **Tegaki Tuesday** (手書きの火曜日) handwriting challenge! :pen_fountain: The prompt is available in both Japanese and English on the website at <https://tegakituesday/{n}>.

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
    send(ctx, msg, &message, true, true).await
}