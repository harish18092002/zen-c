//! End-to-end: SessionService driving real sqlx repositories + a Noop blocker.
//! Verifies that the service's orchestration of FSM + persistence is correct
//! when wired to real implementations rather than mocks.

use std::sync::Arc;
use time::OffsetDateTime;
use zen_core::ports::ProfileRepository;
use zen_core::{NoopBlockingAdapter, SessionService};
use zen_db::{connection, migrations, SqliteProfileRepository, SqliteSessionRepository};
use zen_domain::entities::{
    AbortReason, BlockRule, BlockRuleKind, Profile, ProfileId, SessionMode, SessionState,
    Strictness,
};

async fn build() -> (SessionService, ProfileId) {
    let pool = connection::connect_memory().await.unwrap();
    migrations::run(&pool).await.unwrap();

    let sessions = Arc::new(SqliteSessionRepository::new(pool.clone()));
    let profiles = Arc::new(SqliteProfileRepository::new(pool.clone()));
    let blocker = Arc::new(NoopBlockingAdapter::new());
    let service = SessionService::new(sessions.clone(), profiles.clone(), blocker);

    let pid = ProfileId::new();
    let profile = Profile {
        id: pid.clone(),
        name: "E2E".into(),
        revision: 1,
        block_rules: vec![BlockRule {
            id: uuid::Uuid::new_v4(),
            profile_id: pid.clone(),
            kind: BlockRuleKind::Domain,
            target: "example.com".into(),
            enabled: true,
        }],
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    };
    profiles.save(&profile).await.unwrap();
    (service, pid)
}

#[tokio::test]
async fn full_session_lifecycle() {
    let (service, profile_id) = build().await;

    let started = service
        .start_session(profile_id, 60, SessionMode::Focus, Strictness::Normal)
        .await
        .unwrap();

    let session = service.get_active_session().await.unwrap().unwrap();
    assert_eq!(session.id, started.session_id);
    assert!(matches!(session.state, SessionState::Active { .. }));

    service
        .pause_session(&started.session_id, 30)
        .await
        .unwrap();
    let after_pause = service
        .get_session(&started.session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        after_pause.state,
        SessionState::Paused { remaining_secs: 30 }
    );

    service.resume_session(&started.session_id).await.unwrap();
    let after_resume = service
        .get_session(&started.session_id)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(after_resume.state, SessionState::Active { .. }));

    service.complete_session(&started.session_id).await.unwrap();
    let final_state = service
        .get_session(&started.session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(final_state.state, SessionState::Completed);
    assert!(service.get_active_session().await.unwrap().is_none());
}

#[tokio::test]
async fn cannot_start_two_sessions_concurrently() {
    let (service, profile_id) = build().await;
    service
        .start_session(
            profile_id.clone(),
            60,
            SessionMode::Focus,
            Strictness::Normal,
        )
        .await
        .unwrap();
    let err = service
        .start_session(profile_id, 60, SessionMode::Focus, Strictness::Normal)
        .await
        .expect_err("second start should be rejected");
    assert!(format!("{err}").contains("already active"));
}

#[tokio::test]
async fn abort_records_reason() {
    let (service, profile_id) = build().await;
    let started = service
        .start_session(profile_id, 60, SessionMode::Focus, Strictness::Normal)
        .await
        .unwrap();
    service
        .stop_session(&started.session_id, AbortReason::PermissionRevoked)
        .await
        .unwrap();
    let fetched = service
        .get_session(&started.session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        fetched.state,
        SessionState::Aborted {
            reason: AbortReason::PermissionRevoked
        }
    );
}

#[tokio::test]
async fn recovers_active_session_after_restart() {
    let pool = connection::connect_memory().await.unwrap();
    migrations::run(&pool).await.unwrap();

    // Phase 1: original boot, start a session.
    let sessions: Arc<dyn zen_core::ports::SessionRepository> =
        Arc::new(SqliteSessionRepository::new(pool.clone()));
    let profiles: Arc<dyn zen_core::ports::ProfileRepository> =
        Arc::new(SqliteProfileRepository::new(pool.clone()));
    let blocker = Arc::new(NoopBlockingAdapter::new());
    let service = SessionService::new(sessions.clone(), profiles.clone(), blocker.clone());

    let pid = ProfileId::new();
    let profile = Profile {
        id: pid.clone(),
        name: "Persistence".into(),
        revision: 1,
        block_rules: vec![],
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    };
    profiles.save(&profile).await.unwrap();

    let started = service
        .start_session(pid, 3600, SessionMode::Focus, Strictness::Normal)
        .await
        .unwrap();
    drop(service);

    // Phase 2: simulate restart by building a fresh service against the same pool.
    let sessions: Arc<dyn zen_core::ports::SessionRepository> =
        Arc::new(SqliteSessionRepository::new(pool.clone()));
    let profiles: Arc<dyn zen_core::ports::ProfileRepository> =
        Arc::new(SqliteProfileRepository::new(pool.clone()));
    let new_service = SessionService::new(sessions, profiles, blocker);

    let recovered = new_service
        .recover_active_session()
        .await
        .unwrap()
        .expect("active session should be recovered");
    assert_eq!(recovered.session.id, started.session_id);
    assert!(recovered.remaining_secs <= 3600);
}
