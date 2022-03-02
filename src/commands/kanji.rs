use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use crate::utils::*;

#[command]
async fn i(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let input = args.rest();
    let chars = input.chars();
    let mut message = String::from("");
    let mut covered_chars: Vec<char> = Vec::new();
    let mut skipped_chars = 0;
    let mut found_chars: Vec<char> = Vec::new();
    for character in chars {
        if covered_chars.contains(&character) {
            continue;
        }
        let kanji_info = get_kanji_info(character);
        covered_chars.push(character);
        if kanji_info.len() == 0 {
            skipped_chars += 1;
            continue;
        }
        found_chars.push(character);
        message.push_str(&format!(
            "[{}](https://jisho.org/search/{}%23kanji): {}\n",
            character, character, kanji_info
        ));
    }
    message = format!(
        "Found {} kanji{}\n{}",
        found_chars.len(),
        if found_chars.len() == 0 { "." } else { ":" },
        message
    );
    if skipped_chars > 0 {
        message.push_str(&format!(
            "Skipped {} character{}.",
            skipped_chars,
            if skipped_chars == 1 { "" } else { "s" }
        ));
    }
    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.reference_message(msg);
            m.allowed_mentions(|am| {
                am.empty_parse();
                am
            });
            m.embed(|e| {
                e.title(input);
                e.description(message);
                let mut author = serenity::builder::CreateEmbedAuthor::default();
                author
                    .icon_url("https://raw.githubusercontent.com/ElnuDev/ji-chan/main/ji-chan.png")
                    .name("字ちゃん")
                    .url("https://github.com/ElnuDev/ji-chan");
                e.set_author(author);
                e.color(serenity::utils::Colour::from_rgb(251, 73, 52));
                if found_chars.len() == 1 {
                    e.thumbnail(get_so_diagram(found_chars[0]));
                }
                e
            });
            m
        })
        .await
        .unwrap();
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
