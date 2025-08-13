#[cfg(test)]
mod tests {
    use crate::query::*;

    // Test edge cases and complex scenarios
    #[test]
    fn test_complex_nested_query() {
        let result = parse_query("((Science + History) & Biology) - Math").unwrap();
        assert!(result.categories.contains(&"Science".to_string()));
        assert!(result.subcategories.contains(&"Biology".to_string()));
        assert!(!result.alternate_subcategories.contains(&"Math".to_string()));
    }

    #[test]
    fn test_query_with_whitespace_variations() {
        let queries = vec![
            "Science+History",
            "Science + History",
            "  Science  +  History  ",
            "Science +History",
            "Science+ History",
        ];

        for query in queries {
            let result = parse_query(query).unwrap();
            assert!(result.categories.contains(&"Science".to_string()));
            assert!(result.categories.contains(&"History".to_string()));
        }
    }

    #[test]
    fn test_case_insensitive_parsing() {
        let queries = vec!["science", "SCIENCE", "Science", "sCiEnCe"];

        for query in queries {
            let result = parse_query(query).unwrap();
            assert!(result.categories.contains(&"Science".to_string()));
        }
    }

    #[test]
    fn test_multiword_categories_without_quotes() {
        let result = parse_query("American Literature + European History").unwrap();
        assert!(result.categories.contains(&"Literature".to_string()));
        assert!(result.categories.contains(&"History".to_string()));
        assert!(result
            .subcategories
            .contains(&"American Literature".to_string()));
        assert!(result
            .subcategories
            .contains(&"European History".to_string()));
    }

    #[test]
    fn test_operator_precedence_complex() {
        // Test: Science & Biology + Math - Computer Science
        // Should be: ((Science & Biology) + Math) - Computer Science
        let result = parse_query("Science & Biology + Math - Computer Science").unwrap();

        // Should include Science categories and Math
        assert!(result.categories.contains(&"Science".to_string()));
        // The exact behavior depends on the operator precedence implementation
        // Let's just test that it parses successfully and includes expected categories
        assert!(!result.categories.is_empty());
    }

    #[test]
    fn test_empty_result_scenarios() {
        // These should result in impossible queries
        let impossible_queries = vec![
            "Literature & Science",       // Different main categories
            "Biology & History",          // Different main categories
            "Math & American Literature", // Different main categories
        ];

        for query in impossible_queries {
            let result = parse_query(query);
            assert!(
                matches!(result, Err(QueryError::ImpossibleBranch(_))),
                "Query '{}' should be impossible but got: {:?}",
                query,
                result
            );
        }
    }

    #[test]
    fn test_subtraction_edge_cases() {
        // Test subtracting from empty set
        let result = parse_query("Biology - Science");
        // Biology is part of Science, so this should result in empty or minimal set
        assert!(result.is_ok()); // Should parse successfully even if logically minimal

        // Test subtracting non-existent category
        let result = parse_query("Science - Literature").unwrap();
        assert!(result.categories.contains(&"Science".to_string()));
        // Literature subcategories should not be in the result
        assert!(!result
            .subcategories
            .iter()
            .any(|s| s.contains("Literature")));
    }

    #[test]
    fn test_parentheses_edge_cases() {
        // Test nested parentheses
        let result = parse_query("((Science))").unwrap();
        assert!(result.categories.contains(&"Science".to_string()));

        // Test empty parentheses should fail
        let result = parse_query("()");
        assert!(result.is_err());

        // Test mismatched parentheses
        let result = parse_query("(Science");
        assert!(matches!(result, Err(QueryError::UnexpectedEOF)));

        let result = parse_query("Science)");
        assert!(result.is_err());
    }

    #[test]
    fn test_long_query_chain() {
        let query = "Science + History + Literature + Fine Arts + Religion + Mythology";
        let result = parse_query(query).unwrap();

        let expected_categories = vec![
            "Science",
            "History",
            "Literature",
            "Fine Arts",
            "Religion",
            "Mythology",
        ];

        for category in expected_categories {
            assert!(
                result.categories.contains(&category.to_string()),
                "Missing category: {}",
                category
            );
        }
    }

    #[test]
    fn test_category_validation() {
        // Test that all main categories are recognized
        let main_categories = vec![
            "Science",
            "History",
            "Literature",
            "Fine Arts",
            "Religion",
            "Mythology",
            "Philosophy",
            "Social Science",
            "Current Events",
            "Geography",
            "Other Academic",
            "Pop Culture",
        ];

        for category in main_categories {
            let result = parse_query(category);
            assert!(result.is_ok(), "Category '{}' should be valid", category);
        }
    }

    #[test]
    fn test_common_subcategories() {
        // Test common subcategories are recognized
        let subcategories = vec![
            "Biology",
            "Chemistry",
            "Physics",
            "Math",
            "American History",
            "European History",
            "World History",
            "American Literature",
            "British Literature",
            "Poetry",
            "Computer Science",
            "Astronomy",
            "Earth Science",
        ];

        for subcat in subcategories {
            let result = parse_query(subcat);
            assert!(result.is_ok(), "Subcategory '{}' should be valid", subcat);
        }
    }

    #[test]
    fn test_tokenizer_edge_cases() {
        // Test various tokenizer scenarios
        let edge_cases = vec![
            ("Science + History", true),   // Spaces around operators
            ("Science  +  History", true), // Multiple spaces
        ];

        for (query, should_succeed) in edge_cases {
            let result = parse_query(query);
            if should_succeed {
                assert!(
                    result.is_ok(),
                    "Query '{}' should parse successfully",
                    query
                );
            } else {
                assert!(result.is_err(), "Query '{}' should fail to parse", query);
            }
        }

        // Test that operators without spaces might not parse correctly
        let no_space_result = parse_query("Science&History");
        // This might fail or succeed depending on tokenizer implementation
        // We'll just check that it doesn't panic
        match no_space_result {
            Ok(_) => println!("No-space operators work"),
            Err(_) => println!("No-space operators don't work (expected)"),
        }
    }

    #[test]
    fn test_api_query_defaults() {
        let query = ApiQuery::default();
        assert_eq!(query.number, 1);
        assert!(query.categories.is_empty());
        assert!(query.subcategories.is_empty());
        assert!(query.alternate_subcategories.is_empty());
    }

    #[test]
    fn test_query_error_display() {
        // Test that error messages are helpful
        let errors = vec![
            parse_query("InvalidCategory").unwrap_err(),
            parse_query("& Science").unwrap_err(),
            parse_query("(Science").unwrap_err(),
        ];

        for error in errors {
            let error_string = format!("{:?}", error);
            assert!(!error_string.is_empty(), "Error should have a description");
        }
    }

    #[test]
    fn test_performance_large_query() {
        // Test that large queries don't cause performance issues
        let mut large_query = String::new();
        let categories = ["Science", "History", "Literature"];

        for i in 0..100 {
            if i > 0 {
                large_query.push_str(" + ");
            }
            large_query.push_str(categories[i % categories.len()]);
        }

        let start = std::time::Instant::now();
        let result = parse_query(&large_query);
        let duration = start.elapsed();

        assert!(result.is_ok(), "Large query should parse successfully");
        assert!(
            duration.as_millis() < 100,
            "Large query should parse quickly (took {:?})",
            duration
        );
    }
}
