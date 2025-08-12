#[cfg(test)]
mod tests {
    use crate::query::*;

    fn q(s: &str) -> Result<ApiQuery, QueryError> {
        parse_query(s)
    }

    #[test]
    fn single_category() {
        let r = q("Science").unwrap();
        assert!(r.categories.contains(&"Science".to_string()));
        assert!(r.subcategories.contains(&"Biology".to_string()));
    }

    #[test]
    fn single_subcategory() {
        let r = q("Biology").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert_eq!(r.subcategories, vec!["Biology"]);
    }

    #[test]
    fn alternate_subcategory() {
        let r = q("Math").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert_eq!(r.subcategories, vec!["Other Science"]);
        assert_eq!(r.alternate_subcategories, vec!["Math"]);
    }

    #[test]
    fn multi_word_category() {
        let r = q("American Literature").unwrap();
        assert_eq!(r.categories, vec!["Literature"]);
        assert_eq!(r.subcategories, vec!["American Literature"]);
    }

    #[test]
    fn and_operator_same_category() {
        let r = q("Biology & Chemistry").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert!(r.subcategories.contains(&"Biology".to_string()), "{:?}", r);
        assert!(r.subcategories.contains(&"Chemistry".to_string()));
    }

    #[test]
    fn and_operator_different_category_impossible() {
        let r = q("Biology & History");
        assert!(matches!(r, Err(QueryError::ImpossibleBranch(_))));
    }

    #[test]
    fn or_operator() {
        let r = q("Biology + History").unwrap();
        assert!(r.categories.contains(&"Science".to_string()));
        assert!(r.categories.contains(&"History".to_string()));
    }

    #[test]
    fn not_operator_removes_subcategory() {
        let r = q("Science - Math").unwrap();
        assert!(r.categories.contains(&"Science".to_string()), "{:?}", r);
        assert!(!r.alternate_subcategories.contains(&"Math".to_string()));
    }

    #[test]
    fn parentheses_override_precedence() {
        let r = q("Science & (Biology + Chemistry)").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert!(r.subcategories.contains(&"Biology".to_string()));
        assert!(r.subcategories.contains(&"Chemistry".to_string()));
    }

    #[test]
    fn unexpected_token_error() {
        let r = q("& Science");
        assert!(matches!(r, Err(QueryError::UnexpectedToken(_))));
    }

    #[test]
    fn unexpected_eof_error() {
        let mut tokens = tokenize("(");
        let r = parse_expr(&mut tokens);
        assert!(matches!(r, Err(QueryError::UnexpectedEOF)));
    }

    #[test]
    fn invalid_category_error() {
        let r = q("MadeUpCategory");
        assert!(matches!(r, Err(QueryError::InvalidCategory(_))));
    }

    #[test]
    fn lowercase_and_spacing() {
        let r = q("  biology  +   history  ").unwrap();
        assert!(r.categories.contains(&"Science".to_string()));
        assert!(r.categories.contains(&"History".to_string()));
    }

    #[test]
    fn default_number_is_one() {
        let r = q("Biology").unwrap();
        assert_eq!(r.number, 1);
    }

    #[test]
    fn categories_contain_expected_keys() {
        // Test that main categories exist
        assert!(CATEGORIES.contains_key("Science"));
        assert!(CATEGORIES.contains_key("History"));
        assert!(CATEGORIES.contains_key("Literature"));
        assert!(CATEGORIES.contains_key("Fine Arts"));
    }

    #[test]
    fn science_has_expected_subcategories() {
        let (subcategories, alternate_subcategories) = CATEGORIES.get("Science").unwrap();

        // Test regular subcategories
        assert!(subcategories.contains(&"Biology"));
        assert!(subcategories.contains(&"Chemistry"));
        assert!(subcategories.contains(&"Physics"));

        // Test alternate subcategories
        assert!(alternate_subcategories.contains(&"Math"));
        assert!(alternate_subcategories.contains(&"Computer Science"));
    }

    #[test]
    fn case_insensitive_category_lookup() {
        // This simulates the case-insensitive search in the categories command
        let search_term = "science";
        let found = CATEGORIES
            .keys()
            .find(|&key| key.to_lowercase() == search_term.to_lowercase());

        assert!(found.is_some());
        assert_eq!(found.unwrap(), &"Science");
    }
}
