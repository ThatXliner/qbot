use ::serenity::all::{Mentionable, ReactionType};
use poise::serenity_prelude as serenity;
use std::collections::HashSet;
use tokio::task;

use tokio::sync::watch;
use tokio::time::{Duration, timeout};
use tracing::debug;

use crate::check::check_correct_answer;
use crate::{Context, Data, Error, QuestionState, qb::Tossup};
/// In case we send this to an LLM
fn render_html(answer: &str) -> String {
    answer
        .replace("<b>", "**")
        .replace("</b>", "**")
        .replace("<i>", "_")
        .replace("</i>", "_")
        .replace("<u>", "__")
        .replace("</u>", "__")
}
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
            ),
        );
    }
    let mut message = ctx.say(buffer.clone()).await?.into_message().await?;
    // Yea.. some dupe, right?
    // This is so we could move the sleep call to right before we check
    // for power. This way we maximize delay to allow humans to read
    // and get the power
    let mut do_depower = false;
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
                // we defer our depower to after all state transitions have settled
                // so we can help the user get the power
                do_depower = chunk.join(" ").contains("(*)");
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

                // wait until the user answers or the timeout is reached
                if timeout(Duration::from_secs(10), state_change_rx.changed())
                    .await
                    // timeout has been reached
                    .is_err()
                {
                    let mut states = ctx.data().reading_states.lock().await;
                    if let Some(state) = states.get_mut(&channel) {
                        if let QuestionState::Buzzed(user_id, _) = state.0 {
                            debug!("State transition into invalid (timeout)");
                            state.0 = QuestionState::Invalid(user_id);
                            // Notify about state change
                            let _ = state.3.send(());
                        }
                    }
                }
                // otherwise, let the state fallthrough the next loop
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
                if current_state.1 {
                    channel.say(&ctx.http(), "Correct - power!").await?;
                } else {
                    channel.say(&ctx.http(), "Correct").await?;
                }
                // reveal correct answer
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
        // I feel like != false is more readable than current_state.1
        // because what we're saying here is "if it's not false, we make it false"
        if do_depower && current_state.1 != false {
            let mut states = ctx.data().reading_states.lock().await;
            if let Some(state) = states.get_mut(&channel) {
                state.1 = false;
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
            debug!("buzzed by {:?}", new_message.author.name);
            let current_state = {
                let states = data.reading_states.lock().await;
                match states.get(&new_message.channel_id) {
                    // Yo... is cloning all of this fine?
                    Some(state) => (
                        state.0.clone(),
                        state.1,
                        state.2.clone(),
                        state.3.clone(),
                        state.4.clone(),
                    ),
                    None => {
                        debug!(
                            "No reading state found for channel {}",
                            new_message.channel_id
                        );
                        return Ok(());
                    }
                }
            };
            debug!(
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
                        debug!("Buzz skipped since user already buzzed");
                        new_message
                            .react(&ctx.http, ReactionType::Unicode("❌".into()))
                            .await?;
                        return Ok(());
                    };

                    let buzz_timestamp = new_message.timestamp.unix_timestamp();
                    // State transition
                    {
                        let mut states = data.reading_states.lock().await;
                        if let Some(state) = states.get_mut(&channel_id) {
                            debug!("State transition into Buzzed");
                            state.0 = QuestionState::Buzzed(
                                user_id,
                                // I'm going to use Discord's timestamps
                                buzz_timestamp,
                            );
                            // Notify about state change
                            let _ = state.3.send(());
                        }
                    }
                }
                QuestionState::Buzzed(id, timestamp) => {
                    if *id != new_message.author.id {
                        return Ok(());
                    }
                    if (new_message.timestamp.unix_timestamp() - timestamp) > 10 {
                        debug!("Buzz skipped since time limit exceeded");
                        // State transition to invalid is bound to happen
                        return Ok(());
                    }
                    debug!("Buzz accepted");
                    // State transition
                    // I don't want to clone
                    if check_correct_answer(current_state.4.clone(), new_message.content.as_str()) {
                        let mut states = data.reading_states.lock().await;
                        if let Some(state) = states.get_mut(&new_message.channel_id) {
                            state.0 = QuestionState::Correct;
                            state.2.insert(new_message.author.id);
                            // Notify about state change
                            let _ = state.3.send(());
                        }
                    } else {
                        let mut states = data.reading_states.lock().await;
                        if let Some(state) = states.get_mut(&new_message.channel_id) {
                            state.0 = QuestionState::Incorrect(new_message.author.id);
                            state.2.insert(new_message.author.id);
                            // Notify about state change
                            let _ = state.3.send(());
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
