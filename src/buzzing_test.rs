#[cfg(test)]
mod tests {
    use crate::{Data, QuestionState};
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use tokio::sync::{Mutex, watch};
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
            let (tx, _rx) = watch::channel(());
            states.insert(
                channel_id,
                (QuestionState::Reading, true, HashSet::new(), tx),
            );
        }

        // Verify state exists
        {
            let states = data.reading_states.lock().await;
            assert!(states.contains_key(&channel_id));
            if let Some((state, power, _, _)) = states.get(&channel_id) {
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
            let (tx, _rx) = watch::channel(());
            states.insert(
                channel_id,
                (QuestionState::Reading, true, HashSet::new(), tx),
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
            if let Some((QuestionState::Buzzed(buzz_user, buzz_timestamp), _, _, _)) = states.get(&channel_id) {
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
            let (tx, _rx) = watch::channel(());
            states.insert(
                channel_id,
                (QuestionState::Reading, true, HashSet::new(), tx),
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
    async fn test_state_change_notification() {
        let data = create_test_data();
        let channel_id = ChannelId::new(123456789);
        let user_id = UserId::new(987654321);
        
        // Create a watch receiver to test notifications
        let rx = {
            let mut states = data.reading_states.lock().await;
            let (tx, rx) = watch::channel(());
            states.insert(
                channel_id,
                (QuestionState::Reading, true, HashSet::new(), tx.clone()),
            );
            rx
        };

        // Simulate state change and verify notification
        {
            let mut states = data.reading_states.lock().await;
            if let Some(state) = states.get_mut(&channel_id) {
                state.0 = QuestionState::Buzzed(user_id, 12345);
                // Send notification
                let _ = state.3.send(());
            }
        }

        // Verify that the notification was sent by checking if the receiver can detect the change
        let mut rx_clone = rx;
        let notification_result = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            rx_clone.changed()
        ).await;
        
        assert!(notification_result.is_ok(), "State change notification should be received");
    }
}