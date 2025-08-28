use llm::{
    builder::{LLMBackend, LLMBuilder},
    LLMProvider,
};

use crate::check::healthcheck;

/// In case we send this to an LLM
pub fn render_html(answer: &str) -> String {
    answer
        .replace("<b>", "**")
        .replace("</b>", "**")
        .replace("<i>", "_")
        .replace("</i>", "_")
        .replace("<u>", "__")
        .replace("</u>", "__")
}
pub fn format_question(question: &str) -> String {
    question.replace("*", "\\*")
    // .replace("<b>", "**")
    // .replace("</b>", "**")
    // .replace("<i>", "_")
    // .replace("</i>", "_")
    // .replace("(*)", ":star:")
}
pub fn nth_chunk<I: Iterator>(mut iter: I, n: usize) -> Vec<I::Item> {
    iter.by_ref().take(n).collect()
}
pub async fn get_llm(reqwest: &reqwest::Client) -> Box<dyn LLMProvider> {
    let gemini_api_key = std::env::var("GEMINI_API_KEY").ok();
    let ollama_base_url = std::env::var("OLLAMA_URL").unwrap_or("http://127.0.0.1:11434".into());
    if let Some(api_key) = gemini_api_key {
        LLMBuilder::new()
            .backend(LLMBackend::Google) // Use Google as the LLM backend
            .api_key(api_key)
            .model("gemini-2.5-flash")
            .max_tokens(1000) // Set maximum response length
            .temperature(0.7) // Control response randomness (0.0-1.0)
            .stream(false) // Disable streaming responses
            .build()
            .expect("Failed to build LLM (Google)")
    } else {
        if !healthcheck(&reqwest, &ollama_base_url).await {
            panic!("Ollama is not running");
        }
        LLMBuilder::new()
            .backend(LLMBackend::Ollama) // Use Ollama as the LLM backend
            .base_url(&ollama_base_url) // Set the Ollama server URL
            .model("qwen3:1.7b")
            .max_tokens(1000) // Set maximum response length
            .temperature(0.7) // Control response randomness (0.0-1.0)
            .stream(false) // Disable streaming responses
            .build()
            .expect("Failed to build LLM (Ollama)")
    }
}
pub fn get_llm_no_healthcheck() -> Box<dyn LLMProvider> {
    let gemini_api_key = std::env::var("GEMINI_API_KEY").ok();
    let ollama_base_url = std::env::var("OLLAMA_URL").unwrap_or("http://127.0.0.1:11434".into());
    if let Some(api_key) = gemini_api_key {
        LLMBuilder::new()
            .backend(LLMBackend::Google) // Use Google as the LLM backend
            .api_key(api_key)
            .model("gemini-2.5-flash")
            .max_tokens(1000) // Set maximum response length
            .temperature(0.7) // Control response randomness (0.0-1.0)
            .stream(false) // Disable streaming responses
            .build()
            .expect("Failed to build LLM (Google)")
    } else {
        LLMBuilder::new()
            .backend(LLMBackend::Ollama) // Use Ollama as the LLM backend
            .base_url(&ollama_base_url) // Set the Ollama server URL
            .model("qwen3:1.7b")
            .max_tokens(1000) // Set maximum response length
            .temperature(0.7) // Control response randomness (0.0-1.0)
            .stream(false) // Disable streaming responses
            .build()
            .expect("Failed to build LLM (Ollama)")
    }
}
