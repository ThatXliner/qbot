use rapidfuzz::distance::levenshtein;
use tracing::info;

use crate::Tossup;

// intentionally we have a different threshold for fuzzy matching
// compared the query.rs
const FUZZY_THRESHOLD: usize = 15;
// TODO: add prompt
pub fn check_correct_answer(question: Tossup, answer: &str) -> bool {
    // Implement your logic here to check if the answer is correct for the given question
    // For example:
    // question == "What is the capital of France?" && answer == "Paris"
    // question == "What is the square root of 16?" && answer == "4"
    // ...
    info!("Checking answer for question: {}", question.question);
    info!("Answer: {}", question.answer_sanitized);
    info!("User answer: {}", answer);
    levenshtein::distance(question.answer_sanitized.chars(), answer.chars()) < FUZZY_THRESHOLD
}
