use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::env;
use std::fs;

#[command]
async fn challenge(ctx: &Context, msg: &Message) -> CommandResult {
    println!("Command received");
    let challenge_dir = env::var("CHALLENGE_DIR").unwrap();
    let paths = fs::read_dir(challenge_dir).unwrap();
    let challenge = {
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
    };
    msg.reply(
        &ctx.http,
        format!(
            "Tegaki Tuesday #{n}: https://tegakituesday.com/{n}",
            n = challenge
        ),
    )
    .await?;
    Ok(())
}
