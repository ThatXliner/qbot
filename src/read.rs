use std::collections::HashSet;
use tokio::time::Duration;

use serenity::{
    all::{self, EditMessage, EventHandler, Ready},
    async_trait,
};
use tokio::time::sleep;
use tracing::info;

use crate::{Context, Error, QuestionState, qb::Tossup};
pub struct Handler;

fn format_question(question: &str) -> String {
    question.replace("*", "\\*")
    // .replace("<b>", "**")
    // .replace("</b>", "**")
    // .replace("<i>", "_")
    // .replace("</i>", "_")
    // .replace("(*)", ":star:")
}
fn nth_chunk<I: Iterator>(mut iter: I, n: usize) -> Vec<I::Item> {
    iter.by_ref().take(n).collect()
}

pub async fn read_question(ctx: &Context<'_>, tossups: Vec<Tossup>) -> Result<(), Error> {
    let Some(tossup) = tossups.first() else {
        ctx.say("No tossups found").await?;
        return Ok(());
    };
    // Having it bold (or formatted in any manner) is kinda annoying
    let formatted = format_question(&tossup.question_sanitized);
    let mut question = formatted.split(' ');
    // Start off with 5, an arbitrarily chosen small number
    let chunk = nth_chunk(&mut question, 5);
    let mut buffer = chunk.join(" ");
    let channel = ctx.channel_id();
    // Might be unnecessary but scoped to avoid deadlocks
    {
        ctx.data().reading_states.insert(
            channel,
            (
                QuestionState::Reading,
                !formatted.contains("(*)"),
                HashSet::new(),
            ),
        );
    }
    let mut message = ctx.say(buffer.clone()).await?.into_message().await?;
    loop {
        let Some(state) = ctx.data().reading_states.get(&channel) else {
            break;
        };
        match &state.0 {
            QuestionState::Reading => {
                // The numbers here are arbitrarily chosen, empirically tuned
                // for a balance between reading speed and simulating 180 WPM
                // speaking speed
                let chunk = nth_chunk(&mut question, 4);
                if chunk.is_empty() {
                    break;
                }
                buffer.push(' ');
                buffer.push_str(&chunk.join(" "));
                sleep(Duration::from_millis(750)).await;
                message
                    .edit(&ctx.http(), EditMessage::new().content(buffer.clone()))
                    .await?;
                if chunk.join(" ").contains("(*)") {
                    ctx.data()
                        .reading_states
                        .entry(channel)
                        .and_modify(|value| value.1 = false);
                }
            }
            QuestionState::Buzzed(user_id, notify) => {
                // TODO
                notify.notified().await;
                // if ctx.data().reading_states.get(&channel).is_none() {
                //     // It was the correct answer
                //     break;
                // }
            }
            QuestionState::Invalid(user) => {
                // XXX: there's got to be a more efficient way to do this
                let mut new_users = state.2.clone();
                new_users.insert(*user);
                ctx.data().reading_states.insert(
                    channel,
                    (
                        QuestionState::Reading,
                        !formatted.contains("(*)"),
                        new_users,
                    ),
                );
                continue;
            }
            QuestionState::Correct => {
                break;
            }
        }
    }
    Ok(())
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: all::Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        // for guild in ready.guilds {
        //     info!(
        //         "Ready to verify users in guild: {}",
        //         guild
        //             .id
        //             .name(&ctx.cache)
        //             .get_or_insert("Unknown guild".into())
        //     );
        // }
    }
    // async fn message(&self, ctx: serenity_prelude::Context, new_message: Message) {
    //     if new_message.author.bot {
    //         return;
    //     }
    //     // println!("{:?}", ctx.data);
    // }
    //     if &ctx
    //         .data
    //         .blocking_read()
    //         .reading_states
    //         .contains_key(&new_message.channel_id)
    //     {
    //         state.insert(new_message.channel_id, new_message.content);
    //     }
    // }
    // async fn guild_member_addition(&self, ctx: serenity::Context, new_member: serenity::Member) {
    //     info!("New member: {}", new_member.user.id);
    //     if db::is_verified(&new_member.guild_id).await {
    //         info!("Already verified {}, welcome back!", new_member.user.id);
    //         add_verify_roles(&ctx, &new_member)
    //             .await
    //             .unwrap_or_else(|err| error!("Error adding verified role: {}", err));
    //         STAFF_CHANNEL
    //             .say(&ctx, "Already verified. Welcome back!")
    //             .await
    //             .map(|_| ())
    //             .unwrap_or_else(|err| error!("Error sending message: {}", err));
    //     }
    //     queue_verification(&ctx, &new_member)
    //         .await
    //         .unwrap_or_else(|_| error!("Could not notify user about verification"));
    // }
}
