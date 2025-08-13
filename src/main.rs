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
#[cfg(test)]
mod judge_tests;
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
    #[description = "Number of questions to read (1-10)"]
    #[min = 1]
    #[max = 10]
    number: Option<u32>,
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

    let number_of_questions = number.unwrap_or(1);

    let tossups = if let Some(query) = query {
        let mut parsed_results = parse_query(&query);
        debug!("Query requested: {:?}", query);
        debug!("Parsed query results: {:?}", parsed_results);

        match parsed_results {
            Ok(ref mut api_params) => {
                // Set the number of questions to fetch
                api_params.number = number_of_questions;
                let reqwest = &ctx.data().reqwest;
                let get_tossup = random_tossup(reqwest, api_params).await?;
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
        let mut api_query = ApiQuery::default();
        api_query.number = number_of_questions;
        let get_tossup = random_tossup(reqwest, &api_query).await?;
        get_tossup.tossups
    };

    if tossups.is_empty() {
        ctx.say("No tossups found").await?;
        return Ok(());
    }

    // Read questions one by one
    for (index, question) in tossups.iter().enumerate() {
        if index > 0 {
            // Wait for previous question to finish
            while ctx
                .data()
                .reading_states
                .lock()
                .await
                .contains_key(&ctx.channel_id())
            {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }

            // Announce next question
            ctx.channel_id()
                .say(&ctx.http(), "🔄 **Next question**")
                .await?;
            // Small delay before starting next question
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        read_question(&ctx, vec![question.clone()], index == 0).await?;

        // If this is not the last question, wait for it to complete
        if index < tossups.len() - 1 {
            // Wait for the question reading to complete
            while ctx
                .data()
                .reading_states
                .lock()
                .await
                .contains_key(&ctx.channel_id())
            {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }

    Ok(())
}

/// Displays the quiz bowl categories you can choose from
#[poise::command(slash_command, prefix_command)]
async fn categories(
    ctx: Context<'_>,
    #[description = "A specific category to see subcategories for"] parent_category: Option<String>,
) -> Result<(), Error> {
    if let Some(category) = parent_category {
        // Search for the category case-insensitively
        let category_key = CATEGORIES
            .keys()
            .find(|&key| key.to_lowercase() == category.to_lowercase());

        if let Some(key) = category_key {
            let (subcategories, alternate_subcategories) = CATEGORIES.get(key).unwrap();

            let mut response = format!("**{}**\n", key);

            if !subcategories.is_empty() {
                response.push_str("**Subcategories:**\n");
                for subcat in subcategories.iter() {
                    response.push_str(&format!("• {}\n", subcat));
                }
                response.push('\n');
            }

            if !alternate_subcategories.is_empty() {
                response.push_str("**Alternate Subcategories:**\n");
                for alt_subcat in alternate_subcategories.iter() {
                    response.push_str(&format!("• {}\n", alt_subcat));
                }
            }

            if subcategories.is_empty() && alternate_subcategories.is_empty() {
                response.push_str("*No subcategories available for this category.*");
            }

            ctx.say(response).await?;
        } else {
            ctx.say(format!(
                "❌ Category '{}' not found. Use `/categories` to see all available categories.",
                category
            ))
            .await?;
        }
    } else {
        let mut response = String::from("**Available Quiz Bowl Categories:**\n\n");

        let mut sorted_categories: Vec<_> = CATEGORIES.keys().collect();
        sorted_categories.sort();

        for category in sorted_categories {
            let (subcategories, alternate_subcategories) = CATEGORIES.get(category).unwrap();
            let total_subcats = subcategories.len() + alternate_subcategories.len();

            response.push_str(&format!("**{}**", category));
            if total_subcats > 0 {
                response.push_str(&format!(" ({} subcategories)", total_subcats));
            }
            response.push('\n');
        }

        response.push_str(
            "\n💡 Use `/categories <category_name>` to see subcategories for a specific category.",
        );

        ctx.say(response).await?;
    }
    Ok(())
}

/// Displays help information about the bot and its commands
#[poise::command(slash_command, prefix_command)]
async fn help(
    ctx: Context<'_>,
    #[description = "Specific topic to get help about"] topic: Option<String>,
) -> Result<(), Error> {
    if let Some(topic) = topic {
        match topic.to_lowercase().as_str() {
            "query" | "query-language" | "queries" => {
                show_query_language_help(ctx).await?;
            }
            "commands" => {
                show_commands_help(ctx).await?;
            }
            "tossup" => {
                let help_text = "**📚 /tossup Command**\n\n\
                    **Usage:** `/tossup [query] [number]`\n\n\
                    **Parameters:**\n\
                    • `query` (optional): Filter questions using the query language\n\
                    • `number` (optional): Number of questions to read (1-10, default: 1)\n\n\
                    **Examples:**\n\
                    • `/tossup` - Random question from any category\n\
                    • `/tossup query:Biology` - Random biology question\n\
                    • `/tossup query:Science + History number:3` - 3 questions from Science or History\n\
                    • `/tossup number:5` - 5 random questions\n\n\
                    When reading multiple questions, the bot will say \"Next question\" between each one.";
                ctx.say(help_text).await?;
            }
            "categories" => {
                let help_text = "**📂 /categories Command**\n\n\
                    **Usage:** `/categories [parent_category]`\n\n\
                    **Parameters:**\n\
                    • `parent_category` (optional): Specific category to view subcategories for\n\n\
                    **Examples:**\n\
                    • `/categories` - Show all available categories\n\
                    • `/categories Science` - Show Science subcategories\n\
                    • `/categories Literature` - Show Literature subcategories";
                ctx.say(help_text).await?;
            }
            "query-test" | "query-command" => {
                let help_text = "**🧪 /query Command**\n\n\
                    **Usage:** `/query <query_string>`\n\n\
                    **Purpose:** Test your query language expressions without fetching actual questions.\n\n\
                    **Examples:**\n\
                    • `/query Biology + Chemistry` - Test combining categories\n\
                    • `/query Science - Math` - Test excluding categories\n\
                    • `/query (Biology + Chemistry) & Science` - Test complex expressions\n\n\
                    The command will show you what categories and subcategories your query would match.";
                ctx.say(help_text).await?;
            }
            _ => {
                ctx.say(format!("❌ Unknown help topic: '{}'. Available topics: `query`, `commands`, `tossup`, `categories`, `query-test`", topic)).await?;
            }
        }
    } else {
        show_general_help(ctx).await?;
    }
    Ok(())
}

async fn show_general_help(ctx: Context<'_>) -> Result<(), Error> {
    let help_text = "# 🎯 Quiz Bowl Bot\n\n\
        A Discord bot for quiz bowl question practice with advanced query language support.\n\n\
        ## 📋 Available Commands\n\n\
        • **`/tossup`** - Get quiz bowl questions (supports filtering and multiple questions)\n\
        • **`/categories`** - View available question categories and subcategories\n\
        • **`/query`** - Test query language expressions\n\
        • **`/help`** - Get help (you're here!)\n\n\
        ## 🔍 Quick Start\n\n\
        1. Use `/tossup` to get a random question\n\
        2. Use `/categories` to see what categories are available\n\
        3. Use `/tossup query:Biology` to get biology questions\n\
        4. Use `/help query` to learn about the powerful query language\n\n\
        ## 📖 Get More Help\n\n\
        • `/help query` - Learn the query language syntax\n\
        • `/help commands` - Detailed command reference\n\
        • `/help tossup` - Learn about the tossup command options\n\
        • `/help categories` - Learn about browsing categories";

    ctx.say(help_text).await?;
    Ok(())
}

async fn show_commands_help(ctx: Context<'_>) -> Result<(), Error> {
    let help_text = "## 📋 Detailed Command Reference\n\n\
        **`/tossup [query] [number]`**\n\
        Get quiz bowl questions with optional filtering and quantity.\n\
        • `query`: Use query language to filter by categories\n\
        • `number`: Number of questions (1-10)\n\n\
        **`/categories [parent_category]`**\n\
        Browse available question categories and subcategories.\n\
        • Without parameters: Shows all main categories\n\
        • With category name: Shows subcategories for that category\n\n\
        **`/query <query_string>`**\n\
        Test query language expressions to see what they would match.\n\
        • Shows which categories/subcategories would be included\n\
        • Helpful for building complex queries\n\n\
        **`/help [topic]`**\n\
        Get help about the bot or specific topics.\n\
        • Without parameters: General overview\n\
        • With topic: Detailed help for that topic";

    ctx.say(help_text).await?;
    Ok(())
}

async fn show_query_language_help(ctx: Context<'_>) -> Result<(), Error> {
    let help_text = "## 🔍 Query Language Guide\n\n\
        Filter quiz bowl questions using Boolean expressions with categories.\n\n\
        ### Basic Syntax\n\
        • **Categories**: `Science`, `History`, `Literature`, etc.\n\
        • **Subcategories**: `Biology`, `Chemistry`, `American History`, etc.\n\
        • **Multi-word**: Use quotes or just spaces: `American Literature`\n\n\
        ### Operators (by precedence)\n\
        1. **`-` (Minus/Exclusion)** - Remove categories: `Science - Math`\n\
        2. **`&` (And/Intersection)** - Must match both: `Science & Biology`\n\
        3. **`+` (Or/Union)** - Match either: `Science + History`\n\
        4. **`()` (Parentheses)** - Override precedence: `(Science + History) - Math`\n\n\
        ### Examples\n\
        • `Biology` - All biology questions\n\
        • `Science + History` - Science OR history questions\n\
        • `Biology & Chemistry` - Questions tagged as both\n\
        • `Science - Math` - Science questions excluding math\n\
        • `(Biology + Chemistry) - Math` - Biology or chemistry, but no math\n\n\
        💡 Use `/query <expression>` to test your queries!\n\
        📂 Use `/categories` to see available categories!";

    ctx.say(help_text).await?;
    Ok(())
}

/// Test query language expressions to see what they would match
#[poise::command(slash_command, prefix_command)]
async fn query(
    ctx: Context<'_>,
    #[description = "Query expression to test"] query_string: String,
) -> Result<(), Error> {
    debug!("Testing query: {}", query_string);

    match parse_query(&query_string) {
        Ok(api_params) => {
            let mut response = format!(
                "✅ **Query parsed successfully!**\n\n**Input:** `{}`\n\n",
                query_string
            );

            if !api_params.categories.is_empty() {
                response.push_str("**Main Categories:**\n");
                for cat in &api_params.categories {
                    response.push_str(&format!("• {}\n", cat));
                }
                response.push('\n');
            }

            if !api_params.subcategories.is_empty() {
                response.push_str("**Subcategories:**\n");
                for subcat in &api_params.subcategories {
                    response.push_str(&format!("• {}\n", subcat));
                }
                response.push('\n');
            }

            if !api_params.alternate_subcategories.is_empty() {
                response.push_str("**Alternate Subcategories:**\n");
                for alt_subcat in &api_params.alternate_subcategories {
                    response.push_str(&format!("• {}\n", alt_subcat));
                }
                response.push('\n');
            }

            if api_params.categories.is_empty()
                && api_params.subcategories.is_empty()
                && api_params.alternate_subcategories.is_empty()
            {
                response.push_str("*No specific categories matched - would return questions from all categories.*\n\n");
            }

            response.push_str(&format!(
                "**Number of questions:** {}\n\n",
                api_params.number
            ));
            response.push_str(&format!(
                "💡 Use `/tossup query:{}` to get actual questions with this filter!",
                query_string
            ));

            ctx.say(response).await?;
        }
        Err(err) => {
            let error_msg = match err {
                QueryError::UnexpectedToken(message) => {
                    format!("❌ **Syntax Error**\n\n{}\n\n💡 Check your operator placement and parentheses.", message)
                }
                QueryError::InvalidCategory(category) => {
                    format!("❌ **Invalid Category**\n\n'{}' is not a recognized category.\n\n💡 Use `/categories` to see available categories.", category)
                }
                QueryError::ImpossibleBranch(issue) => {
                    format!("❌ **Impossible Query**\n\nThe query has conflicting categories: {}\n\n💡 Check for contradictory AND conditions.", issue)
                }
                QueryError::UnexpectedEOF => {
                    "❌ **Incomplete Query**\n\nThe query ended unexpectedly. Check for unclosed parentheses.\n\n💡 Make sure all parentheses are properly closed.".to_string()
                }
            };

            ctx.say(error_msg).await?;
        }
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
            commands: vec![tossup(), categories(), help(), query()],
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
                        .model("qwen3:1.7b")
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
