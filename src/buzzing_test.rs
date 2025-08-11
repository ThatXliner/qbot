#[cfg(test)]
mod tests {
    use crate::{Data, QuestionState};
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use serenity::all::{ChannelId, UserId};

    fn create_test_data() -> Data {
        Data {
            reqwest: reqwest::Client::new(),
            reading_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tokio::test]
    async fn test_reading_state_creation() {
        let data = create_test_data();
        let channel_id = ChannelId::new(123456789);
        
        // Insert a reading state
        {
            let mut states = data.reading_states.lock().await;
            states.insert(
                channel_id,
                (QuestionState::Reading, true, HashSet::new()),
            );
        }

        // Verify state exists
        {
            let states = data.reading_states.lock().await;
            assert!(states.contains_key(&channel_id));
            if let Some((state, power, _)) = states.get(&channel_id) {
                assert!(matches!(state, QuestionState::Reading));
                assert_eq!(*power, true);
            }
        }
    }

    #[tokio::test]
    async fn test_buzz_state_transition() {
        let data = create_test_data();
        let channel_id = ChannelId::new(123456789);
        let user_id = UserId::new(987654321);
        let timestamp = 1234567890;

        // Start with reading state
        {
            let mut states = data.reading_states.lock().await;
            states.insert(
                channel_id,
                (QuestionState::Reading, true, HashSet::new()),
            );
        }

        // Transition to buzzed state
        {
            let mut states = data.reading_states.lock().await;
            if let Some(state) = states.get_mut(&channel_id) {
                state.0 = QuestionState::Buzzed(user_id, timestamp);
            }
        }

        // Verify transition
        {
            let states = data.reading_states.lock().await;
            if let Some((QuestionState::Buzzed(buzz_user, buzz_timestamp), _, _)) = states.get(&channel_id) {
                assert_eq!(*buzz_user, user_id);
                assert_eq!(*buzz_timestamp, timestamp);
            } else {
                panic!("Expected Buzzed state");
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_state_access() {
        let data = Arc::new(create_test_data());
        let channel_id = ChannelId::new(123456789);
        
        // Insert initial state
        {
            let mut states = data.reading_states.lock().await;
            states.insert(
                channel_id,
                (QuestionState::Reading, true, HashSet::new()),
            );
        }

        // Spawn multiple tasks that access the state concurrently
        let handles: Vec<_> = (0..10).map(|i| {
            let data_clone = data.clone();
            tokio::spawn(async move {
                for _ in 0..100 {
                    // Read access
                    {
                        let states = data_clone.reading_states.lock().await;
                        let _ = states.get(&channel_id);
                    }
                    
                    // Write access
                    {
                        let mut states = data_clone.reading_states.lock().await;
                        if let Some(state) = states.get_mut(&channel_id) {
                            // Simulate state transitions
                            match i % 3 {
                                0 => state.0 = QuestionState::Reading,
                                1 => state.0 = QuestionState::Buzzed(UserId::new(i as u64), 12345),
                                _ => state.0 = QuestionState::Invalid(UserId::new(i as u64)),
                            }
                        }
                    }
                    
                    // Small delay to allow other tasks to run
                    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                }
            })
        }).collect();

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Task should complete successfully");
        }

        // Verify state still exists and is accessible
        {
            let states = data.reading_states.lock().await;
            assert!(states.contains_key(&channel_id));
        }
    }

    #[tokio::test]
    async fn test_immediate_state_transition_response() {
        let data = create_test_data();
        let channel_id = ChannelId::new(123456789);
        let user_id = UserId::new(987654321);
        let timestamp = 1234567890;

        // Start with buzzed state
        {
            let mut states = data.reading_states.lock().await;
            states.insert(
                channel_id,
                (QuestionState::Buzzed(user_id, timestamp), true, HashSet::new()),
            );
        }

        // Simulate immediate state transition to Correct (like user answered immediately)
        let data_arc = Arc::new(data);
        let data_clone = data_arc.clone();
        
        // Spawn a task that will change the state after a short delay
        let transition_task = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            let mut states = data_clone.reading_states.lock().await;
            if let Some(state) = states.get_mut(&channel_id) {
                state.0 = QuestionState::Correct;
            }
        });

        // Simulate the buzz handling logic
        let start_time = std::time::Instant::now();
        let timeout_duration = tokio::time::Duration::from_secs(10);
        
        loop {
            // Sleep for a short interval to check frequently for state changes
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            // Check if state has changed or timeout reached
            let current_state = {
                let states = data_arc.reading_states.lock().await;
                match states.get(&channel_id) {
                    Some(state) => state.clone(),
                    None => break,
                }
            };
            
            // If state is no longer Buzzed, break to handle the new state
            if !matches!(current_state.0, QuestionState::Buzzed(_, _)) {
                break;
            }
            
            // Check if timeout has been reached
            if start_time.elapsed() >= timeout_duration {
                // This should not happen in this test
                panic!("Timeout reached before state transition");
            }
        }

        // Verify that we detected the state change quickly (should be around 200ms, not 10 seconds)
        let elapsed = start_time.elapsed();
        assert!(elapsed < tokio::time::Duration::from_secs(1), "Should respond within 1 second, took {:?}", elapsed);
        
        // Verify the state transitioned correctly
        {
            let states = data_arc.reading_states.lock().await;
            if let Some((QuestionState::Correct, _, _)) = states.get(&channel_id) {
                // Expected
            } else {
                panic!("Expected Correct state");
            }
        }

        // Clean up the transition task
        transition_task.await.expect("Transition task should complete");
    }
}