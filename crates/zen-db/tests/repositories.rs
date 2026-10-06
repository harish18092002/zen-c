use std::sync::Arc;

use time::OffsetDateTime;
use uuid::Uuid;
use zen_core::ports::{ProfileRepository, SessionRepository};
use zen_db::{connection, migrations, SqliteProfileRepository, SqliteSessionRepository};
use zen_domain::{
    entities::{
        AbortReason, BlockRule, BlockRuleKind, EnforcementPlan, Profile, ProfileId, Session,
        SessionId, SessionMode, SessionState, Strictness,
    },
    events::DomainEvent,
};

async fn fresh_db() -> sqlx::SqlitePool {
    let pool = connection::connect_memory()
        .await
        .expect("memory db connect");
    migrations::run(&pool).await.expect("migrations");
    pool
}

fn fixture_profile(name: &str) -> Profile {
    let now = OffsetDateTime::now_utc();
    let id = ProfileId::new();
    Profile {
        block_rules: vec![
            BlockRule {
                id: Uuid::new_v4(),
                profile_id: id.clone(),
                kind: BlockRuleKind::App,
                target: "com.apple.Safari".to_string(),
                enabled: true,
            },
            BlockRule {
                id: Uuid::new_v4(),
                profile_id: id.clone(),
                kind: BlockRuleKind::Domain,
                target: "twitter.com".to_string(),
                enabled: true,
            },
        ],
        id,
        name: name.to_string(),
        revision: 1,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn profile_save_and_round_trip() {
    let pool = fresh_db().await;
    let repo = SqliteProfileRepository::new(pool);

    let profile = fixture_profile("Deep Work");
    repo.save(&profile).await.expect("save profile");

    let fetched = repo
        .find_by_id(&profile.id)
        .await
        .expect("find profile")
        .expect("profile present");

    assert_eq!(fetched.name, "Deep Work");
    assert_eq!(fetched.revision, 1);
    assert_eq!(fetched.block_rules.len(), 2);

    let kinds: Vec<_> = fetched.block_rules.iter().map(|r| r.kind.clone()).collect();
    assert!(kinds.contains(&BlockRuleKind::App));
    assert!(kinds.contains(&BlockRuleKind::Domain));
}

#[tokio::test]
async fn profile_update_replaces_rules() {
    let pool = fresh_db().await;
    let repo = SqliteProfileRepository::new(pool);

    let mut profile = fixture_profile("Mornings");
    repo.save(&profile).await.unwrap();

    profile.block_rules.truncate(1);
    profile.revision = 2;
    profile.updated_at = OffsetDateTime::now_utc();
    repo.save(&profile).await.unwrap();

    let fetched = repo.find_by_id(&profile.id).await.unwrap().unwrap();
    assert_eq!(fetched.block_rules.len(), 1);
    assert_eq!(fetched.revision, 2);
}

#[tokio::test]
async fn profile_list_returns_all_with_rules() {
    let pool = fresh_db().await;
    let repo = SqliteProfileRepository::new(pool);

    repo.save(&fixture_profile("A")).await.unwrap();
    repo.save(&fixture_profile("B")).await.unwrap();
    repo.save(&fixture_profile("C")).await.unwrap();

    let list = repo.list_all().await.unwrap();
    assert_eq!(list.len(), 3);
    for p in &list {
        assert_eq!(p.block_rules.len(), 2);
    }
}

#[tokio::test]
async fn session_save_and_find() {
    let pool = fresh_db().await;
    let profile_repo = SqliteProfileRepository::new(pool.clone());
    let session_repo = SqliteSessionRepository::new(pool);

    let profile = fixture_profile("Focus");
    profile_repo.save(&profile).await.unwrap();

    let session = Session::new(profile.id.clone(), SessionMode::Focus, 25 * 60);
    session_repo.save(&session).await.unwrap();

    let fetched = session_repo.find_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(fetched.id, session.id);
    assert_eq!(fetched.mode, SessionMode::Focus);
    assert_eq!(fetched.planned_duration_secs, 25 * 60);
    assert!(matches!(fetched.state, SessionState::Idle));
}

#[tokio::test]
async fn session_active_round_trip() {
    let pool = fresh_db().await;
    let profile_repo = SqliteProfileRepository::new(pool.clone());
    let session_repo = SqliteSessionRepository::new(pool);

    let profile = fixture_profile("Round trip");
    profile_repo.save(&profile).await.unwrap();

    let now = OffsetDateTime::now_utc();
    let session = Session {
        id: SessionId::new(),
        profile_id: profile.id.clone(),
        mode: SessionMode::Focus,
        state: SessionState::Active {
            started_at: now,
            ends_at: now + time::Duration::seconds(1500),
        },
        planned_duration_secs: 1500,
        created_at: now,
    };
    session_repo.save(&session).await.unwrap();

    let fetched = session_repo.find_active().await.unwrap().unwrap();
    match fetched.state {
        SessionState::Active {
            started_at,
            ends_at,
        } => {
            assert_eq!((ends_at - started_at).whole_seconds(), 1500);
        }
        other => panic!("expected Active, got {other:?}"),
    }
}

#[tokio::test]
async fn session_aborted_persists_reason() {
    let pool = fresh_db().await;
    let profile_repo = SqliteProfileRepository::new(pool.clone());
    let session_repo = SqliteSessionRepository::new(pool);

    let profile = fixture_profile("Abort test");
    profile_repo.save(&profile).await.unwrap();

    let mut session = Session::new(profile.id.clone(), SessionMode::Focus, 1500);
    session.state = SessionState::Aborted {
        reason: AbortReason::UserRequested,
    };
    session_repo.save(&session).await.unwrap();

    let fetched = session_repo.find_by_id(&session.id).await.unwrap().unwrap();
    assert_eq!(
        fetched.state,
        SessionState::Aborted {
            reason: AbortReason::UserRequested
        }
    );
}

#[tokio::test]
async fn session_find_active_skips_terminal() {
    let pool = fresh_db().await;
    let profile_repo = SqliteProfileRepository::new(pool.clone());
    let session_repo = SqliteSessionRepository::new(pool);

    let profile = fixture_profile("Filter");
    profile_repo.save(&profile).await.unwrap();

    let mut completed = Session::new(profile.id.clone(), SessionMode::Focus, 1500);
    completed.state = SessionState::Completed;
    session_repo.save(&completed).await.unwrap();

    assert!(session_repo.find_active().await.unwrap().is_none());
}

#[tokio::test]
async fn append_event_writes_session_event_row() {
    let pool = fresh_db().await;
    let profile_repo = SqliteProfileRepository::new(pool.clone());
    let session_repo: Arc<dyn SessionRepository> =
        Arc::new(SqliteSessionRepository::new(pool.clone()));

    let profile = fixture_profile("Events");
    profile_repo.save(&profile).await.unwrap();

    let mut session = Session::new(profile.id.clone(), SessionMode::Focus, 60);
    session.state = SessionState::Preparing;
    session_repo.save(&session).await.unwrap();

    let plan = EnforcementPlan {
        revision: 1,
        session_id: session.id.clone(),
        app_rules: vec![],
        domain_rules: vec![],
        network_rules: vec![],
        strictness: Strictness::Normal,
        plan_hash: "deadbeef".into(),
    };
    let event = DomainEvent::EnforcementApplied {
        session_id: session.id.clone(),
        plan: Box::new(plan),
        at: OffsetDateTime::now_utc(),
    };
    session_repo.append_event(&event).await.unwrap();

    use sqlx::Row;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);

    let enforcement_row =
        sqlx::query("SELECT plan_revision, plan_hash, status FROM enforcement_state")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        enforcement_row.try_get::<i64, _>("plan_revision").unwrap(),
        1
    );
    assert_eq!(
        enforcement_row.try_get::<String, _>("plan_hash").unwrap(),
        "deadbeef"
    );
}

#[tokio::test]
async fn tamper_event_routes_to_tamper_table() {
    let pool = fresh_db().await;
    let session_repo = SqliteSessionRepository::new(pool.clone());

    let event = DomainEvent::TamperDetected {
        kind: zen_domain::events::TamperKind::ClockRollback,
        detail: Some("clock moved backwards 5m".into()),
        at: OffsetDateTime::now_utc(),
    };
    session_repo.append_event(&event).await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tamper_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);

    let session_event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(session_event_count, 0);
}
