use crate::{Context, Error};
use poise::serenity_prelude::CreateEmbed;

// A struct to hold the data from our leaderboard query.
#[derive(sqlx::FromRow)]
struct LeaderboardEntry {
    user_id: i64,
    amount: i32,
}

#[poise::command(slash_command, description_localized("en-US", "View the pawdium."))]
pub async fn top(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get() as i64;
    let conn = &ctx.data().db;

    let total_paws: (i64,) = sqlx::query_as("SELECT SUM(amount) FROM paws")
        .fetch_one(conn)
        .await?;

    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM paws")
        .fetch_one(conn)
        .await?;

    let top_users: Vec<LeaderboardEntry> =
        sqlx::query_as("SELECT user_id, amount FROM paws ORDER BY amount DESC LIMIT 10")
            .fetch_all(conn)
            .await?;

    let mut user_in_top_10 = false;
    let mut description_string = format!("🐶 {}\n🧑‍🌾 {}\n", total_paws.0, total_users.0);

    for (i, entry) in top_users.iter().enumerate() {
        let rank = i + 1;
        let ranked_user_id = entry.user_id;

        let line = format!(
            "{}. <@{}> - **{}** paws\n",
            rank, ranked_user_id, entry.amount
        );
        description_string.push_str(&line);

        if entry.user_id == user_id {
            user_in_top_10 = true;
        }
    }

    if !user_in_top_10 {
        let user_rank: Option<(i64, i32)> = sqlx::query_as(
            "WITH ranked_users AS (
                SELECT user_id, amount, ROW_NUMBER() OVER (ORDER BY amount DESC) as rank
                FROM paws
            )
            SELECT rank, amount FROM ranked_users WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(conn)
        .await?;

        if let Some((rank, amount)) = user_rank {
            let other_furs = format!("... _{} other furries_ ...\n", rank - 11);
            description_string.push_str(&other_furs);
            let desc = format!("{}. <@{}> - **{}** paws", rank, ctx.author().id, amount);
            description_string.push_str(&desc);
        } else {
            description_string.push_str("...");
            description_string.push_str("You have **0** paws and are not yet ranked.");
        }
    }

    let embed = CreateEmbed::new()
        .title("🐶 Paw Leaderboard 👑")
        .description(description_string)
        .color(0x00BFFF);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
