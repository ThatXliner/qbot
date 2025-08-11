use std::collections::HashSet;

use dashmap::DashMap;
use serenity::all::{ChannelId, UserId};
use tokio::sync::Notify;

pub mod qb;
pub mod query;
pub mod read;
// https://mermaid.live/edit#pako:eNplkMtugzAQRX_FmmUFCNuYOF5UaummGxZdtu7CAocgBTsypg8Q_14eKY2aWc09d-7YmgEKW2oQ0Hrl9VOtKqea8INIg6ZaIJKQW_Rg2k_tJCDVonx13-7eURjeoxetytpUK7yIxXjs-n6lc7egSzS_DW4jmXVOF_4ffTbFNd_k7aLsypi-CAFUri5BeNfpABrtGjVLGOZxCf6oGy1BTG2pD6o7eQnSjFPsrMyrtc1v0tmuOoI4qFM7qe5c_h1so06bUrvMdsaDIMmeL1tADPAFAic0wozSNI055-mOBPANIqURxyThnDG2jzkZA-iXV-OI71gcx5ikmFNGcTL-ABL-f_0
pub enum QuestionState {
    Reading,
    Buzzed(UserId, Notify),
    Invalid(UserId),
    Correct,
    // OPTIMIZE: Idle state rather than deleting it from the map?
    // I'll need to figure out which is more performant
}
pub struct Data {
    pub reqwest: reqwest::Client,
    // (channel_id, (question_state, power?, blocklist))
    pub reading_states: DashMap<ChannelId, (QuestionState, bool, HashSet<UserId>)>,
} // User data, which is stored and accessible in all command invocations
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
