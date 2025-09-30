use crate::{Context, Error};

#[poise::command(slash_command)]
pub async fn balance(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get() as i64;
    let conn = &ctx.data().db;

    let paws: Option<(i32,)> = sqlx::query_as("SELECT amount FROM paws WHERE user_id = $1;")
        .bind(user_id)
        .fetch_optional(conn)
        .await?;

    let paws: i32 = match paws {
        Some(c) => c.0,
        None => 0
    };

    ctx.reply(format!("You have {} paws!", paws)).await?;

    Ok(())
}