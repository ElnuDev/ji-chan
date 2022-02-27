use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::http::typing::Typing;

use std::env;
use std::fs;

use serde_json::Map;
use serde_json::Value;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use slug::slugify;

use crate::commands::owner::get_guild_data;

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

fn get_hugo_path() -> String {
    env::var("HUGO").unwrap()
}

fn get_submission_images_dir() -> String {
    format!("{}/assets/{}", get_hugo_path(), get_challenge_number())
}

fn get_submission_data_path() -> String {
    format!(
        "{}/data/challenges/{}.json",
        get_hugo_path(),
        get_challenge_number()
    )
}

fn get_submission_data() -> Vec<Value> {
    let submission_data_path = get_submission_data_path();
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

fn set_submission_data(submission_data: Vec<Value>) {
    let submission_data: Value = submission_data.into();
    let mut submission_data_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(get_submission_data_path())
        .unwrap();
    submission_data_file
        .write_all(
            serde_json::to_string_pretty(&submission_data)
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
}

fn is_matching_submission(submission: &Value, msg: &Message) -> bool {
    submission["id"].as_str().unwrap() == msg.author.id.as_u64().to_string()
}

fn to_fullwidth(string: &str) -> String {
    let mut fullwidth = String::new();
    for character in string.chars() {
        fullwidth.push(match unicode_hfwidth::to_fullwidth(character) {
            Some(character) => character,
            None => character,
        });
    }
    fullwidth
}

fn rebuild_site() {
    Command::new("./build.sh")
        .current_dir(get_hugo_path())
        .spawn()
        .expect("Failed to rebuild site");
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

#[command]
#[owners_only]
#[allow(non_snake_case)]
async fn pullAndRebuildSite(ctx: &Context, msg: &Message) -> CommandResult {
    Command::new("git")
        .current_dir(get_hugo_path())
        .arg("pull")
        .spawn()
        .expect("Failed to git pull")
        .wait()?;
    rebuild_site();
    msg.reply(&ctx.http, "Pulled and started site rebuild process!")
        .await?;
    Ok(())
}

#[command]
async fn challenge(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(
        &ctx.http,
        format!(
            "Tegaki Tuesday #{n}: <https://tegakituesday.com/{n}>",
            n = get_challenge_number()
        ),
    )
    .await?;
    Ok(())
}

#[command]
async fn submit(ctx: &Context, msg: &Message) -> CommandResult {
    // TODO: The code for this command needs to be refactored,
    // there are large duplicated sections that need to be merged somehow.
    let guild_data = get_guild_data();
    let guild = msg.guild_id.unwrap().as_u64().to_string();
    if !guild_data.contains_key(&guild)
        || !&guild_data[&guild]
            .as_object()
            .unwrap()
            .contains_key("submissionChannel")
    {
        msg.reply(&ctx.http, "Submissions aren't enabled for this server yet.")
            .await?;
        return Ok(());
    }
    let current_guild_data = &guild_data[&guild].as_object().unwrap();
    let submission_channel = current_guild_data["submissionChannel"].as_str().unwrap();
    if submission_channel != &msg.channel_id.as_u64().to_string() {
        msg.reply(
            &ctx.http,
            format!(
                "Sorry, submissions aren't permitted here. Please go to <#{}>. Thanks!",
                guild
            ),
        )
        .await?;
        return Ok(());
    }
    if msg.attachments.len() == 0 {
        msg.reply(&ctx.http, "Please attach at least one image.")
            .await?;
        return Ok(());
    }
    let typing = Typing::start(ctx.http.clone(), *msg.channel_id.as_u64()).unwrap();
    let challenge_number = get_challenge_number();
    let submission_images_dir = get_submission_images_dir();

    // Ensure that submission_images_dir exists
    let path = Path::new(&submission_images_dir);
    std::fs::create_dir_all(path)?;

    let mut submission_data = get_submission_data();
    let mut existing_submitter = false;
    let mut invalid_types = false;
    let mut requires_rebuild = false;
    for (i, submission) in submission_data.iter_mut().enumerate() {
        if is_matching_submission(&submission, &msg) {
            existing_submitter = true;
            let mut images = submission["images"].as_array_mut().unwrap().clone();
            for attachment in msg.attachments.iter() {
                let extension;
                if let Some(content_type) = &attachment.content_type {
                    if content_type == "image/png" {
                        extension = "png";
                    } else if content_type == "image/jpeg" {
                        extension = "jpg";
                    } else {
                        invalid_types = true;
                        continue;
                    }
                } else {
                    invalid_types = true;
                    continue;
                }
                requires_rebuild = true;
                let file_name = format!(
                    "{}-{}-{}-{}.{}",
                    i + 1,
                    slugify(&msg.author.name),
                    msg.author.discriminator,
                    images.len() + 1,
                    extension
                );
                images.push(file_name.clone().into());
                let image = reqwest::get(&attachment.url).await?.bytes().await?;
                let mut image_file =
                    File::create(format!("{}/{}", submission_images_dir, file_name))?;
                image_file.write_all(&image)?;
                image_file.flush()?;
            }
            submission["images"] = images.into();
            break;
        }
    }
    if !existing_submitter {
        let mut submitter_data = Map::new();
        submitter_data.insert(
            String::from("username"),
            format!("{}#{}", msg.author.name, msg.author.discriminator).into(),
        );
        let mut images: Vec<String> = Vec::new();
        for attachment in msg.attachments.iter() {
            let extension;
            if let Some(content_type) = &attachment.content_type {
                if content_type == "image/png" {
                    extension = "png";
                } else if content_type == "image/jpeg" {
                    extension = "jpg";
                } else {
                    invalid_types = true;
                    continue;
                }
            } else {
                invalid_types = true;
                continue;
            }
            requires_rebuild = true;
            let file_name = format!(
                "{}-{}-{}{}.{}",
                submission_data.len() + 1,
                slugify(&msg.author.name),
                msg.author.discriminator,
                if images.len() == 0 {
                    String::from("")
                } else {
                    format!("-{}", images.len() + 1)
                },
                extension
            );
            images.push(file_name.clone().into());
            let image = reqwest::get(&attachment.url).await?.bytes().await?;
            let mut image_file = File::create(format!("{}/{}", submission_images_dir, file_name))?;
            image_file.write_all(&image)?;
            image_file.flush()?;
        }
        submitter_data.insert(String::from("images"), images.into());
        submitter_data.insert(
            String::from("id"),
            msg.author.id.as_u64().to_string().into(),
        );
        submission_data.push(submitter_data.into());
    }
    set_submission_data(submission_data);
    let mut message = String::new();
    if requires_rebuild {
        message.push_str(&format!("Thank you for submitting! You can view your submission at <https://tegakituesday.com/{}>", challenge_number));
        if invalid_types {
            message.push_str("\nSome of your attachments could not be uploaded; only **.png**, **.jpg**, and **.jpeg** files are permitted.");
        }
        rebuild_site();
    } else if invalid_types {
        message.push_str("Sorry, your submission could not be uploaded; only **.png**, **.jpg**, and **.jpeg** files are permitted.");
    }
    typing.stop();
    msg.reply(&ctx.http, message).await?;
    Ok(())
}

#[command]
async fn images(ctx: &Context, msg: &Message) -> CommandResult {
    let submission_data = get_submission_data();
    let images: Vec<String> = {
        let mut images = Vec::new();
        for submission in submission_data.iter() {
            if is_matching_submission(&submission, &msg) {
                for image in submission["images"].as_array().unwrap().iter() {
                    images.push(String::from(image.as_str().unwrap()));
                }
                break;
            }
        }
        images
    };
    let challenge_number = get_challenge_number();
    if images.len() == 0 {
        msg.reply(
            &ctx.http,
            format!(
                "You haven't submitted anything for Tegaki Tuesday #{}.",
                challenge_number
            ),
        )
        .await?;
        return Ok(());
    }
    let mut message = String::from(format!(
        "Your submission images for Tegaki Tuesday #{}:\n",
        challenge_number
    ));
    for (i, image) in images.iter().enumerate() {
        message.push_str(&format!(
            "（{}）<https://tegakituesday.com/{}/{}>\n",
            to_fullwidth(&(i + 1).to_string()),
            challenge_number,
            image
        ));
    }
    msg.reply(&ctx.http, message).await?;
    Ok(())
}

#[command]
// imageDelete instead of image_delete to keep command naming from Python version
#[allow(non_snake_case)]
async fn imageDelete(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let number;
    match args.single::<i32>() {
        Ok(value) => number = value,
        Err(_) => {
            msg.reply(&ctx.http, format!("Please provide the image number you want to delete. You can get a list of your submitted images using `{}images`.", env::var("PREFIX").unwrap())).await?;
            return Ok(());
        }
    }
    if number < 1 {
        msg.reply(
            &ctx.http,
            "That isn't a valid image number. Image numbers start at 1.",
        )
        .await?;
        return Ok(());
    }
    let challenge_number = get_challenge_number();
    let mut submission_data = get_submission_data();
    for (i, submission) in submission_data.iter_mut().enumerate() {
        if !is_matching_submission(&submission, &msg) {
            continue;
        }
        let mut images = submission["images"].as_array().unwrap().clone();
        let image_count = images.len();
        if image_count < number.try_into().unwrap() {
            msg.reply(
                &ctx.http,
                if image_count == 0 {
                    // This is an edge case that should never happen.
                    // In this scenario, there is submission data with an empty image list.
                    // Submission data should be deleted uppon imageDelete if there are no images remaining.
                    format!(
                        "You haven't submitted anything for Tegaki Tuesday #{}.",
                        challenge_number
                    )
                } else {
                    format!(
                        "That image number doesn't exist, you only have {} image{}.",
                        image_count,
                        if image_count == 1 { "" } else { "s" }
                    )
                },
            )
            .await?;
            return Ok(());
        }
        let index = number as usize - 1;
        let image = images[index].as_str().unwrap().to_owned();
        let submission_images_dir = get_submission_images_dir();
        let image_path = format!("{}/{}", submission_images_dir, image);
        match fs::remove_file(image_path) {
            Ok(_) => (),
            // No need to worry about if the file is already missing
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => panic!("Failed to remove file"),
        };
        let mut message = String::from(format!("Deleted **{}** from your submission.", image));
        if images.len() == 1 {
            message.push_str(" As there are no more images left attached to your submission, it has been deleted.");
            submission_data.remove(i);
        } else {
            images.remove(index);
            // One must rename all of the submitted images to prevent overwrites.
            // Consider the following scenario:
            // The submissions are ["bob-1234.png", "bob-1234-2.png"]
            // Bob uses the command imageDelete 1
            // The submissions become ["bob-1234-2.png"]
            // Bob makes a second submission
            // The submissions become ["bob-1234-2.png", "bob-1234-2.png"]
            // The original bob-1234-2.png gets overwriten,
            // and both submissions end up pointing to the same image.
            // In order to prevent this, images need to be renamed according to their index.
            for (j, image) in images.iter_mut().enumerate() {
                let old = image.as_str().unwrap();
                let from = format!("{}/{}", submission_images_dir, old);
                let new = format!(
                    "{}-{}-{}{}.{}",
                    i + 1,
                    slugify(&msg.author.name),
                    msg.author.discriminator,
                    if j == 0 {
                        String::from("")
                    } else {
                        format!("-{}", j + 1)
                    },
                    Path::new(old).extension().unwrap().to_str().unwrap()
                );
                let to = format!("{}/{}", submission_images_dir, new);
                fs_extra::file::move_file(from, to, &fs_extra::file::CopyOptions::default())
                    .unwrap();
                *image = new.into();
            }
            submission["images"] = images.into();
        }
        set_submission_data(submission_data);
        rebuild_site();
        msg.reply(&ctx.http, message).await?;
        return Ok(());
    }
    msg.reply(
        &ctx.http,
        format!(
            "You haven't submitted anything for Tegaki Tuesday #{}.",
            challenge_number
        ),
    )
    .await?;
    Ok(())
}
