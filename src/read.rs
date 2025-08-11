use ::serenity::all::Mentionable;
use dashmap::DashMap;
use poise::serenity_prelude as serenity;
use std::collections::HashSet;

use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::time::Duration;

use tokio::time::sleep;
use tracing::info;

use crate::{Context, Data, Error, QuestionState, qb::Tossup};

const BUZZ_TIME_SECONDS: u64 = 10;
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
// TODO: this code structure is suicide for maintainance
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
    // Yea.. some dupe, right?
    // This is so we could move the sleep call to right before we check
    // for power. This way we maximize delay to allow humans to read
    // and get the power
    sleep(Duration::from_millis(750)).await;
    loop {
        info!("Pre reading loop");
        let Some(state) = ctx.data().reading_states.get(&channel) else {
            break;
        };
        info!("Reading loop: {:?}", &state.0);
        match &state.0 {
            QuestionState::Reading => {
                // The numbers here are arbitrarily chosen, empirically tuned
                // for a balance between reading speed and simulating 180 WPM
                // speaking speed
                let chunk = nth_chunk(&mut question, 5);
                if chunk.is_empty() {
                    break;
                }
                buffer.push(' ');
                buffer.push_str(&chunk.join(" "));
                message
                    .edit(
                        &ctx.http(),
                        serenity::EditMessage::new().content(buffer.clone()),
                    )
                    .await?;
                sleep(Duration::from_millis(750)).await;

                // Check for potential state changes between then and now
                let Some(state) = ctx.data().reading_states.get(&channel) else {
                    info!("Post message edit: the state doesn't exist anymore; breaking");
                    break;
                };
                match state.0 {
                    QuestionState::Reading => {
                        if chunk.join(" ").contains("(*)") {
                            info!("Post message edit: powering down");
                            ctx.data()
                                .reading_states
                                .entry(channel)
                                .and_modify(|value| value.1 = false);
                        };
                    }
                    // Since we're looping, this is functionally the same
                    // as "fallthrough" (which rust doesn't support)
                    _ => {
                        info!("Post message edit: state transition");
                        continue;
                    }
                }
            }
            QuestionState::Buzzed(user_id, notify, timestamp) => {
                channel
                    .say(
                        &ctx.http(),
                        format!("buzz from {}! 10 seconds to answer", user_id.mention()),
                    )
                    .await?;

                // Set up a timeout to check if the buzz expires

                tokio_scoped::scope(|scope| {
                    scope.spawn(async move {
                        sleep(Duration::from_secs(BUZZ_TIME_SECONDS)).await;
                        let data = ctx.data();
                        let Some(new_state) = data.reading_states.get(&channel) else {
                            return;
                        };
                        let QuestionState::Buzzed(new_user_id, notify, new_timestamp) =
                            &new_state.0
                        else {
                            return;
                        };
                        // XXX: deadlocks??
                        if *new_user_id == *user_id && *new_timestamp == *timestamp {
                            // Time is up, mark as invalid
                            data.reading_states.entry(channel).and_modify(|new_state| {
                                new_state.0 = QuestionState::Invalid(*user_id);
                                // The management of the ban list and sending
                                // the message to the channel is in the main FSM loop
                            });
                            notify.notify_one();
                        }
                    });
                    scope.block_on(notify.notified());
                })
            }
            QuestionState::Invalid(user_id) => {
                ctx.data()
                    .reading_states
                    .entry(channel)
                    .and_modify(|state| {
                        state.0 = QuestionState::Reading;
                        state.1 = !formatted.contains("(*)");
                        state.2.insert(*user_id);
                        // The management of the ban list and sending
                        // the message to the channel is in the main FSM loop
                    });
                channel.say(&ctx.http(), "No answer!").await?;
                continue;
            }
            QuestionState::Correct => {
                channel.say(&ctx.http(), "CORRECT").await?;
                break;
            }
        }
    }
    ctx.data().reading_states.remove(&channel);
    Ok(())
}

// #[instrument]
pub async fn event_handler(
    _ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            info!("{} is connected!", data_about_bot.user.name);
        }
        // Only manage state transitions
        serenity::FullEvent::Message { new_message } => {
            if new_message.author.bot {
                return Ok(());
            }
            if new_message.content != "buzz" {
                return Ok(());
            }
            info!("buzzed by {:?}", new_message.author.name);
            let Some(state) = data.reading_states.get(&new_message.channel_id) else {
                info!(
                    "No reading state found for channel {}",
                    new_message.channel_id
                );
                return Ok(());
            };
            info!(
                "Reading state found for channel {}: {:?}",
                new_message.channel_id, &state.0
            );
            match &state.0 {
                QuestionState::Reading => {
                    let user_id = new_message.author.id;
                    let channel_id = new_message.channel_id;
                    if state.2.contains(&user_id) {
                        info!("Buzz skipped since user already buzzed");
                        return Ok(());
                    };

                    let buzz_timestamp = new_message.timestamp.unix_timestamp();
                    info!("initiating state transition");
                    // State transition
                    data.reading_states.entry(channel_id).and_modify(|state| {
                        info!("State transition into Buzzed");
                        state.0 = QuestionState::Buzzed(
                            user_id,
                            Notify::new(),
                            // I'm going to use Discord's timestamps
                            buzz_timestamp,
                        );
                    });
                }
                QuestionState::Buzzed(id, notify, timestamp) => {
                    if *id != new_message.author.id {
                        return Ok(());
                    }
                    if (new_message.timestamp.unix_timestamp() - timestamp)
                        > BUZZ_TIME_SECONDS.try_into().unwrap()
                    {
                        info!("Buzz skipped since time limit exceeded");
                        // State transition to invalid is bound to happen
                        return Ok(());
                    }
                    info!("Buzz accepted");
                    // State transition
                    data.reading_states
                        .entry(new_message.channel_id)
                        .and_modify(|state| {
                            state.0 = QuestionState::Correct;
                            state.2.insert(new_message.author.id);
                        });
                    notify.notify_one();
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}
