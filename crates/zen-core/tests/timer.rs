use std::sync::Arc;

use zen_core::{TimerEngine, TimerState};
use zen_domain::entities::SessionId;

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn ticks_decrement_remaining() {
    let timer = Arc::new(TimerEngine::new());
    let id = SessionId::new();
    timer.start(id.clone(), 5).await;

    let snap = timer.tick().await.unwrap();
    assert_eq!(snap.session_id, id);
    assert_eq!(snap.planned_secs, 5);
    assert_eq!(snap.remaining_secs, 5);

    tokio::time::advance(std::time::Duration::from_secs(3)).await;
    let snap = timer.tick().await.unwrap();
    assert_eq!(snap.remaining_secs, 2);
    assert_eq!(snap.elapsed_secs, 3);
    assert_eq!(snap.state, TimerState::Running);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn completes_when_elapsed_reaches_planned() {
    let timer = Arc::new(TimerEngine::new());
    timer.start(SessionId::new(), 2).await;

    tokio::time::advance(std::time::Duration::from_secs(2)).await;
    let snap = timer.tick().await.unwrap();
    assert_eq!(snap.remaining_secs, 0);
    assert_eq!(snap.state, TimerState::Completed);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn pause_and_resume_preserve_remaining() {
    let timer = Arc::new(TimerEngine::new());
    timer.start(SessionId::new(), 10).await;

    tokio::time::advance(std::time::Duration::from_secs(3)).await;
    let remaining = timer.pause().await.unwrap();
    assert_eq!(remaining, 7);

    // While paused, time can pass with no decrement.
    tokio::time::advance(std::time::Duration::from_secs(60)).await;
    let snap = timer.tick().await.unwrap();
    assert_eq!(snap.remaining_secs, 7);
    assert_eq!(snap.state, TimerState::Paused);

    timer.resume().await;
    tokio::time::advance(std::time::Duration::from_secs(4)).await;
    let snap = timer.tick().await.unwrap();
    assert_eq!(snap.remaining_secs, 3);
    assert_eq!(snap.state, TimerState::Running);
}

#[tokio::test]
async fn stop_clears_active_timer() {
    let timer = Arc::new(TimerEngine::new());
    timer.start(SessionId::new(), 10).await;
    assert!(timer.is_running().await);
    timer.stop().await;
    assert!(!timer.is_running().await);
    assert!(timer.tick().await.is_none());
}
