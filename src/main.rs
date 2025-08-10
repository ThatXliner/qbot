use poise::serenity_prelude as serenity;

use qbot::{Data, read};

use qbot::qb::{format_question, random_tossup};
use qbot::query::{ApiQuery, CATEGORIES, parse_query};
use qbot::{Context, Error};

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn tossup(
    ctx: Context<'_>,
    #[description = "Query for selecting the category"] query: Option<String>,
) -> Result<(), Error> {
    if let Some(query) = query {
        let parsed_results = parse_query(&query);
        if let Some(channel) = ctx.guild_channel().await {
            channel
                .say(&ctx.http(), format!("```json\n{:?}\n```\n", parsed_results))
                .await?;
        }
        if let Ok(api_params) = parsed_results {
            let reqwest = &ctx.data().reqwest;
            let get_tossup = random_tossup(reqwest, &api_params).await?;
            if let Some(tossup) = get_tossup.tossups.get(0) {
                ctx.say(format_question(&tossup.question)).await?;
            }
            // if let Some(channel) = ctx.guild_channel().await {
            //     ctx.say("Got it").await?;
            //     channel
            //         .say(
            //             &ctx.http(),
            //             format!(
            //                 "```json\n{:?}\n```\n",
            //                 random_tossup(reqwest, &api_params)
            //                     .await?
            //                     .tossups
            //                     .get(0)
            //                     .unwrap()
            //                     .question
            //             ),
            //         )
            //         .await?;
            // }
        }
    } else {
        let reqwest = &ctx.data().reqwest;
        let get_tossup = random_tossup(reqwest, &ApiQuery::default()).await?;
        if let Some(tossup) = get_tossup.tossups.get(0) {
            ctx.say(format_question(&tossup.question)).await?;
        }
    }
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
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![tossup(), categories()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    reqwest: reqwest::Client::new(),
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .event_handler(read::Handler)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
