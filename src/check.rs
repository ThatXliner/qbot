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
static TEMPLATER: LazyLock<Tera> = LazyLock::new(|| {
    let mut output = Tera::default();

    output.add_raw_template("prompt", r#"You're now a national-level Quiz Bowl judge. I will provide you the question read so far, our contestant's answer (and whether or not it is a response to a prompt), and the answer key. The answer key may contain hints on how to grade their response. 
    
You may only respond with one of:
- "CORRECT", meaning the answer matches the answer key exactly or is an acceptable equivalent.
- "INCORRECT", meaning the answer is wrong, incomplete, or outside the acceptable range
- or "PROMPT", meaning the contestant's answer is on the right track but too vague, incomplete, or ambiguous.

Only PROMPT if the answer could be correct but needs clarification or specificity. If you do decide to answer with "PROMPT", you may optionally include a clarifying question (but only if specified in the answer key) like so: "PROMPT: which cell?". Otherwise, simply respond with "PROMPT"

You may judge our contestant somewhat leniently, so semantically equivalent statements or typos may be considered correct (e.g. "burners lee" vs "Tim Berners-Lee" or "the peroidic table" vs "The Periodic Table of Elements"). Try to be as lenient as possible, but also be strict enough to ensure that the contestant is not simply guessing.

Here is the question read so far:
```
{{ question }}
```
Here is our contestant's response:
```
{{ response }}
```
Here is the answer key:
```
{{ answer }}
```

Judge, what is your response?"#).unwrap();
    output.add_raw_template("prompt_no_prompt", r#"You're now a national-level Quiz Bowl judge. I will provide you the question read so far, our contestant's answer, and the answer key. The answer key may contain hints on how to grade their response.
    
You may only respond with one of "CORRECT", "INCORRECT". Typically, you would also have the option to respond with "PROMPT" and a clarifying question, but in this case you do NOT have that option since our contestant is currently responding to a prompt (and you cannot prompt them more than once).

You may judge our contestant somewhat leniently, so semantically equivalent statements or typos may be considered correct (e.g. "burners lee" vs "Tim Berners-Lee" or "the peroidic table" vs "The Periodic Table of Elements")

Here is the question read so far:
```
{{ question }}
```
Here is our contestant's response:
```
{{ response }}
```
Here is the answer key:
```
{{ answer }}
```

Judge, what is your response?"#).unwrap();
    output
});

pub async fn check_correct_answer(
    llm: &Box<dyn LLMProvider>,
    question_so_far: &str,
    answer: &str,
    answer_key: &str,
    // TODO: make this a bit smarter (like perhaps take into account of the prompt)
    prompted: bool,
) -> Result<Response, LLMError> {
    // TODO: normalize digits
    let mut context = tera::Context::new();
    context.insert("question", question_so_far);
    context.insert("response", answer);
    context.insert("answer", answer_key);
    let normalized_answer = ANSWER_RE.replace(&answer_key, "").into_owned();
    if levenshtein::distance(normalized_answer.chars(), answer.chars()) < 5 {
        return Ok(Response::Correct);
    }
    // TODO: add word2vec
    let messages = vec![
        ChatMessage::user()
            .content(
                TEMPLATER
                    .render(
                        if prompted {
                            "prompt_no_prompt"
                        } else {
                            "prompt"
                        },
                        &context,
                    )
                    .unwrap(),
            )
            .build(),
    ];

    info!("Checking answer for question: {}", question_so_far);
    info!("Answer: {}", answer_key);
    info!("Normalized Answer: {}", normalized_answer);
    info!("User answer: {}", answer);
    llm.chat(&messages)
        .await
        .map(|response| response.text().expect("LLM did not respond with text"))
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

    static llm: LazyLock<Box<dyn LLMProvider>> = LazyLock::new(|| {
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
    #[tokio::test]
    async fn test_exact_match() {
        let result = check_correct_answer(
            &llm,
            "What is the capital of France?",
            "Paris",
            "Paris",
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Correct), "{:?}", result);
    }

    #[tokio::test]
    async fn test_close_match() {
        let result = check_correct_answer(
            &llm,
            "Who invented the World Wide Web?",
            "Tim burners lee",
            "Tim Berners-Lee",
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Correct), "{:?}", result);
    }

    #[tokio::test]
    async fn test_incorrect_answer() {
        let result = check_correct_answer(
            &llm,
            "What is the capital of France?",
            "London",
            "Paris",
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Incorrect(_)), "{:?}", result);
    }

    #[tokio::test]
    async fn test_real_case_1() {
        let result = check_correct_answer(
            &llm,
            "This quantity is related to a specific wavelength, lambda, by A lambda squared plus B plus C lambda to the minus two plus D lambda to the minus four, where A through D are material constants, in Cauchy's equation. It is sometimes useful to derive this quantity as the square root of relative permittivity times relative permeability. The arcsine of the ratio of this quantity for two media gives the critical angle for (*) total internal reflection. The ratio of this quantity for two media is equal to the ratio of the sine",
            "indxe fo refarction",
            "index of refraction [or n until it is read]",
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
            &llm,
            r#"The energy eigenspectrum associated with this system's quantum analogue can be solved for analytically using Hermite Polynomials or algebraically using the creation and annihilation operators. If its potential is truncated quadratically in the Taylor series centered around the minimum potential, any arbitrary system can be (*) modelled by this system. The general homogeneous solutions to this system's equations of motion are complex exponentials in time. Approximating sine of x to first order allows for the use of this system for ideal pendulums at small angles. For 10 points, name this physical system which can be used to model frictionless, Hookean springs."#,
            "simple harmonic system",
            r#"simple harmonic oscillators (accept SHOs, prompt on "harmonic oscillators")"#,
            false,
        )
        .await
        .unwrap();

        assert!(matches!(result, Response::Correct), "{:?}", result);
    }
}
