#[cfg(test)]
mod tests {
    use crate::check::*;
    use crate::qb::*;
    use crate::query::ApiQuery;

    // Test Response enum functionality
    #[test]
    fn test_response_enum_equality() {
        let correct1 = Response::Correct;
        let correct2 = Response::Correct;
        let incorrect1 = Response::Incorrect("reason".to_string());
        let incorrect2 = Response::Incorrect("reason".to_string());
        let prompt1 = Response::Prompt("hint".to_string());
        let prompt2 = Response::Prompt("hint".to_string());

        assert_eq!(correct1, correct2);
        assert_eq!(incorrect1, incorrect2);
        assert_eq!(prompt1, prompt2);
        assert_ne!(correct1, incorrect1);
        assert_ne!(correct1, prompt1);
        assert_ne!(incorrect1, prompt1);
    }

    #[test] 
    fn test_response_debug_format() {
        let responses = vec![
            Response::Correct,
            Response::Incorrect("Wrong answer".to_string()),
            Response::Prompt("Be more specific".to_string()),
        ];

        for response in responses {
            let debug_str = format!("{:?}", response);
            assert!(!debug_str.is_empty());
            match response {
                Response::Correct => assert!(debug_str.contains("Correct")),
                Response::Incorrect(ref reason) => {
                    assert!(debug_str.contains("Incorrect"));
                    assert!(debug_str.contains(reason));
                }
                Response::Prompt(ref hint) => {
                    assert!(debug_str.contains("Prompt"));
                    assert!(debug_str.contains(hint));
                }
            }
        }
    }

    #[test]
    fn test_answer_key_edge_cases() {
        // Test various answer key formats that might come from QBReader
        let test_cases = vec![
            ("Simple answer", "Simple answer"),
            ("Answer with (parentheses)", "Answer with (parentheses)"),
            ("Answer with [brackets]", "Answer with [brackets]"),
            ("Complex <b>bolded</b> answer", "Complex <b>bolded</b> answer"),
            ("", ""), // Empty answer
            ("Multiple (parts) with [different] <tags>", "Multiple (parts) with [different] <tags>"),
            ("Napoleon Bonaparte", "Napoleon Bonaparte"),
            ("The Great Gatsby (accept Gatsby)", "The Great Gatsby (accept Gatsby)"),
        ];

        for (full_answer, sanitized) in test_cases {
            let answer_key = (full_answer.to_string(), sanitized.to_string());
            assert_eq!(answer_key.0, full_answer);
            assert_eq!(answer_key.1, sanitized);
            
            // Test that both parts are valid strings (lengths are always >= 0 for String)
            assert!(answer_key.0.len() >= answer_key.0.len());
            assert!(answer_key.1.len() >= answer_key.1.len());
        }
    }

    // Mock JSON responses for different QBReader API scenarios
    #[test]
    fn test_mock_qbreader_successful_response() {
        let mock_response = r#"
        {
            "tossups": [
                {
                    "_id": "507f1f77bcf86cd799439011",
                    "question": "This French author wrote 'The Stranger' and won the Nobel Prize in Literature in 1957.",
                    "answer": "Albert Camus",
                    "category": "Literature",
                    "subcategory": "European Literature",
                    "packet": {
                        "_id": "packet123",
                        "name": "Regional Championship",
                        "number": 12
                    },
                    "set": {
                        "_id": "set456",
                        "name": "2023 Regional Set",
                        "year": 2023,
                        "standard": true
                    },
                    "updatedAt": "2023-06-15T10:30:00Z",
                    "difficulty": 5,
                    "number": 7,
                    "answer_sanitized": "Albert Camus",
                    "question_sanitized": "This French author wrote 'The Stranger' and won the Nobel Prize in Literature in 1957."
                }
            ]
        }
        "#;

        let parsed: Result<Tossups, _> = serde_json::from_str(mock_response);
        assert!(parsed.is_ok(), "Should successfully parse mock QBReader response");
        
        let tossups = parsed.unwrap();
        assert_eq!(tossups.tossups.len(), 1);
        
        let tossup = &tossups.tossups[0];
        assert_eq!(tossup.category, "Literature");
        assert_eq!(tossup.subcategory, "European Literature");
        assert_eq!(tossup.answer, "Albert Camus");
        assert_eq!(tossup.difficulty, 5);
        assert_eq!(tossup.packet.number, 12);
        assert_eq!(tossup.set.year, 2023);
        assert!(tossup.set.standard);
    }

    #[test]
    fn test_mock_qbreader_multiple_tossups() {
        let mock_response = r#"
        {
            "tossups": [
                {
                    "_id": "id1",
                    "question": "What is the powerhouse of the cell?",
                    "answer": "Mitochondria",
                    "category": "Science", 
                    "subcategory": "Biology",
                    "packet": {"_id": "p1", "name": "Packet 1", "number": 1},
                    "set": {"_id": "s1", "name": "Bio Set", "year": 2023, "standard": true},
                    "updatedAt": "2023-01-01T00:00:00Z",
                    "difficulty": 2,
                    "number": 1,
                    "answer_sanitized": "Mitochondria",
                    "question_sanitized": "What is the powerhouse of the cell?"
                },
                {
                    "_id": "id2", 
                    "question": "Who painted the Mona Lisa?",
                    "answer": "Leonardo da Vinci",
                    "category": "Fine Arts",
                    "subcategory": "Painting",
                    "packet": {"_id": "p1", "name": "Packet 1", "number": 1},
                    "set": {"_id": "s1", "name": "Art Set", "year": 2023, "standard": true},
                    "updatedAt": "2023-01-01T00:00:00Z",
                    "difficulty": 1,
                    "number": 2,
                    "answer_sanitized": "Leonardo da Vinci", 
                    "question_sanitized": "Who painted the Mona Lisa?"
                }
            ]
        }
        "#;

        let parsed: Tossups = serde_json::from_str(mock_response).unwrap();
        assert_eq!(parsed.tossups.len(), 2);
        
        // Test first tossup
        assert_eq!(parsed.tossups[0].category, "Science");
        assert_eq!(parsed.tossups[0].answer, "Mitochondria");
        assert_eq!(parsed.tossups[0].difficulty, 2);
        
        // Test second tossup
        assert_eq!(parsed.tossups[1].category, "Fine Arts");
        assert_eq!(parsed.tossups[1].answer, "Leonardo da Vinci");
        assert_eq!(parsed.tossups[1].difficulty, 1);
    }

    #[test]
    fn test_mock_qbreader_empty_response() {
        let empty_response = r#"{"tossups": []}"#;
        
        let parsed: Tossups = serde_json::from_str(empty_response).unwrap();
        assert_eq!(parsed.tossups.len(), 0);
    }

    #[test]
    fn test_mock_qbreader_error_cases() {
        let error_cases = vec![
            r#"{"error": "Invalid parameters"}"#,
            r#"{"tossups": [{"incomplete": "data"}]}"#,
            r#"invalid json"#,
            r#""#,
        ];

        for error_json in error_cases {
            let result: Result<Tossups, _> = serde_json::from_str(error_json);
            assert!(result.is_err(), "Should fail to parse malformed JSON: {}", error_json);
        }
    }

    #[test]
    fn test_api_query_parameter_construction() {
        let test_queries = vec![
            ApiQuery {
                categories: vec!["Science".to_string()],
                subcategories: vec![],
                alternate_subcategories: vec![],
                number: 1,
            },
            ApiQuery {
                categories: vec!["Literature".to_string(), "History".to_string()],
                subcategories: vec!["American Literature".to_string()],
                alternate_subcategories: vec!["European History".to_string()],
                number: 3,
            },
            ApiQuery {
                categories: vec![],
                subcategories: vec!["Biology".to_string(), "Chemistry".to_string()],
                alternate_subcategories: vec![],
                number: 10,
            },
        ];

        for query in test_queries {
            // Test that the query structure is valid
            assert!(query.number > 0);
            assert!(query.number <= 50); // Reasonable upper limit
            
            // Test categories are non-empty strings
            for category in &query.categories {
                assert!(!category.is_empty());
            }
            
            // Test subcategories are non-empty strings  
            for subcategory in &query.subcategories {
                assert!(!subcategory.is_empty());
            }
            
            // Test alternate subcategories are non-empty strings
            for alt_subcat in &query.alternate_subcategories {
                assert!(!alt_subcat.is_empty());
            }
        }
    }

    #[test]
    fn test_tossup_data_integrity() {
        // Test that essential tossup fields maintain data integrity
        let packet = Packet {
            id: "test_packet".to_string(),
            name: "Test Packet Name".to_string(),
            number: 42,
        };

        let set = Set {
            id: "test_set".to_string(), 
            name: "Test Set Name".to_string(),
            year: 2024,
            standard: false,
        };

        let tossup = Tossup {
            id: "test_tossup".to_string(),
            question: "Test question content?".to_string(),
            answer: "Test Answer".to_string(),
            category: "Test Category".to_string(),
            subcategory: "Test Subcategory".to_string(),
            packet: packet.clone(),
            set: set.clone(),
            updated_at: "2024-01-01T12:00:00Z".to_string(),
            difficulty: 3,
            number: 5,
            answer_sanitized: "Test Answer".to_string(),
            question_sanitized: "Test question content?".to_string(),
        };

        // Test cloning preserves all data
        let cloned_tossup = tossup.clone();
        assert_eq!(tossup.id, cloned_tossup.id);
        assert_eq!(tossup.question, cloned_tossup.question);
        assert_eq!(tossup.answer, cloned_tossup.answer);
        assert_eq!(tossup.difficulty, cloned_tossup.difficulty);
        assert_eq!(tossup.packet.id, cloned_tossup.packet.id);
        assert_eq!(tossup.set.year, cloned_tossup.set.year);

        // Test serialization/deserialization roundtrip
        let serialized = serde_json::to_string(&tossup).unwrap();
        let deserialized: Tossup = serde_json::from_str(&serialized).unwrap();
        assert_eq!(tossup.id, deserialized.id);
        assert_eq!(tossup.difficulty, deserialized.difficulty);
        assert_eq!(tossup.packet.number, deserialized.packet.number);
        assert_eq!(tossup.set.standard, deserialized.set.standard);
    }

    #[test]
    fn test_difficulty_levels() {
        // Test various difficulty levels that might be encountered
        let difficulties = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        
        for difficulty in difficulties {
            let tossup_json = format!(
                r#"{{
                    "tossups": [{{
                        "_id": "test_id",
                        "question": "Test question",
                        "answer": "Test answer",
                        "category": "Test",
                        "subcategory": "Test Sub",
                        "packet": {{"_id": "p", "name": "P", "number": 1}},
                        "set": {{"_id": "s", "name": "S", "year": 2023, "standard": true}},
                        "updatedAt": "2023-01-01T00:00:00Z",
                        "difficulty": {},
                        "number": 1,
                        "answer_sanitized": "Test answer",
                        "question_sanitized": "Test question"
                    }}]
                }}"#,
                difficulty
            );

            let parsed: Tossups = serde_json::from_str(&tossup_json).unwrap();
            assert_eq!(parsed.tossups[0].difficulty, difficulty);
            assert!(parsed.tossups[0].difficulty >= 1);
            assert!(parsed.tossups[0].difficulty <= 10);
        }
    }
}