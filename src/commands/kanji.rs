use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use serde_json::Value;
use std::fs::File;
use std::io::Read;

use rand::seq::IteratorRandom;

fn random_from_string(string: &str) -> char {
    string.chars().choose(&mut rand::thread_rng()).unwrap()
}

async fn display_kanji(ctx: &Context, msg: &Message, kanji: char, comment: &str) -> CommandResult {
    msg.reply(&ctx.http, format!("{}{}", kanji, comment))
        .await?;
    let url = format!(
        "https://raw.githubusercontent.com/mistval/kanji_images/master/gifs/{}.gif",
        format!("{:x}", kanji as i32)
    );
    let client = reqwest::Client::new();
    let request = client.head(&url).build().unwrap();
    let response = client.execute(request).await?.status();
    let link_validated = response != reqwest::StatusCode::NOT_FOUND;
    msg.channel_id
        .say(
            &ctx.http,
            if link_validated {
                &url
            } else {
                "The stroke order diagram for this kanji is unavailable."
            },
        )
        .await?;
    Ok(())
}

fn get_lists_data() -> Value {
    let mut lists_file = File::open("kanji_lists.json").unwrap();
    let mut lists_json = String::new();
    lists_file.read_to_string(&mut lists_json).unwrap();
    let lists_data: Value = serde_json::from_str(&lists_json).unwrap();
    lists_data
}

async fn random_kanji(
    category: &str,
    ctx: &Context,
    msg: &Message,
    mut args: Args,
) -> CommandResult {
    let lists_data = get_lists_data();
    let category = &lists_data[category];
    let default_version = category["default"].as_str().unwrap();
    let list = &category["versions"][default_version]["characters"];
    match list.as_str() {
        Some(string) => {
            let kanji = random_from_string(string);
            display_kanji(&ctx, &msg, kanji, "").await?;
        }
        None => {
            let subcategory = args.single::<String>();
            let subcategories = list.as_object().unwrap();
            let subcategory_list = {
                let mut string = String::from("\n");
                let mut total_count = 0;
                for subcategory in subcategories.keys() {
                    let subcategory = subcategory.as_str();
                    // One has to do .chars().count() as opposed to .len() here,
                    // because .len() returns the byte length NOT the number of characters.
                    // For kanji, they are always multi-byte Unicode characters.
                    let count = &subcategories[subcategory].as_str().unwrap().chars().count();
                    total_count += count;
                    string.push_str(&format!("**{}** ({} kanji),\n", subcategory, count));
                }
                string.push_str(&format!(
                    "or **all** ({} kanji total) for all subcategories",
                    total_count
                ));
                string
            };
            match subcategory {
                Ok(string) => {
                    let string = string.to_uppercase();
                    if string == "ALL" {
                        let subcategory_key = subcategories
                            .keys()
                            .choose(&mut rand::thread_rng())
                            .unwrap();
                        let list = subcategories[subcategory_key].as_str().unwrap();
                        let kanji = random_from_string(&list);
                        display_kanji(&ctx, &msg, kanji, &format!(", **{}**", subcategory_key))
                            .await?;
                    } else if subcategories.contains_key(&string) {
                        let list = list[&string].as_str().unwrap();
                        let kanji = random_from_string(&list);
                        display_kanji(&ctx, &msg, kanji, "").await?;
                    } else {
                        let message = format!(
                            "That is an invalid subcategory. Please use {}.",
                            &subcategory_list
                        );
                        msg.reply(&ctx.http, message).await?;
                    }
                }
                Err(_) => {
                    let mut message = String::from("Please specify a subcategory: ");
                    message.push_str(&subcategory_list);
                    msg.reply(&ctx.http, message).await?;
                }
            }
        }
    }
    Ok(())
}

#[command]
async fn joyo(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    random_kanji("JOYO", ctx, msg, args).await
}

#[command]
async fn jinmeiyo(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    random_kanji("JINMEIYO", ctx, msg, args).await
}

#[command]
async fn kyoiku(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    random_kanji("KYOIKU", ctx, msg, args).await
}

#[command]
async fn jlpt(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    random_kanji("JLPT", ctx, msg, args).await
}

#[command]
async fn hyogai(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    random_kanji("HYOGAI", ctx, msg, args).await
}

#[command]
async fn so(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    const MAX_CHARS: i32 = 4;
    let text = args.rest();
    if text.is_empty() {
        msg.reply(
            &ctx.http,
            "Please provide some text you want the stroke order for.",
        )
        .await?;
        return Ok(());
    }
    let mut displayed_characters: Vec<char> = Vec::new();
    let mut displayed_character_count = 0;
    for character in text.chars() {
        if displayed_character_count >= MAX_CHARS {
            msg.channel_id
                .say(
                    &ctx.http,
                    ":warning: Maximum number of stroke order diagrams per command reached.",
                )
                .await?;
            break;
        }
        if character.is_whitespace() || displayed_characters.contains(&character) {
            continue;
        }
        // Don't show same character twice
        displayed_characters.push(character);
        displayed_character_count += 1;
        display_kanji(&ctx, &msg, character, "").await?;
    }
    Ok(())
}
