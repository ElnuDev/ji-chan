use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use std::env;

#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    let prefix = env::var("PREFIX").unwrap();
    let message = format!(
"<:jichan:943336845637480478> Hello! I'm 字【じ】ちゃん (Ji-chan), the Tegaki Tuesday bot (and mascot!).
For more information about the challenge, check out the website at <https://tegakituesday.com>

__**Tegaki Tuesday 手書きの火曜日**__
:ballot_box: `{p}submit` Submit to the latest handwriting challenge.
:pencil: `{p}challenge` View the latest handwriting challenge info.
:trophy: `{p}leaderboard` ~~View the community leaderboard and challenge stats!~~ **(Coming back soon)**
:frame_photo: `{p}images` List images in your current submission, if available.
:wastebasket: `{p}imageDelete <image number>` Delete images from your current submission using image numbers from -h images.

__**Kanji 漢字**__
:paintbrush: `{p}so <text>` Get stroke order diagrams for character(s), maximum 4
:game_die: `{p}jinmeiyo` Random Jinmeiyō kanji
:game_die: `{p}joyo` Random Jōyō kanji
:game_die: `{p}kyoiku <grade|all>` Random Kyōiku kanji
:game_die: `{p}jlpt <level|all>` Random JLPT kanji
:game_die: `{p}hyogai <group|all>` Random Hyōgai kanji",
        p = prefix
    );
    msg.reply(&ctx.http, message).await?;
    Ok(())
}
