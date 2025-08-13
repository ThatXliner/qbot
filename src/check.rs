use std::sync::LazyLock;

use llm::{LLMProvider, chat::ChatMessage, error::LLMError};
use rapidfuzz::distance::levenshtein;
use tera::Tera;
use tracing::{error, info};
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Correct,
    Incorrect(String),
    Prompt(String),
}
static PROMPT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?s)<think>.+</think>\s+").expect("Failed to compile regex")
});
static ANSWER_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\s+(\(|\[).+$").expect("Failed to compile regex"));
static EXTRACT_SUB: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"<\w>(.+?)</\w>").expect("Failed to compile regex"));
static TEMPLATER: LazyLock<Tera> =
    LazyLock::new(|| Tera::new("templates/latest/*.jinja").expect("Failed to parse templates"));
// Threshold for fuzzy matching
// intentionally separate from its appearance in the other file
const FUZZY_THRESHOLD: usize = 5;
pub async fn check_correct_answer(
    llm: &Box<dyn LLMProvider>,
    // TODO: maybe input the whole question with a mark of where we left off
    question_so_far: &str,
    answer: &str,
    // (answer, answer_sanitized)
    answer_key: &(String, String),
    // TODO: account for this in the prompt
    prompted: bool,
) -> Result<Response, LLMError> {
    // TODO: normalize digits
    let mut context = tera::Context::new();
    context.insert(
        "question",
        &if answer_key.1.contains("read")|| answer_key.1.contains("before") || answer_key.1.contains("mention") {
            format!(
                "Since deciding on whether to prompt or mark as incorrect depends on how far we've read, I will also provide the question. Here is the question read so far:
```
{}
```",
                question_so_far
            )
        } else {
            "Remember, don't think about the question but simply compare the user's answer to the correct answer.".into()
        },
    );
    context.insert("response", answer);
    context.insert("answer", &answer_key.0);
    // Basic levenshtein distance
    let normalized_answer = ANSWER_RE.replace(&answer_key.1, "").into_owned();
    if levenshtein::distance(
        normalized_answer.to_lowercase().chars(),
        answer.to_lowercase().chars(),
    ) < FUZZY_THRESHOLD
    {
        return Ok(Response::Correct);
    };
    // Levenshtein distance on extracted subword
    for (_, [normalized_answer]) in EXTRACT_SUB
        .captures_iter(&answer_key.0.replace("<b>", "").replace("</b>", ""))
        .map(|capture| capture.extract())
    {
        if levenshtein::distance(
            normalized_answer.to_lowercase().chars(),
            answer.to_lowercase().chars(),
        ) < FUZZY_THRESHOLD
        {
            return Ok(Response::Correct);
        }
    }
    // TODO: add "matches a subword"?
    // TODO: add word2vec
    let messages = vec![
        ChatMessage::user()
            .content(
                TEMPLATER
                    .render(
                        if prompted {
                            "prompt_no_prompt.jinja"
                        } else {
                            "prompt.jinja"
                        },
                        &context,
                    )
                    .unwrap(),
            )
            .build(),
    ];

    info!("Checking answer for question: {}", question_so_far);
    info!("Answer: {:?}", answer_key);
    info!("Normalized Answer: {}", normalized_answer);
    info!("User answer: {}", answer);
    llm.chat(&messages)
        .await
        .map(|response| response.text().expect("LLM did not respond"))
        .map(|text|{
            info!("LLM raw response: {}", text);
            (text.clone(),PROMPT_RE.replace(&text,"").into_owned())
        })
        .map(|(raw,text)| {
            let response = text.trim();
            info!("LLM response: {}", response);


            match response {
                "CORRECT" => Response::Correct,
                "INCORRECT" => Response::Incorrect(raw),
                text => {
                    if prompted {
                        error!("Judge did not respond with 'INCORRECT' or 'CORRECT' to prompt, but instead: {}", text);
                        Response::Incorrect(raw)
                    } else {
                        Response::Prompt(text.to_string())
                    }
                    // If the response starts with "PROMPT: ", we extract the prompt
                    // Otherwise, we just return the response as a prompt
                    // This allows us to handle both cases where the judge provides a prompt or not
                    //     // if text.starts_with("PROMPT: ") {
                    //     //     let prompt = text[8..].trim().to_string();
                    //     //     return Response::Prompt(prompt);
                    //     // }

                }
            }
        })
}
