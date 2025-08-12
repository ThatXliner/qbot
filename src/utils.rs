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
