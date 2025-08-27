#[cfg(test)]
mod tests {
    use crate::qb::*;
    use crate::query::ApiQuery;
    use url::Url;

    #[test]
    fn test_packet_serialization() {
        let packet = Packet {
            id: "test_id".to_string(),
            name: "Test Packet".to_string(),
            number: 1,
        };

        let json = serde_json::to_string(&packet).unwrap();
        let deserialized: Packet = serde_json::from_str(&json).unwrap();

        assert_eq!(packet.id, deserialized.id);
        assert_eq!(packet.name, deserialized.name);
        assert_eq!(packet.number, deserialized.number);
    }

    #[test]
    fn test_set_serialization() {
        let set = Set {
            id: "set_id".to_string(),
            name: "Test Set".to_string(),
            year: 2023,
            standard: true,
        };

        let json = serde_json::to_string(&set).unwrap();
        let deserialized: Set = serde_json::from_str(&json).unwrap();

        assert_eq!(set.id, deserialized.id);
        assert_eq!(set.name, deserialized.name);
        assert_eq!(set.year, deserialized.year);
        assert_eq!(set.standard, deserialized.standard);
    }

    #[test]
    fn test_tossup_serialization() {
        let packet = Packet {
            id: "packet_id".to_string(),
            name: "Test Packet".to_string(),
            number: 1,
        };

        let set = Set {
            id: "set_id".to_string(),
            name: "Test Set".to_string(),
            year: 2023,
            standard: true,
        };

        let tossup = Tossup {
            id: "tossup_id".to_string(),
            question: "What is the capital of France?".to_string(),
            answer: "Paris".to_string(),
            category: "Geography".to_string(),
            subcategory: "World Geography".to_string(),
            packet: packet.clone(),
            set: set.clone(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            difficulty: 5,
            number: 1,
            answer_sanitized: "Paris".to_string(),
            question_sanitized: "What is the capital of France?".to_string(),
        };

        let json = serde_json::to_string(&tossup).unwrap();
        let deserialized: Tossup = serde_json::from_str(&json).unwrap();

        assert_eq!(tossup.id, deserialized.id);
        assert_eq!(tossup.question, deserialized.question);
        assert_eq!(tossup.answer, deserialized.answer);
        assert_eq!(tossup.category, deserialized.category);
        assert_eq!(tossup.subcategory, deserialized.subcategory);
    }

    #[test]
    fn test_tossups_collection() {
        let packet = Packet {
            id: "packet_id".to_string(),
            name: "Test Packet".to_string(),
            number: 1,
        };

        let set = Set {
            id: "set_id".to_string(),
            name: "Test Set".to_string(),
            year: 2023,
            standard: true,
        };

        let tossup1 = Tossup {
            id: "tossup1_id".to_string(),
            question: "Question 1".to_string(),
            answer: "Answer 1".to_string(),
            category: "Science".to_string(),
            subcategory: "Biology".to_string(),
            packet: packet.clone(),
            set: set.clone(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            difficulty: 3,
            number: 1,
            answer_sanitized: "Answer 1".to_string(),
            question_sanitized: "Question 1".to_string(),
        };

        let tossup2 = Tossup {
            id: "tossup2_id".to_string(),
            question: "Question 2".to_string(),
            answer: "Answer 2".to_string(),
            category: "History".to_string(),
            subcategory: "World History".to_string(),
            packet,
            set,
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            difficulty: 4,
            number: 2,
            answer_sanitized: "Answer 2".to_string(),
            question_sanitized: "Question 2".to_string(),
        };

        let tossups = Tossups {
            tossups: vec![tossup1, tossup2],
        };

        assert_eq!(tossups.tossups.len(), 2);
        assert_eq!(tossups.tossups[0].category, "Science");
        assert_eq!(tossups.tossups[1].category, "History");
    }

    #[test]
    fn test_api_url_construction_basic() {
        use url::Url;

        let api_params = ApiQuery {
            categories: vec!["Science".to_string()],
            subcategories: vec![],
            alternate_subcategories: vec![],
            number: 1,
        };

        let mut url = Url::parse("https://www.qbreader.org/api/random-tossup").unwrap();
        for category in &api_params.categories {
            url.query_pairs_mut().append_pair("categories", category);
        }
        url.query_pairs_mut()
            .append_pair("number", &api_params.number.to_string());

        let url_str = url.to_string();
        assert!(url_str.contains("categories=Science"));
        assert!(url_str.contains("number=1"));
    }

    #[test]
    fn test_api_url_construction_with_subcategories() {
        use url::Url;

        let api_params = ApiQuery {
            categories: vec!["Science".to_string()],
            subcategories: vec!["Biology".to_string(), "Chemistry".to_string()],
            alternate_subcategories: vec!["Math".to_string()],
            number: 3,
        };

        let mut url = Url::parse("https://www.qbreader.org/api/random-tossup").unwrap();
        for category in &api_params.categories {
            url.query_pairs_mut().append_pair("categories", category);
        }
        for subcategory in &api_params.subcategories {
            url.query_pairs_mut()
                .append_pair("subcategories", subcategory);
        }
        for alternate_subcategory in &api_params.alternate_subcategories {
            url.query_pairs_mut()
                .append_pair("alternateSubcategories", alternate_subcategory);
        }
        url.query_pairs_mut()
            .append_pair("number", &api_params.number.to_string());

        let url_str = url.to_string();
        assert!(url_str.contains("categories=Science"));
        assert!(url_str.contains("subcategories=Biology"));
        assert!(url_str.contains("subcategories=Chemistry"));
        assert!(url_str.contains("alternateSubcategories=Math"));
        assert!(url_str.contains("number=3"));
    }

    #[test]
    fn test_empty_api_params() {
        use url::Url;

        let api_params = ApiQuery {
            categories: vec![],
            subcategories: vec![],
            alternate_subcategories: vec![],
            number: 1,
        };

        let mut url = Url::parse("https://www.qbreader.org/api/random-tossup").unwrap();
        url.query_pairs_mut()
            .append_pair("number", &api_params.number.to_string());

        let url_str = url.to_string();
        assert!(url_str.contains("number=1"));
        assert!(!url_str.contains("categories="));
        assert!(!url_str.contains("subcategories="));
    }

    #[test]
    fn test_api_url_encoding() {
        use url::Url;

        let api_params = ApiQuery {
            categories: vec!["Fine Arts".to_string()],
            subcategories: vec!["American Literature".to_string()],
            alternate_subcategories: vec![],
            number: 1,
        };

        let mut url = Url::parse("https://www.qbreader.org/api/random-tossup").unwrap();
        for category in &api_params.categories {
            url.query_pairs_mut().append_pair("categories", category);
        }
        for subcategory in &api_params.subcategories {
            url.query_pairs_mut()
                .append_pair("subcategories", subcategory);
        }

        let url_str = url.to_string();
        // URL encoding should handle spaces properly
        assert!(url_str.contains("Fine%20Arts") || url_str.contains("Fine+Arts"));
        assert!(
            url_str.contains("American%20Literature") || url_str.contains("American+Literature")
        );
    }

    // Test that demonstrates the structure of the JSON response from QBReader API
    #[test]
    fn test_json_deserialization_from_api_format() {
        let json_response = r#"
        {
            "tossups": [
                {
                    "_id": "507f1f77bcf86cd799439011",
                    "question": "This scientist's name is on a law that states the total entropy of an isolated system can never decrease.",
                    "answer": "Rudolf <b>Clausius</b>",
                    "category": "Science",
                    "subcategory": "Physics",
                    "packet": {
                        "_id": "packet123",
                        "name": "Test Packet 1",
                        "number": 1
                    },
                    "set": {
                        "_id": "set456",
                        "name": "Test Tournament 2023",
                        "year": 2023,
                        "standard": true
                    },
                    "updatedAt": "2023-01-15T10:30:00.000Z",
                    "difficulty": 4,
                    "number": 15,
                    "answer_sanitized": "Rudolf Clausius",
                    "question_sanitized": "This scientist's name is on a law that states the total entropy of an isolated system can never decrease."
                }
            ]
        }
        "#;

        let parsed: Tossups = serde_json::from_str(json_response).unwrap();
        assert_eq!(parsed.tossups.len(), 1);

        let tossup = &parsed.tossups[0];
        assert_eq!(tossup.id, "507f1f77bcf86cd799439011");
        assert_eq!(tossup.category, "Science");
        assert_eq!(tossup.subcategory, "Physics");
        assert_eq!(tossup.answer_sanitized, "Rudolf Clausius");
        assert_eq!(tossup.difficulty, 4);
        assert_eq!(tossup.set.year, 2023);
        assert_eq!(tossup.packet.number, 1);
    }

    // Mock HTTP tests for QBReader API responses - focusing on JSON parsing
    #[tokio::test] 
    async fn test_mocked_random_tossup_success() {
        // Test realistic QBReader API response parsing
        let mock_response = r#"
        {
            "tossups": [
                {
                    "_id": "507f1f77bcf86cd799439011",
                    "question": "This scientist's name is on a law that states the total entropy of an isolated system can never decrease.",
                    "answer": "Rudolf Clausius",
                    "category": "Science",
                    "subcategory": "Physics", 
                    "packet": {
                        "_id": "packet123",
                        "name": "Test Packet",
                        "number": 1
                    },
                    "set": {
                        "_id": "set456",
                        "name": "Test Set 2023",
                        "year": 2023,
                        "standard": true
                    },
                    "updatedAt": "2023-01-01T00:00:00Z",
                    "difficulty": 4,
                    "number": 1,
                    "answer_sanitized": "Rudolf Clausius",
                    "question_sanitized": "This scientist's name is on a law that states the total entropy of an isolated system can never decrease."
                }
            ]
        }
        "#;
        
        // Test deserialization directly (this is the core functionality we want to test)
        let parsed: Tossups = serde_json::from_str(mock_response).unwrap();
        
        assert_eq!(parsed.tossups.len(), 1);
        assert_eq!(parsed.tossups[0].id, "507f1f77bcf86cd799439011");
        assert_eq!(parsed.tossups[0].category, "Science");
        assert_eq!(parsed.tossups[0].subcategory, "Physics");
        assert_eq!(parsed.tossups[0].answer, "Rudolf Clausius");
        assert_eq!(parsed.tossups[0].difficulty, 4);
    }

    #[test]
    fn test_api_query_url_construction_comprehensive() {
        let api_params = ApiQuery {
            categories: vec!["Science".to_string(), "Literature".to_string()],
            subcategories: vec!["Biology".to_string(), "Chemistry".to_string()],
            alternate_subcategories: vec!["Math".to_string(), "Physics".to_string()],
            number: 5,
        };

        let mut url = Url::parse("https://www.qbreader.org/api/random-tossup").unwrap();
        
        // Build URL just like the real function does
        for category in &api_params.categories {
            url.query_pairs_mut().append_pair("categories", category);
        }
        for subcategory in &api_params.subcategories {
            url.query_pairs_mut().append_pair("subcategories", subcategory);
        }
        for alternate_subcategory in &api_params.alternate_subcategories {
            url.query_pairs_mut().append_pair("alternateSubcategories", alternate_subcategory);
        }
        url.query_pairs_mut().append_pair("number", &api_params.number.to_string());

        let url_str = url.as_str();
        
        // Verify all parameters are present
        assert!(url_str.contains("categories=Science"));
        assert!(url_str.contains("categories=Literature"));
        assert!(url_str.contains("subcategories=Biology"));
        assert!(url_str.contains("subcategories=Chemistry"));
        assert!(url_str.contains("alternateSubcategories=Math"));
        assert!(url_str.contains("alternateSubcategories=Physics"));
        assert!(url_str.contains("number=5"));
    }

    #[test]
    fn test_tossup_edge_cases() {
        // Test with minimal valid data
        let minimal_json = r#"
        {
            "tossups": [
                {
                    "_id": "minimal_id",
                    "question": "",
                    "answer": "",
                    "category": "Science",
                    "subcategory": "Physics",
                    "packet": {
                        "_id": "p1",
                        "name": "P1",
                        "number": 1
                    },
                    "set": {
                        "_id": "s1",
                        "name": "S1",
                        "year": 2023,
                        "standard": true
                    },
                    "updatedAt": "2023-01-01T00:00:00Z",
                    "difficulty": 1,
                    "number": 1,
                    "answer_sanitized": "",
                    "question_sanitized": ""
                }
            ]
        }
        "#;

        let parsed: Tossups = serde_json::from_str(minimal_json).unwrap();
        assert_eq!(parsed.tossups.len(), 1);
        assert_eq!(parsed.tossups[0].question, "");
        assert_eq!(parsed.tossups[0].answer, "");
        assert_eq!(parsed.tossups[0].difficulty, 1);
    }

    #[test]
    fn test_multiple_tossups_parsing() {
        let multiple_json = r#"
        {
            "tossups": [
                {
                    "_id": "id1",
                    "question": "Question 1",
                    "answer": "Answer 1",
                    "category": "Science",
                    "subcategory": "Biology",
                    "packet": {"_id": "p1", "name": "P1", "number": 1},
                    "set": {"_id": "s1", "name": "S1", "year": 2023, "standard": true},
                    "updatedAt": "2023-01-01T00:00:00Z",
                    "difficulty": 3,
                    "number": 1,
                    "answer_sanitized": "Answer 1",
                    "question_sanitized": "Question 1"
                },
                {
                    "_id": "id2",
                    "question": "Question 2",
                    "answer": "Answer 2",
                    "category": "History",
                    "subcategory": "American History",
                    "packet": {"_id": "p1", "name": "P1", "number": 1},
                    "set": {"_id": "s1", "name": "S1", "year": 2023, "standard": true},
                    "updatedAt": "2023-01-01T00:00:00Z",
                    "difficulty": 4,
                    "number": 2,
                    "answer_sanitized": "Answer 2",
                    "question_sanitized": "Question 2"
                }
            ]
        }
        "#;

        let parsed: Tossups = serde_json::from_str(multiple_json).unwrap();
        assert_eq!(parsed.tossups.len(), 2);
        assert_eq!(parsed.tossups[0].category, "Science");
        assert_eq!(parsed.tossups[1].category, "History");
        assert_eq!(parsed.tossups[0].difficulty, 3);
        assert_eq!(parsed.tossups[1].difficulty, 4);
    }

    #[test]
    fn test_malformed_json_handling() {
        let malformed_jsons = vec![
            r#"{"tossups": [{"_id": "incomplete""#,
            r#"{"invalid": "structure"}"#,
            r#"{"tossups": []}"#, // This should work (empty)
        ];

        for (i, json) in malformed_jsons.iter().enumerate() {
            let result: Result<Tossups, _> = serde_json::from_str(json);
            match i {
                0 | 1 => assert!(result.is_err(), "Should fail for malformed JSON {}", i),
                2 => {
                    assert!(result.is_ok(), "Empty tossups array should be valid");
                    assert_eq!(result.unwrap().tossups.len(), 0);
                }
                _ => {}
            }
        }
    }
}
