use crate::{Context, Error};
use serenity::all::User;

#[poise::command(slash_command)]
pub async fn give(ctx: Context<'_>, receiver: User, count: i32) -> Result<(), Error> {
    if count > 10 {
        ctx.reply("You can only give away a maximum of 10 paws!".to_string())
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
        .bind(receiver.id.get() as i64)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    ctx.reply(format!(
        "You have {} paws to {}! How nice of you.",
        count, receiver.name
    ))
    .await?;

    Ok(())
}
