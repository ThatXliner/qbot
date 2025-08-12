use llm::LLMProvider;
use llm::builder::{LLMBackend, LLMBuilder};
use poise::{CreateReply, send_reply, serenity_prelude as serenity};
use tracing::debug;

use crate::qb::{Tossup, random_tossup};
use crate::query::{ApiQuery, CATEGORIES, QueryError, parse_query};
use crate::read::{event_handler, read_question};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serenity::all::{ChannelId, UserId};
use tokio::sync::{Mutex, watch};

// #[cfg(test)]
// mod buzzing_test;
mod check;
mod qb;
mod query;
#[cfg(test)]
mod query_tests;
mod read;
mod utils;

// https://mermaid.live/edit#pako:eNplkMtugzAQRX_FmmUFCNuYOF5UaummGxZdtu7CAocgBTsypg8Q_14eKY2aWc09d-7YmgEKW2oQ0Hrl9VOtKqea8INIg6ZaIJKQW_Rg2k_tJCDVonx13-7eURjeoxetytpUK7yIxXjs-n6lc7egSzS_DW4jmXVOF_4ffTbFNd_k7aLsypi-CAFUri5BeNfpABrtGjVLGOZxCf6oGy1BTG2pD6o7eQnSjFPsrMyrtc1v0tmuOoI4qFM7qe5c_h1so06bUrvMdsaDIMmeL1tADPAFAic0wozSNI055-mOBPANIqURxyThnDG2jzkZA-iXV-OI71gcx5ikmFNGcTL-ABL-f_0
#[derive(Debug, Clone)]
pub enum QuestionState {
    Reading,
    // Buzzed (user_id, timestamp)
    Buzzed(UserId, i64),
    // Prompt (user_id, prompt, timestamp)
    Prompt(UserId, String, i64),
    Invalid(UserId),
    Incorrect(UserId),
    Correct,
    Judging,
    // OPTIMIZE: Idle state rather than deleting it from the map?
    // I'll need to figure out which is more performant
}

/// User data, which is stored and accessible in all command invocations
pub struct Data {
    pub reqwest: reqwest::Client,
    // (channel_id, (question_state, power?, blocklist, state_change_notifier))
    pub reading_states: Arc<
        Mutex<
            HashMap<
                ChannelId,
                (
                    QuestionState,
                    // shoot, i need to remove this... but it's gonna be a pain to change...
                    bool,
                    HashSet<UserId>,
                    watch::Sender<()>,
                    Tossup,
                    String,
                ),
            >,
        >,
    >,
    pub llm: Box<dyn LLMProvider>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command)]
async fn tossup(
    ctx: Context<'_>,
    #[description = "Query for selecting the category"] query: Option<String>,
) -> Result<(), Error> {
    if ctx
        .data()
        .reading_states
        .lock()
        .await
        .contains_key(&ctx.channel_id())
    {
        send_reply(
            ctx,
            CreateReply::default()
                .ephemeral(true)
                .content("Already reading a question"),
        )
        .await?;
        return Ok(());
    }
    let tossup = if let Some(query) = query {
        let parsed_results = parse_query(&query);
        debug!("Query requested: {:?}", query);
        debug!("Parsed query results: {:?}", parsed_results);
        match parsed_results {
            Ok(api_params) => {
                let reqwest = &ctx.data().reqwest;
                let get_tossup = random_tossup(reqwest, &api_params).await?;
                get_tossup.tossups
            }
            Err(err) => {
                match err {
                    QueryError::UnexpectedToken(message) => {
                        ctx.say(message).await?;
                    }
                    QueryError::InvalidCategory(category) => {
                        ctx.say(format!("Invalid category: {}", category)).await?;
                    }
                    QueryError::ImpossibleBranch(issue) => {
                        ctx.say(format!(
                            "The query is impossible (conflicting categories): {}",
                            issue
                        ))
                        .await?;
                    }
                    QueryError::UnexpectedEOF => {
                        ctx.say("Unexpected end of input").await?;
                    }
                };
                return Ok(());
            }
        }
    } else {
        let reqwest = &ctx.data().reqwest;
        let get_tossup = random_tossup(reqwest, &ApiQuery::default()).await?;
        get_tossup.tossups
    };
    read_question(&ctx, tossup).await?;
    Ok(())
}
/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn categories(
    ctx: Context<'_>,
    #[description = "A specific category to see subcategories for"] parent_category: Option<String>,
) -> Result<(), Error> {
    if let Some(category) = parent_category {
        ctx.say(format!("```\n{:?}\n```\n", CATEGORIES.get(&category)))
            .await?;
    } else {
        ctx.say(format!("```\n{:?}\n```\n", CATEGORIES)).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let ollama_base_url = std::env::var("OLLAMA_URL").unwrap_or("http://127.0.0.1:11434".into());
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![tossup(), categories()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    reqwest: reqwest::Client::new(),
                    reading_states: Arc::new(Mutex::new(HashMap::new())),
                    llm: LLMBuilder::new()
                        .backend(LLMBackend::Ollama) // Use Ollama as the LLM backend
                        .base_url(ollama_base_url) // Set the Ollama server URL
                        .model("qwen3:0.6b")
                        .max_tokens(1000) // Set maximum response length
                        .temperature(0.7) // Control response randomness (0.0-1.0)
                        .stream(false) // Disable streaming responses
                        .build()
                        .expect("Failed to build LLM (Ollama)"),
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
