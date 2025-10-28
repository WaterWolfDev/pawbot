use crate::{Context, Error};
use poise::CreateReply;
use serenity::all::User;

#[poise::command(
    slash_command,
    description_localized("en-US", "Give your paws to another fur.")
)]
pub async fn give(
    ctx: Context<'_>,
    #[description = "Who do you want to donate paws to?"] who: User,
    #[description = "How many paws?"] count: i32,
) -> Result<(), Error> {
    if count > 10 {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .content("You can only give a maximum of 10 paws!"),
        )
        .await?;
        return Ok(());
    }

    let user_id = ctx.author().id.get() as i64;
    let conn = &ctx.data().db;

    let paws: Option<(i32,)> = sqlx::query_as("SELECT amount FROM paws WHERE user_id = $1;")
        .bind(user_id)
        .fetch_optional(conn)
        .await?;

    let paws: i32 = match paws {
        Some(c) => c.0,
        None => 0,
    };

    if paws == 0 || paws < count {
        ctx.reply("You don't have enough paws to give!".to_string())
            .await?;
        return Ok(());
    }

    let mut tx = conn.begin().await?;
    sqlx::query("UPDATE paws SET amount = amount - $1 WHERE user_id = $2;")
        .bind(count)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("INSERT INTO paws (amount, user_id) VALUES ($1, $2) ON CONFLICT(user_id) DO UPDATE SET amount = paws.amount + $1;")
        .bind(count)
        .bind(who.id.get() as i64)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    ctx.reply(format!(
        "You gave {} paw{} to <@{}>! How nice of you.",
        count,
        if count > 1 { "s" } else { "" },
        who.id
    ))
    .await?;

    Ok(())
}
