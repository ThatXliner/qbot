use ::serenity::all::Mentionable;
use poise::serenity_prelude as serenity;
use std::collections::HashSet;

use tokio::time::{Duration, Instant, sleep};

use tracing::info;

use crate::{Context, Data, Error, QuestionState, qb::Tossup};
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
        ctx.data().reading_states.lock().await.insert(
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
        let current_state = {
            let states = ctx.data().reading_states.lock().await;
            match states.get(&channel) {
                Some(state) => state.clone(),
                None => break,
            }
        };
        match &current_state.0 {
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
                let updated_state = {
                    let states = ctx.data().reading_states.lock().await;
                    match states.get(&channel) {
                        Some(state) => state.clone(),
                        None => {
                            info!("Post message edit: the state doesn't exist anymore; breaking");
                            break;
                        }
                    }
                };
                match updated_state.0 {
                    QuestionState::Reading => {
                        if chunk.join(" ").contains("(*)") {
                            info!("Post message edit: powering down");
                            let mut states = ctx.data().reading_states.lock().await;
                            if let Some(state) = states.get_mut(&channel) {
                                state.1 = false;
                            }
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
            QuestionState::Buzzed(user_id, _) => {
                channel
                    .say(
                        &ctx.http(),
                        format!("buzz from {}! 10 seconds to answer", user_id.mention()),
                    )
                    .await?;

                // Simple timeout approach: sleep for the buzz time and then check if the state is still buzzed
                sleep(Duration::from_secs(10)).await;
                {
                    let mut states = ctx.data().reading_states.lock().await;
                    if let Some(state) = states.get_mut(&channel) {
                        if let QuestionState::Buzzed(user_id, _) = state.0 {
                            info!("State transition into invalid");
                            // Timeout
                            state.0 = QuestionState::Invalid(user_id);
                        } else {
                            continue;
                        }
                    } else {
                        break;
                    }
                }
            }
            QuestionState::Invalid(user_id) => {
                {
                    let mut states = ctx.data().reading_states.lock().await;
                    if let Some(state) = states.get_mut(&channel) {
                        state.0 = QuestionState::Reading;
                        state.1 = !formatted.contains("(*)");
                        state.2.insert(*user_id);
                    }
                }
                channel.say(&ctx.http(), "No answer!").await?;
                continue;
            }
            QuestionState::Correct => {
                if current_state.1 {
                    channel.say(&ctx.http(), "CORRECT - power!").await?;
                } else {
                    channel.say(&ctx.http(), "CORRECT").await?;
                }
                break;
            }
        }
    }
    ctx.data().reading_states.lock().await.remove(&channel);
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
            info!("buzzed by {:?}", new_message.author.name);
            let current_state = {
                let states = data.reading_states.lock().await;
                match states.get(&new_message.channel_id) {
                    Some(state) => state.clone(),
                    None => {
                        info!(
                            "No reading state found for channel {}",
                            new_message.channel_id
                        );
                        return Ok(());
                    }
                }
            };
            info!(
                "Reading state found for channel {}: {:?}",
                new_message.channel_id, &current_state.0
            );
            match &current_state.0 {
                QuestionState::Reading => {
                    if new_message.content != "buzz" {
                        return Ok(());
                    }
                    let user_id = new_message.author.id;
                    let channel_id = new_message.channel_id;
                    if current_state.2.contains(&user_id) {
                        info!("Buzz skipped since user already buzzed");
                        return Ok(());
                    };

                    let buzz_timestamp = new_message.timestamp.unix_timestamp();
                    // State transition
                    {
                        let mut states = data.reading_states.lock().await;
                        if let Some(state) = states.get_mut(&channel_id) {
                            info!("State transition into Buzzed");
                            state.0 = QuestionState::Buzzed(
                                user_id,
                                // I'm going to use Discord's timestamps
                                buzz_timestamp,
                            );
                        }
                    }
                }
                QuestionState::Buzzed(id, timestamp) => {
                    if *id != new_message.author.id {
                        return Ok(());
                    }
                    if (new_message.timestamp.unix_timestamp() - timestamp) > 10 {
                        info!("Buzz skipped since time limit exceeded");
                        // State transition to invalid is bound to happen
                        return Ok(());
                    }
                    info!("Buzz accepted");
                    // State transition
                    {
                        let mut states = data.reading_states.lock().await;
                        if let Some(state) = states.get_mut(&new_message.channel_id) {
                            state.0 = QuestionState::Correct;
                            state.2.insert(new_message.author.id);
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}
