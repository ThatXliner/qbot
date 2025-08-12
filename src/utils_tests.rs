#[cfg(test)]
mod tests {
    use crate::utils::*;

    #[test]
    fn test_render_html_bold() {
        assert_eq!(
            render_html("This is <b>bold</b> text"),
            "This is **bold** text"
        );
    }

    #[test]
    fn test_render_html_italic() {
        assert_eq!(
            render_html("This is <i>italic</i> text"),
            "This is _italic_ text"
        );
    }

    #[test]
    fn test_render_html_underline() {
        assert_eq!(
            render_html("This is <u>underlined</u> text"),
            "This is __underlined__ text"
        );
    }

    #[test]
    fn test_render_html_mixed_formatting() {
        let input = "This has <b>bold</b>, <i>italic</i>, and <u>underlined</u> text";
        let expected = "This has **bold**, _italic_, and __underlined__ text";
        assert_eq!(render_html(input), expected);
    }

    #[test]
    fn test_render_html_nested_tags() {
        let input = "This is <b><i>bold and italic</i></b>";
        let expected = "This is **_bold and italic_**";
        assert_eq!(render_html(input), expected);
    }

    #[test]
    fn test_render_html_no_tags() {
        let input = "This is plain text";
        assert_eq!(render_html(input), input);
    }

    #[test]
    fn test_format_question_escapes_asterisks() {
        assert_eq!(
            format_question("This has * asterisks *"),
            "This has \\* asterisks \\*"
        );
    }

    #[test]
    fn test_format_question_no_asterisks() {
        let input = "This has no asterisks";
        assert_eq!(format_question(input), input);
    }

    #[test]
    fn test_format_question_multiple_asterisks() {
        assert_eq!(
            format_question("*** multiple ***"),
            "\\*\\*\\* multiple \\*\\*\\*"
        );
    }

    #[test]
    fn test_nth_chunk_basic() {
        let vec = vec![1, 2, 3, 4, 5];
        let result = nth_chunk(vec.into_iter(), 3);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_nth_chunk_empty_iterator() {
        let vec: Vec<i32> = vec![];
        let result = nth_chunk(vec.into_iter(), 3);
        let expected: Vec<i32> = vec![];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_nth_chunk_more_than_available() {
        let vec = vec![1, 2];
        let result = nth_chunk(vec.into_iter(), 5);
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn test_nth_chunk_zero_items() {
        let vec = vec![1, 2, 3];
        let result = nth_chunk(vec.into_iter(), 0);
        let expected: Vec<i32> = vec![];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_nth_chunk_strings() {
        let words = vec!["hello", "world", "test"];
        let result = nth_chunk(words.into_iter(), 2);
        assert_eq!(result, vec!["hello", "world"]);
    }
}
