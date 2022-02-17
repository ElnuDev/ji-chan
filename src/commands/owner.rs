use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use serde_json::Map;
use serde_json::Value;
use serde_json::json;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::ShardManagerContainer;

const GUILD_DATA_PATH: &str = "guilds.json";

pub fn get_guild_data() -> Map<String, Value> {
    let guild_data_json = match File::open(GUILD_DATA_PATH) {
        Ok(mut file) => {
            let mut json = String::new();
            file.read_to_string(&mut json).unwrap();
            json
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut file = File::create(GUILD_DATA_PATH).unwrap();
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
        guild_data.insert(guild, json!({
            "submissionChannel": channel
        }));
    }
    set_guild_data(guild_data);
    // TODO: Add guild name in message
    msg.reply(&ctx.http, format!("Submission channel set to <#{}>.", msg.channel_id)).await?;
    Ok(())
}