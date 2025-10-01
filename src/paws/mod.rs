use crate::{Context, Error};

pub mod balance;
pub mod daily;
pub mod gamble;
pub mod give;
pub mod steal;
pub mod top;
pub mod event_handler;

#[poise::command(
    slash_command,
    subcommands(
        "daily::daily",
        "gamble::gamble",
        "balance::balance",
        "give::give",
        "steal::steal",
        "top::top",
    )
)]
pub async fn paw(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
