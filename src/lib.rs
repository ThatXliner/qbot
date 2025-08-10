pub mod qb;
pub mod query;
pub mod read;

pub struct Data {
    pub reqwest: reqwest::Client,
    // TODO: locks for tossups
} // User data, which is stored and accessible in all command invocations
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
