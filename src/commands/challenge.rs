use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::env;
use std::fs;

use serde_json::Map;
use serde_json::Value;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
// Required for rebuilding site
// use std::process::Command;

use slugify::slugify;

fn get_challenge_number() -> i32 {
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

#[command]
async fn challenge(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(
        &ctx.http,
        format!(
            "Tegaki Tuesday #{n}: https://tegakituesday.com/{n}",
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
    if msg.attachments.len() == 0 {
        msg.reply(&ctx.http, "Please attach at least one image.")
            .await?;
        return Ok(());
    }
    let hugo_path = env::var("HUGO").unwrap();
    let challenge_number = get_challenge_number();
    let submission_images_dir = format!("{}/assets/{}", hugo_path, challenge_number);
    let submission_data_path = format!("{}/data/challenges/{}.json", hugo_path, challenge_number);
    let submission_data_json = match File::open(&submission_data_path) {
        Ok(mut file) => {
            let mut json = String::new();
            file.read_to_string(&mut json)?;
            json
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut file = File::create(&submission_data_path)?;
            file.write_all(b"[]")?;
            file.flush()?;
            String::from("[]")
        }
        Err(_) => panic!("Failed to open submission data file"),
    };
    let mut submission_data: Value = serde_json::from_str(&submission_data_json).unwrap();
    let mut submission_data = submission_data.as_array_mut().unwrap().clone();
    let mut existing_submitter = false;
    let mut invalid_types = false;
    let mut requires_rebuild = false;
    for (i, submission) in submission_data.iter_mut().enumerate() {
        if &submission["id"].as_str().unwrap() == &msg.author.id.as_u64().to_string() {
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
                    slugify!(&msg.author.name),
                    msg.author.discriminator,
                    images.len() + 1,
                    extension
                );
                images.push(file_name.clone().into());
                let image = reqwest::get(&attachment.url).await?.bytes().await?;
                let mut image_file =
                    File::create(format!("{}/{}", submission_images_dir, file_name))?;
                image_file.write_all(&image)?;
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
                slugify!(&msg.author.name),
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
        }
        submitter_data.insert(String::from("images"), images.into());
        submitter_data.insert(
            String::from("id"),
            msg.author.id.as_u64().to_string().into(),
        );
        submission_data.push(submitter_data.into());
    }
    let submission_data: Value = submission_data.into();
    let mut submission_data_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&submission_data_path)?;
    submission_data_file
        .write_all(serde_json::to_string_pretty(&submission_data)?.as_bytes())
        .unwrap();
    let mut message = String::new();
    if requires_rebuild {
        message.push_str(&format!("Thank you for submitting! You can view your submission at <https://tegakituesday.com/{}>", challenge_number));
        if invalid_types {
            message.push_str("\nSome of your attachments could not be uploaded; only **.png**, **.jpg**, and **.jpeg** files are permitted.");
        }
    // Currently untested, and thus commented
    // Command::new("hugo").current_dir(&hugo_path).spawn().expect("Failed to rebuild site");
    } else if invalid_types {
        message.push_str("Sorry, your submission could not be uploaded; only **.png**, **.jpg**, and **.jpeg** files are permitted.");
    }
    msg.reply(&ctx.http, &message).await?;
    Ok(())
}
