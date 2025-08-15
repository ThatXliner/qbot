use ::serenity::all::{Mentionable, ReactionType};
use poise::serenity_prelude as serenity;
use std::collections::HashSet;
use std::thread::sleep;

use tokio::task;

use tokio::sync::watch;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

use crate::check::{check_correct_answer, Response};
use crate::utils::*;
use crate::{qb::Tossup, Context, Data, Error, QuestionState};
// TODO: this code structure is suicide for maintainance
pub async fn read_question(
    ctx: &Context<'_>,
    tossups: Vec<Tossup>,
    say: bool,
) -> Result<(), Error> {
    let Some(tossup) = tossups.first() else {
        ctx.say("No tossups found").await?;
        return Ok(());
    };
    // Having it bold (or formatted in any manner) is kinda annoying
    let formatted = format_question(&tossup.question_sanitized);
    let mut question = formatted.split(' ');
    // Start off with 3, an arbitrarily chosen small number
    let chunk = nth_chunk(&mut question, 3);
    let mut buffer = chunk.join(" ");
    let channel = ctx.channel_id();

    // Create notification channel for state changes
    // lowk this is inefficient, and I should switch this to a notifier
    // but you know what they say: if it ain't broke, don't fix it
    let (state_change_tx, mut state_change_rx) = watch::channel(());

    // Might be unnecessary but scoped to avoid deadlocks
    {
        ctx.data().reading_states.lock().await.insert(
            channel,
            (
                QuestionState::Reading,
                !formatted.contains("(*)"),
                HashSet::new(),
                state_change_tx,
                // sob... but it should be fine
                tossup.clone(),
                buffer.clone(),
            ),
        );
    }

    let mut message = if say {
        ctx.say(buffer.clone()).await?.into_message().await?
    } else {
        ctx.channel_id().say(&ctx.http(), buffer.clone()).await?
    };

    loop {
        // Let potential state transitions happen first
        task::yield_now().await;
        let current_state = {
            let states = ctx.data().reading_states.lock().await;
            match states.get(&channel) {
                Some(state) => (state.0.clone(), state.1, state.2.clone()),
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
                    task::yield_now().await;
                    sleep(Duration::from_secs(1));
                    if timeout(Duration::from_secs(5), state_change_rx.changed())
                        .await
                        .is_ok()
                    {
                        continue;
                    }
                    channel.say(&ctx.http(), "Time's up!").await?;
                    let states = ctx.data().reading_states.lock().await;
                    match states.get(&channel) {
                        Some(state) => {
                            channel
                                .say(&ctx.http(), &render_html(&state.4.answer))
                                .await?;
                        }
                        None => break,
                    }
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
                {
                    let mut states = ctx.data().reading_states.lock().await;
                    if let Some(state) = states.get_mut(&channel) {
                        state.5.push(' ');
                        state.5.push_str(&chunk.join(" "));
                    }
                }
                task::yield_now().await;
                if timeout(Duration::from_millis(750), state_change_rx.changed())
                    .await
                    .is_ok()
                {
                    continue;
                }
            }
            QuestionState::Buzzed(user_id, _) => {
                buffer.push_str(":bell:");
                message
                    .edit(&ctx.http(), serenity::EditMessage::new().content(&buffer))
                    .await?;
                channel
                    .say(
                        &ctx.http(),
                        format!("buzz from {}! 10 seconds to answer", user_id.mention()),
                    )
                    .await?;
                task::yield_now().await;

                // wait until the user answers or the timeout is reached
                if timeout(Duration::from_secs(10), state_change_rx.changed())
                    .await
                    // timeout has been reached
                    .is_err()
                {
                    info!("Time out reached! (buzz)");
                    let mut states = ctx.data().reading_states.lock().await;
                    if let Some(state) = states.get_mut(&channel) {
                        if state.0 == QuestionState::Judging {
                            // Ok idk what happened here but clearly this is a possible state
                            // transition
                            continue;
                        }
                        debug!("State transition into invalid (timeout)");
                        state.0 = QuestionState::Invalid(*user_id);
                        // Notify about state change
                        let _ = state.3.send(());
                    }
                }
                // otherwise, let the state fallthrough the next loop
            }
            // nearly identical to Buzzed
            QuestionState::Prompt(user_id, prompt, _) => {
                channel
                    .say(&ctx.http(), format!("{} {}", prompt, user_id.mention()))
                    .await?;
                task::yield_now().await;
                // wait until the user answers or the timeout is reached
                if timeout(Duration::from_secs(5), state_change_rx.changed())
                    .await
                    // timeout has been reached
                    .is_err()
                {
                    info!("Time out reached! (prompt)");
                    let mut states = ctx.data().reading_states.lock().await;
                    if let Some(state) = states.get_mut(&channel) {
                        if state.0 == QuestionState::Judging {
                            // Ok idk what happened here but clearly this is a possible state
                            // transition
                            continue;
                        }
                        debug!("State transition into invalid (timeout)");
                        state.0 = QuestionState::Invalid(*user_id);
                        // Notify about state change
                        let _ = state.3.send(());
                    }
                }
                continue;
            }
            QuestionState::Judging => {
                task::yield_now().await;
                // Wait for state change
                state_change_rx.changed().await?;

                continue;
            }
            QuestionState::Invalid(user_id) => {
                let mut states = ctx.data().reading_states.lock().await;
                if let Some(state) = states.get_mut(&channel) {
                    state.0 = QuestionState::Reading;
                    state.1 = !formatted.contains("(*)");
                    state.2.insert(*user_id);
                    // Notify about state change
                    let _ = state.3.send(());
                }
                channel.say(&ctx.http(), "No answer!").await?;
                buffer = buffer.replace(":bell:", ":no_bell:");
                message = channel.say(&ctx.http(), &buffer).await?;
                continue;
            }

            QuestionState::Incorrect(user_id) => {
                let mut states = ctx.data().reading_states.lock().await;
                if let Some(state) = states.get_mut(&channel) {
                    state.0 = QuestionState::Reading;
                    state.1 = !formatted.contains("(*)");
                    state.2.insert(*user_id);
                    // Notify about state change
                    let _ = state.3.send(());
                }
                channel.say(&ctx.http(), "incorrect!").await?;
                buffer = buffer.replace(":bell:", ":no_bell:");
                message = channel.say(&ctx.http(), &buffer).await?;
                continue;
            }
            QuestionState::Correct => {
                if formatted.contains("(\\*)") && !buffer.contains("(\\*)") {
                    channel.say(&ctx.http(), "Correct - power!").await?;
                } else {
                    channel.say(&ctx.http(), "Correct").await?;
                }
                // reveal correct answer
                buffer.push(' ');
                buffer.push_str(&question.collect::<Vec<&str>>().join(" "));
                message
                    // no need to clone since we won't ever use it again
                    .edit(&ctx.http(), serenity::EditMessage::new().content(buffer))
                    .await?;
                // TODO: bold matching parts

                let states = ctx.data().reading_states.lock().await;
                match states.get(&channel) {
                    Some(state) => {
                        channel
                            .say(&ctx.http(), &render_html(&state.4.answer))
                            .await?;
                    }
                    None => break,
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
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            debug!("{} is connected!", data_about_bot.user.name);
        }
        // Only manage state transitions
        serenity::FullEvent::Message { new_message } => {
            if new_message.author.bot {
                return Ok(());
            }
            let current_state = {
                let states = data.reading_states.lock().await;
                match states.get(&new_message.channel_id) {
                    // Yo... is cloning all of this fine?
                    Some(state) => (
                        state.0.clone(),
                        state.1,
                        state.2.contains(&new_message.author.id),
                        // Answer sanitized
                        (state.4.answer.clone(), state.4.answer_sanitized.clone()),
                        // Question so far
                        state.5.clone(),
                    ),
                    None => {
                        return Ok(());
                    }
                }
            };
            match &current_state.0 {
                QuestionState::Reading => {
                    if new_message.content.to_lowercase() != "buzz" {
                        return Ok(());
                    }
                    let channel_id = new_message.channel_id;
                    if current_state.2 {
                        debug!("Buzz skipped since user already buzzed");
                        new_message
                            .react(&ctx.http, ReactionType::Unicode("❌".into()))
                            .await?;
                        return Ok(());
                    };

                    // State transition
                    {
                        let mut states = data.reading_states.lock().await;
                        if let Some(state) = states.get_mut(&channel_id) {
                            debug!("State transition into Buzzed");
                            state.0 = QuestionState::Buzzed(
                                new_message.author.id,
                                // I'm going to use Discord's timestamps
                                new_message.timestamp.unix_timestamp(),
                            );
                            // Notify about state change
                            let _ = state.3.send(());
                        }
                    }
                }
                QuestionState::Buzzed(user_id, timestamp) => {
                    if *user_id != new_message.author.id {
                        return Ok(());
                    }
                    let new_message_timestamp = new_message.timestamp.unix_timestamp();
                    if (new_message_timestamp - timestamp) > 10 {
                        debug!("Buzz skipped since time limit exceeded");
                        // State transition to invalid is bound to happen
                        return Ok(());
                    }
                    let mut states = data.reading_states.lock().await;
                    if let Some(state) = states.get_mut(&new_message.channel_id) {
                        new_message.reply(&ctx.http, "Judging...").await?;
                        state.0 = QuestionState::Judging;
                        let _ = state.3.send(());
                        state.0 = match check_correct_answer(
                            &data.llm,
                            &data.reqwest,
                            &current_state.4,
                            new_message.content.as_str(),
                            &current_state.3,
                            false,
                        )
                        .await
                        .map_err(|_|"Failed to access LLM")?
                        // State transition
                        {
                            Response::Correct => QuestionState::Correct,
                            Response::Incorrect(_) => {
                                state.2.insert(new_message.author.id);
                                QuestionState::Incorrect(new_message.author.id)
                            }
                            Response::Prompt(text) => QuestionState::Prompt(
                                new_message.author.id,
                                text,
                                new_message_timestamp,
                            ),
                        };
                        // Notify about state change
                        let _ = state.3.send(());
                    }
                }
                QuestionState::Prompt(user_id, _, timestamp) => {
                    if *user_id != new_message.author.id {
                        return Ok(());
                    }
                    let new_message_timestamp = new_message.timestamp.unix_timestamp();
                    if (new_message_timestamp - timestamp) > 10 {
                        debug!("Buzz skipped since time limit exceeded");
                        // State transition to invalid is bound to happen
                        return Ok(());
                    }
                    let mut states = data.reading_states.lock().await;
                    if let Some(state) = states.get_mut(&new_message.channel_id) {
                        new_message.reply(&ctx.http, "Judging...").await?;
                        state.0 = QuestionState::Judging;
                        let _ = state.3.send(());

                        state.0 = match check_correct_answer(
                            &data.llm,
                            &data.reqwest,
                            &current_state.4,
                            new_message.content.as_str(),
                            &current_state.3,
                            true,
                        )
                        .await
                        .map_err(|_|"Failed to access LLM")?
                        // State transition
                        {
                            Response::Correct => QuestionState::Correct,
                            Response::Incorrect(_) | Response::Prompt(_) => {
                                state.2.insert(new_message.author.id);
                                QuestionState::Incorrect(new_message.author.id)
                            }
                        };
                        // Notify about state change
                        let _ = state.3.send(());
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}
