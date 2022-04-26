use rand::seq::IteratorRandom;
use serde_json::Map;
use serde_json::Value;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::process::Command;

pub fn get_challenge_number() -> i32 {
    let challenge_dir = format!("{}/content/challenges", env::var("HUGO").unwrap());
    let paths = fs::read_dir(challenge_dir).unwrap();
    let mut max = 0;
    for path in paths {
        let number = path
            .unwrap()
            .path()
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<i32>()
            .unwrap();
        if number > max {
            max = number;
        }
    }
    max
}

pub fn get_hugo_path() -> String {
    env::var("HUGO").unwrap()
}

pub fn get_submission_images_dir() -> String {
    format!("{}/assets/{}", get_hugo_path(), get_challenge_number())
}

pub fn get_submission_data_path(challenge: i32) -> String {
    format!("{}/data/challenges/{}.json", get_hugo_path(), challenge)
}

pub fn get_current_submission_data_path() -> String {
    get_submission_data_path(get_challenge_number())
}

pub fn get_current_submission_data() -> Vec<Value> {
    get_submission_data(get_challenge_number())
}

pub fn get_submission_data(challenge: i32) -> Vec<Value> {
    let submission_data_path = get_submission_data_path(challenge);
    let submission_data_json = match File::open(&submission_data_path) {
        Ok(mut file) => {
            let mut json = String::new();
            file.read_to_string(&mut json).unwrap();
            json
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut file = File::create(&submission_data_path).unwrap();
            file.write_all(b"[]").unwrap();
            file.flush().unwrap();
            String::from("[]")
        }
        Err(_) => panic!("Failed to open submission data file"),
    };
    let mut submission_data: Value = serde_json::from_str(&submission_data_json).unwrap();
    submission_data.as_array_mut().unwrap().clone()
}

pub fn set_submission_data(submission_data: Vec<Value>) {
    let submission_data: Value = submission_data.into();
    let mut submission_data_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(get_current_submission_data_path())
        .unwrap();
    submission_data_file
        .write_all(
            serde_json::to_string_pretty(&submission_data)
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
}

pub fn is_matching_submission(submission: &Value, msg: &Message) -> bool {
    submission["id"].as_str().unwrap() == msg.author.id.as_u64().to_string()
}

pub fn to_fullwidth(string: &str) -> String {
    let mut fullwidth = String::new();
    for character in string.chars() {
        fullwidth.push(match unicode_hfwidth::to_fullwidth(character) {
            Some(character) => character,
            None => character,
        });
    }
    fullwidth
}

pub fn rebuild_site() {
    Command::new("./build.sh")
        .current_dir(get_hugo_path())
        .spawn()
        .expect("Failed to rebuild site");
}

pub fn get_guild_data_path() -> String {
    env::var("GUILD_DATA").unwrap()
}

pub fn get_guild_data() -> Map<String, Value> {
    let guild_data_path = get_guild_data_path();
    let guild_data_json = match File::open(&guild_data_path) {
        Ok(mut file) => {
            let mut json = String::new();
            file.read_to_string(&mut json).unwrap();
            json
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut file = File::create(&guild_data_path).unwrap();
            file.write_all(b"{}").unwrap();
            file.flush().unwrap();
            String::from("{}")
        }
        Err(_) => panic!("Failed to open guild data file"),
    };
    let mut submission_data: Value = serde_json::from_str(&guild_data_json).unwrap();
    submission_data.as_object_mut().unwrap().clone()
}

pub fn set_guild_data(guild_data: Map<String, Value>) {
    let guild_data: Value = guild_data.into();
    let mut guild_data_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(get_guild_data_path())
        .unwrap();
    guild_data_file
        .write_all(
            serde_json::to_string_pretty(&guild_data)
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
}

pub async fn send(
    ctx: &Context,
    msg: &Message,
    message: &str,
    ping: bool,
    pin: bool,
) -> CommandResult {
    let guild_data = get_guild_data();
    let mut announcements_count = 0;
    for (_guild, data) in guild_data.iter() {
        let data = data.as_object().unwrap();
        if !data.contains_key("submissionChannel") {
            continue;
        }
        let channel = ChannelId(
            data["submissionChannel"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
        );
        let mut message_to_send = String::from("");
        if ping && data.contains_key("announcementRole") {
            message_to_send.push_str(&format!(
                "<@&{}> ",
                data["announcementRole"].as_str().unwrap()
            ));
        }
        message_to_send.push_str(message);
        let sent_message = channel
            .send_message(&ctx.http, |e| {
                e.content(message_to_send);
                e
            })
            .await
            .unwrap();
        if pin {
            // No need to do anything on error,
            // it just means we don't have pin permissions
            match sent_message.pin(&ctx.http).await {
                Ok(_) => (),
                Err(_) => (),
            };
        }
        announcements_count += 1;
    }
    msg.reply(
        &ctx.http,
        format!(
            "Announced to {} server{}!",
            announcements_count,
            if announcements_count == 1 { "" } else { "s" }
        ),
    )
    .await?;
    Ok(())
}

pub fn random_from_string(string: &str) -> char {
    string.chars().choose(&mut rand::thread_rng()).unwrap()
}

pub fn get_so_diagram(kanji: char) -> String {
    format!(
        "https://raw.githubusercontent.com/mistval/kanji_images/master/gifs/{}.gif",
        format!("{:x}", kanji as i32)
    )
}

pub async fn display_kanji(
    ctx: &Context,
    msg: &Message,
    kanji: char,
    comment: &str,
) -> CommandResult {
    msg.reply(&ctx.http, format!("{}{}", kanji, comment))
        .await?;
    let url = get_so_diagram(kanji);
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

pub fn get_lists_data() -> Map<String, Value> {
    let mut lists_file = File::open("kanji_lists.json").unwrap();
    let mut lists_json = String::new();
    lists_file.read_to_string(&mut lists_json).unwrap();
    let lists_data: Value = serde_json::from_str(&lists_json).unwrap();
    lists_data.as_object().unwrap().clone()
}

pub fn get_kanji_info(kanji: char) -> String {
    let lists_data = get_lists_data();
    let mut info = String::from("");
    for category_name in lists_data.keys() {
        let category = &lists_data[category_name];
        let default_version = category["default"].as_str().unwrap();
        match &category["versions"][default_version]["characters"].as_str() {
            // if no variants (string)
            Some(list) => {
                if list.contains(kanji) {
                    info.push_str(&format!("**{}**, ", category_name));
                }
            }
            // if variants (map)
            None => {
                let variants = &category["versions"][default_version]["characters"]
                    .as_object()
                    .unwrap();
                let mut in_variants = false;
                for variant in variants.keys() {
                    let list = variants[variant].as_str().unwrap();
                    if list.contains(kanji) {
                        if !in_variants {
                            info.push_str(&format!("**{}** (", category_name));
                            in_variants = true;
                        }
                        info.push_str(&format!("{}, ", variant));
                    }
                }
                if in_variants {
                    // Remove last two characters (comma and space)
                    info.pop();
                    info.pop();
                    info.push_str("), ");
                }
            }
        };
    }
    // Remove last two characters again
    info.pop();
    info.pop();
    info
}

pub async fn random_kanji(
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

pub fn get_avatar(user: &User) -> String {
    match user.avatar_url() {
        Some(avatar_url) => avatar_url,
        None => user.default_avatar_url(),
    }
}

pub async fn leaderboard(ctx: &Context) -> CommandResult {
    const LENGTH: usize = 5;
    let mut submission_counts: HashMap<String, u32> = HashMap::new();
    for challenge in 1..get_challenge_number() + 1 {
        let submission_data = get_submission_data(challenge);
        for submission in submission_data.iter() {
            let user = submission_counts
                .entry(String::from(
                    submission.as_object().unwrap()["id"].as_str().unwrap(),
                ))
                .or_insert(0);
            *user += 1;
        }
    }
    let mut top_submitters: Vec<(&String, &u32)> = submission_counts.iter().collect();
    top_submitters.sort_by(|a, b| b.1.cmp(a.1));
    let mut leaderboard_html = String::from("<table id=\"leaderboard\">");
    for (i, (id, count)) in top_submitters[0..LENGTH].iter().enumerate() {
        let place = i + 1;
        let user = UserId(id.parse::<u64>().unwrap())
            .to_user(&ctx.http)
            .await?;
        let avatar = get_avatar(&user);
        let profile = format!("https://discord.com/users/{id}");
        let name = user.name;
        let discriminator = user.discriminator;
        leaderboard_html.push_str(&format!("<tr><td>{place}</td><td><img src=\"{avatar}\" alt=\"avatar\"></td><td><a href=\"{profile}\" target=\"_blank\">{name}<span class=\"muted\">#{discriminator}</span></a></td><td>{count}</td></tr>"));
    }
    leaderboard_html.push_str("</table>");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(env::var("LEADERBOARD").unwrap())
        .unwrap();
    file.write_all(leaderboard_html.as_bytes())?;
    file.flush()?;
    Ok(())
}
