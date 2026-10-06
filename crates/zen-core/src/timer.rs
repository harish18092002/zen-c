use std::sync::Arc;

use time::OffsetDateTime;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{info, warn};
use zen_domain::entities::SessionId;

/// Snapshot returned on every tick. Frontends pull this through a Tauri event
/// and use `remaining_secs` directly — never compute their own countdown from
/// `started_at`, since wall-clock drift would diverge from this source.
#[derive(Debug, Clone)]
pub struct TimerSnapshot {
    pub session_id: SessionId,
    pub remaining_secs: u64,
    pub elapsed_secs: u64,
    pub planned_secs: u64,
    pub state: TimerState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Running,
    Paused,
    Completed,
    Idle,
}

#[derive(Clone)]
struct ActiveTimer {
    session_id: SessionId,
    monotonic_start: Instant,
    wall_start: OffsetDateTime,
    planned: u64,
    /// Wall-clock instant we observed on the previous tick. Used to flag
    /// rollback (negative deltas) and large forward jumps (sleep/resume on a
    /// machine whose clock skewed).
    last_wall: OffsetDateTime,
    paused_remaining: Option<u64>,
}

/// `TimerEngine` is the single source of truth for "how much time is left in
/// the active session." It uses a monotonic clock for elapsed-time math so
/// that user-induced wall-clock changes can't shorten a session. The wall
/// clock is observed separately to detect tampering.
#[derive(Default)]
pub struct TimerEngine {
    inner: Arc<RwLock<Option<ActiveTimer>>>,
}

impl TimerEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(&self, session_id: SessionId, planned_secs: u64) {
        let now_wall = OffsetDateTime::now_utc();
        let mut guard = self.inner.write().await;
        *guard = Some(ActiveTimer {
            session_id,
            monotonic_start: Instant::now(),
            wall_start: now_wall,
            planned: planned_secs,
            last_wall: now_wall,
            paused_remaining: None,
        });
    }

    pub async fn pause(&self) -> Option<u64> {
        let mut guard = self.inner.write().await;
        if let Some(timer) = guard.as_mut() {
            if timer.paused_remaining.is_some() {
                return timer.paused_remaining;
            }
            let elapsed = timer.monotonic_start.elapsed().as_secs();
            let remaining = timer.planned.saturating_sub(elapsed);
            timer.paused_remaining = Some(remaining);
            return Some(remaining);
        }
        None
    }

    pub async fn resume(&self) {
        let mut guard = self.inner.write().await;
        if let Some(timer) = guard.as_mut() {
            if let Some(remaining) = timer.paused_remaining.take() {
                timer.monotonic_start = Instant::now();
                timer.planned = remaining;
                timer.wall_start = OffsetDateTime::now_utc();
                timer.last_wall = timer.wall_start;
            }
        }
    }

    pub async fn stop(&self) {
        let mut guard = self.inner.write().await;
        *guard = None;
    }

    pub async fn is_running(&self) -> bool {
        self.inner.read().await.is_some()
    }

    pub async fn current_session(&self) -> Option<SessionId> {
        self.inner
            .read()
            .await
            .as_ref()
            .map(|t| t.session_id.clone())
    }

    pub async fn tick(&self) -> Option<TimerSnapshot> {
        let mut guard = self.inner.write().await;
        let timer = guard.as_mut()?;

        let now_wall = OffsetDateTime::now_utc();
        let wall_delta = (now_wall - timer.last_wall).whole_seconds();
        if wall_delta < -2 {
            warn!(
                session_id = %timer.session_id.0,
                wall_delta,
                "timer: wall clock moved backwards"
            );
        }
        timer.last_wall = now_wall;

        let (state, elapsed_secs, remaining_secs) = if let Some(rem) = timer.paused_remaining {
            (TimerState::Paused, timer.planned.saturating_sub(rem), rem)
        } else {
            let elapsed = timer.monotonic_start.elapsed().as_secs();
            if elapsed >= timer.planned {
                (TimerState::Completed, timer.planned, 0)
            } else {
                (TimerState::Running, elapsed, timer.planned - elapsed)
            }
        };

        Some(TimerSnapshot {
            session_id: timer.session_id.clone(),
            remaining_secs,
            elapsed_secs,
            planned_secs: timer.planned,
            state,
        })
    }

    /// Spawn a 1-Hz tick loop on the current tokio runtime. Each emitted
    /// snapshot is handed to `on_tick`; the loop exits when the timer is
    /// stopped externally and the callback returns `false`.
    pub fn spawn_loop<F>(self: Arc<Self>, mut on_tick: F) -> tokio::task::JoinHandle<()>
    where
        F: FnMut(TimerSnapshot) -> bool + Send + 'static,
    {
        let engine = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                match engine.tick().await {
                    Some(snap) => {
                        let continue_loop = on_tick(snap);
                        if !continue_loop {
                            break;
                        }
                    }
                    None => {
                        // Timer was stopped externally — keep the loop alive so a
                        // future `start` resumes ticks without a new task.
                        continue;
                    }
                }
            }
            info!("timer loop exited");
        })
    }
}
