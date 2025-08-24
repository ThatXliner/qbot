use std::sync::LazyLock;

use llm::{chat::ChatMessage, error::LLMError, LLMProvider};
use rapidfuzz::distance::levenshtein;
use serde::{Deserialize, Serialize};
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
const FUZZY_THRESHOLD: f64 = 0.3;
fn cosine_similarity(a: &Vec<f32>, b: &Vec<f32>) -> f64 {
    let dot_product = a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    dot_product as f64 / ((norm_a * norm_b) as f64)
}
#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    prompt: String,
}
#[derive(Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}
// we hardcode this bois
async fn get_embedding(http: &reqwest::Client, text: &str) -> Result<Vec<f32>, reqwest::Error> {
    let url = format!(
        "{}/api/embeddings",
        std::env::var("OLLAMA_URL").unwrap_or("http://127.0.0.1:11434".into())
    );

    let body = EmbeddingRequest {
        model: "nomic-embed-text".into(),
        prompt: text.into(),
    };

    let resp = http
        .post(&url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    let json_resp: EmbeddingResponse = resp.json().await?;
    Ok(json_resp.embedding)
}

const ENABLE_LEVENSHTEIN_DISTANCE: bool = true;
const ENABLE_EMBEDDING_DISTANCE: bool = false;
#[allow(clippy::borrowed_box)]
pub async fn check_correct_answer(
    llm: &Box<dyn LLMProvider>,
    http: &reqwest::Client,
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
    info!("Checking answer for question: {}", question_so_far);
    info!("Answer: {:?}", answer_key);
    info!("Normalized Answer: {}", normalized_answer);
    info!("User answer: {}", answer);
    if ENABLE_LEVENSHTEIN_DISTANCE {
        if levenshtein::normalized_distance(
            normalized_answer.to_lowercase().chars(),
            answer.to_lowercase().chars(),
        ) < FUZZY_THRESHOLD
        {
            info!("Levenshtein distance is below threshold");
            return Ok(Response::Correct);
        };
        // Levenshtein distance on extracted subword
        for (_, [sub_normalized_answer]) in EXTRACT_SUB
            .captures_iter(&answer_key.0.replace("<b>", "").replace("</b>", ""))
            .map(|capture| capture.extract())
        {
            // TODO: make sure this wasn't being surrounded by "prompt on"
            // (use the LLM)
            if levenshtein::normalized_distance(
                sub_normalized_answer.to_lowercase().chars(),
                answer.to_lowercase().chars(),
            ) < FUZZY_THRESHOLD
            {
                info!(
                    "Checked sub answer {} and levenshtein distance is below threshold",
                    sub_normalized_answer
                );
                return Ok(Response::Correct);
            }
        }
    }
    if ENABLE_EMBEDDING_DISTANCE {
        let similarity = cosine_similarity(
            &get_embedding(&http, &answer)
                .await
                .map_err(|e| LLMError::HttpError(format!("{:?}", e)))?,
            &get_embedding(&http, &normalized_answer)
                .await
                .map_err(|e| LLMError::HttpError(format!("{:?}", e)))?,
        );
        if similarity >= 0.9 {
            info!("It's semantically similar enough");
            return Ok(Response::Correct);
        }
        if similarity >= 0.8 {
            return Ok(Response::Prompt("PROMPT".to_string()));
        }
        info!("Similarity: {} | insufficient", similarity);
    }

    // if answer_key.0.contains("prompt") {
    let messages = vec![ChatMessage::user()
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
        .build()];

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
    // } else {
    // Ok(Response::Incorrect(
    // "(Insufficiently close + no instructions to prompt)".into(),
    // ))
    // }
}
