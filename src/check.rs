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
static EXTRACT_SUB: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"<\w>(?P<inner>.+)</\w>").expect("Failed to compile regex")
});
static TEMPLATER: LazyLock<Tera> = LazyLock::new(|| {
    let mut output = Tera::new("templates/*.jinja").expect("Failed to parse templates");
    output
});
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
    // TODO: make this a bit smarter (like perhaps take into account of the prompt)
    prompted: bool,
) -> Result<Response, LLMError> {
    // TODO: normalize digits
    let mut context = tera::Context::new();
    context.insert(
        "question",
        &if answer_key.1.contains("read")|| answer_key.1.contains("before") || answer_key.1.contains("mention") {
            format!(
                "Here is the question read so far:
```
{}
```",
                question_so_far
            )
        } else {
            "Remember, judge strictly but be lenient on typos. Don't think about the question but simply compare the user's answer to the correct answer.".into()
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
    for normalized_answer in
        EXTRACT_SUB.captures_iter(&answer_key.0.replace("<u>", "").replace("</u>", ""))
    {
        if let Some(a) = normalized_answer.name("inner").map(|inner| inner.as_str()) {
            if levenshtein::distance(a.to_lowercase().chars(), answer.to_lowercase().chars())
                < FUZZY_THRESHOLD
            {
                return Ok(Response::Correct);
            }
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
#[cfg(test)]
mod tests {
    use llm::builder::LLMBuilder;

    use super::*;

    static LLM: LazyLock<Box<dyn LLMProvider>> = LazyLock::new(|| {
        LLMBuilder::new()
            .backend(llm::builder::LLMBackend::Ollama) // Use Ollama as the LLM backend
            .base_url(std::env::var("OLLAMA_URL").unwrap_or("http://127.0.0.1:11434".into())) // Set the Ollama server URL
            .model("qwen3:8b")
            .max_tokens(1000) // Set maximum response length
            .temperature(0.7) // Control response randomness (0.0-1.0)
            .stream(false) // Disable streaming responses
            .build()
            .expect("Failed to build LLM (Ollama)")
    });
    fn e(a: &str, b: &str) -> (String, String) {
        (a.to_string(), b.to_string())
    }
    #[tokio::test]
    async fn test_exact_match() {
        let result = check_correct_answer(
            &LLM,
            "What is the capital of France?",
            "Paris",
            &e("Paris", "Paris"),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Correct), "{:?}", result);
    }

    #[tokio::test]
    async fn test_incorrect_answer() {
        let result = check_correct_answer(
            &LLM,
            "What is the capital of France?",
            "London",
            &e("Paris", "Paris"),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Incorrect(_)), "{:?}", result);
    }

    #[tokio::test]
    async fn test_real_case_1() {
        let result = check_correct_answer(
            &LLM,
            "This quantity is related to a specific wavelength, lambda, by A lambda squared plus B plus C lambda to the minus two plus D lambda to the minus four, where A through D are material constants, in Cauchy's equation. It is sometimes useful to derive this quantity as the square root of relative permittivity times relative permeability. The arcsine of the ratio of this quantity for two media gives the critical angle for (*) total internal reflection. The ratio of this quantity for two media is equal to the ratio of the sine",
            "indxe fo refarction",
            &e("index of refraction [or n until it is read]", "index of <b>refraction</b> [or n until it is read]"),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Correct), "{:?}", result);
    }

    // leniency
    #[tokio::test]
    async fn test_real_case_2() {
        let result = check_correct_answer(
            &LLM,
            r#"The energy eigenspectrum associated with this system's quantum analogue can be solved for analytically using Hermite Polynomials or algebraically using the creation and annihilation operators. If its potential is truncated quadratically in the Taylor series centered around the minimum potential, any arbitrary system can be (*) modelled by this system. The general homogeneous solutions to this system's equations of motion are complex exponentials in time. Approximating sine of x to first order allows for the use of this system for ideal pendulums at small angles. For 10 points, name this physical system which can be used to model frictionless, Hookean springs."#,
            "simple harmonic system",
            &e(r#"simple harmonic oscillators (accept SHOs, prompt on "harmonic oscillators")"#, r#"simple harmonic oscillators (accept SHOs, prompt on "harmonic oscillators")"#),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Incorrect(_)), "{:?}", result);
    }

    // underlined part
    #[tokio::test]
    async fn test_real_case_3() {
        let result = check_correct_answer(
            &LLM,
            r#"Mark Moseley was playing for this team when he became the only placekicker to be awarded MVP. This team reached Super Bowl VII ["seven"] with a team of veterans nicknamed the "Over the Hill Gang". Gary Clark and Ricky Sanders joined a member of "The Fun Bunch", Art Monk, in a wide receiver trio for this team nicknamed "The (*) Posse". Cornerback Darrell Green played his entire career for this team. In the 2016 playoffs, this winner of the NFC East lost to the Green Bay Packers at their home stadium of FedExField. For 10 points, name this NFL team whose name combines a controversial slang term for Native Americans with the US capital."#,
            "redskins",
            &e(r#"<b><u>Washington</u></b> <b><u>Redskins</u></b> [accept either underlined part]"#, r#"Washington Redskins [accept either underlined part]"#),
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Correct), "{:?}", result);
    }
}
