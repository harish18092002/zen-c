# Zen Mode — Project Status & Implementation Plan

_Last updated: 2026-05-12. Branch: `master`._

This document is the authoritative "where are we and what's next" view of the codebase. The polished marketing/onboarding `README.md` describes what Zen Mode **is**. This document describes what's actually **built**, what's still **deferred**, and the rationale for both.

---

## 1. TL;DR

Phase 1 MVP is **mechanically complete**. The desktop app:

- Persists profiles and sessions to a real SQLite database (sqlx, WAL, migrations, transactions).
- Drives a real session FSM through `SessionService` with full pause/resume/abort/complete.
- Runs a monotonic-clock `TimerEngine` and emits 1-Hz `session://tick` events to the React UI.
- Recovers an interrupted session on startup, re-applies enforcement, detects clock rollback.
- Offers Profile CRUD (create/edit/delete) with react-hook-form + zod validation.
- Shows a system tray with a live countdown and Pause/Resume/End menu items.
- Performs macOS app blocking via a `sysinfo`-driven polling loop that terminates processes matching bundle-id or executable-name rules.
- Probes the macOS Accessibility permission via direct FFI to `AXIsProcessTrusted`.
- Ships a real `zen-cli` diagnostic tool that reads the same SQLite file.

| Layer | State |
|-------|-------|
| Workspace, crate layout, deps | ✅ |
| Domain layer (`zen-domain`) | ✅ FSM, policy compiler, events, errors — 4 unit tests |
| Core orchestration (`zen-core`) | ✅ `SessionService`, `TimerEngine`, `NoopBlockingAdapter`, ports — 4 timer tests |
| Persistence (`zen-db`) | ✅ Real sqlx repos, transactions, tamper event routing — 13 integration tests |
| macOS adapter (`zen-macos`) | ✅ Polling-based app blocker, AX permission FFI — 3 unit tests |
| Windows adapter (`zen-windows`) | 🟡 Composed `BlockingAdapter` builds, but enforcement is still stub-level |
| Tauri shell (`apps/desktop-ui/src-tauri`) | ✅ 12 typed commands, real DB-backed app state, crash recovery on boot, tray, timer loop |
| React UI (`apps/desktop-ui/src`) | ✅ Dashboard with live timer, Profiles list, ProfileEditor (RHF+zod), Settings with live perms |
| Shared contracts (`@zen-mode/ui-contracts`) | ✅ Single source of TS truth, consumed by the desktop UI |
| `zen-cli` | ✅ Real DB queries: active-session / history / profiles / tamper / integrity |
| CI | ✅ `fmt --check`, `clippy -D warnings`, `cargo test` on macOS + Windows; frontend typecheck + lint |
| Tests | ✅ 24 passing (`cargo test --workspace`), `cargo clippy` clean |

What's still genuinely deferred is called out in §4 below.

---

## 2. What's actually built today

### 2.1 `zen-domain` — Pure logic ✅

- `entities.rs` — Full type model with `as_str()` / `parse()` helpers on every enum that touches the DB or wire.
- `session.rs` — `SessionFsm` with all transitions plus a `restore()` constructor for crash recovery; new `is_terminal()` / `is_running()` predicates.
- `policy.rs` — `PolicyCompiler::compile` normalizes domains, hashes the plan.
- `events.rs` — `DomainEvent::event_type()` and `session_id()` helpers so the persistence layer doesn't have to re-match every variant.
- `errors.rs` — Added `InvalidInput` and `Persistence` variants so the DB layer can surface infra errors without abusing `PlanCompilationFailed`.

Tests: `session_happy_path`, `invalid_transition_returns_error`, `compiles_domain_rules`, `skips_disabled_rules`.

### 2.2 `zen-core` — Use cases, timer, ports ✅

- `ports.rs` — Unchanged: `SessionRepository`, `ProfileRepository`, `BlockingAdapter`.
- `session_service.rs` — Rewritten. Now produces `StartedSession` (with `ends_at`, plan), supports `pause_session` / `resume_session` / `complete_session`, validates duration (>0, ≤24h), refuses a second concurrent session, and `recover_active_session` returns a `RecoveredSession` with `clock_rollback_suspected` + remaining seconds. Fixed the stale-state bug where `Session` was saved as `Idle` after the FSM had moved it to `Active`.
- `timer.rs` — New. `TimerEngine` uses `tokio::time::Instant` (monotonic in production, mockable in tests) for elapsed math, tracks `last_wall` to flag rollback, supports start/pause/resume/stop/tick. Spawns a 1-Hz loop via `spawn_loop`.
- `noop_blocker.rs` — New. `NoopBlockingAdapter` for non-mac/non-win targets, tests, and CLI use.

Tests: `ticks_decrement_remaining`, `completes_when_elapsed_reaches_planned`, `pause_and_resume_preserve_remaining`, `stop_clears_active_timer`.

### 2.3 `zen-db` — Real persistence ✅

- `connection.rs` — Adds `connect_file(&Path)` (creates parent dirs, WAL, 30s busy timeout, 8-conn pool) and `connect_memory()` (1-conn `:memory:` for tests).
- `migrations/001_initial.sql` — Schema unchanged.
- `migrations/002_session_state_columns.sql` — **New.** Adds `ends_at`, `paused_remaining_secs`, `aborted_reason`, `updated_at` columns; `state` column now holds only the discriminant string.
- `profile_repo.rs` — Real upsert in a transaction (`profiles` upsert → delete-and-reinsert `block_rules`). `find_by_id` and `list_all` join rules in one extra query. New `delete()` method.
- `session_repo.rs` — Real upsert with denormalized state columns, derives `started_at` / `completed_at` from the FSM state, `find_active` filters on the discriminant set. `append_event` routes `TamperDetected` to `tamper_events` and mirrors `EnforcementApplied` into `enforcement_state`.
- `tamper_repo.rs` — **New.** Read/write helper for `tamper_events` (used by Tauri's "recent tamper" command and the CLI).
- `error.rs` — **New.** Centralized sqlx/serde/time/uuid → `DomainError::Persistence` mappers.

Tests: `profile_save_and_round_trip`, `profile_update_replaces_rules`, `profile_list_returns_all_with_rules`, `session_save_and_find`, `session_active_round_trip`, `session_aborted_persists_reason`, `session_find_active_skips_terminal`, `append_event_writes_session_event_row`, `tamper_event_routes_to_tamper_table`, plus 4 end-to-end `SessionService` tests covering the full lifecycle, two-session rejection, abort reason persistence, and crash recovery across a service restart.

### 2.4 OS adapters

**macOS (`zen-macos`)** ✅
- `app_blocker.rs` — Real implementation. Spawns a 750ms-interval polling task using `sysinfo`. Matches block targets against process name AND executable path AND the tail segment of dotted bundle IDs (`com.apple.Safari` → also matches a process named `Safari`). Terminates via `kill_with(Signal::Term)`. Aborts the previous loop on revoke / re-apply. Includes 3 unit tests for `split_target` and `matches_any`.
- `permissions.rs` — FFI to `AXIsProcessTrusted` from `ApplicationServices`. Returns real `Healthy` / `PermissionDenied`.
- `dns_proxy.rs` / `workspace_monitor.rs` — Still log-only. Network domain blocking + NSWorkspace notifications are Phase 2 work (see §4).
- `adapter.rs` — **New.** `MacOsBlockingAdapter` composes app + DNS adapters under the unified `BlockingAdapter` trait, tracking the active plan revision.

**Windows (`zen-windows`)** 🟡
- `adapter.rs` — **New.** `WindowsBlockingAdapter` composes app + WFP adapters under the unified trait, same shape as macOS.
- `app_blocker.rs` / `wfp_blocker.rs` — Still log-only. Real enforcement via `windows-rs` is Phase 2 work.

### 2.5 Tauri shell (`apps/desktop-ui/src-tauri`) ✅

- `lib.rs` — Real bootstrap. Initializes tracing, builds the Tauri app, runs `setup::bootstrap` synchronously (via `block_on`), spawns the timer event loop, installs the tray icon, and registers 12 commands.
- `setup.rs` — **New.** Opens the SQLite file in `app_data_dir`, runs migrations, builds repos + the platform-specific `BlockingAdapter` (macOS / Windows / fallback `Noop`), constructs `SessionService` and `TimerEngine`. Runs `recover_active_session` before the window appears.
- `state.rs` — **New.** `AppState { service, timer, tamper, pool }`. Cheap to clone (all `Arc` or shallow handles).
- `dto.rs` — **New.** Serde-friendly DTOs (camelCase, RFC3339 strings) with conversion helpers from domain types.
- `commands.rs` — Rewritten. All 12 commands hit the real service: `start_session`, `stop_session`, `pause_session`, `resume_session`, `get_session_state`, `get_active_session`, `list_profiles`, `get_profile`, `upsert_profile`, `delete_profile`, `probe_permissions`, `list_recent_tamper`. Validates UUIDs, refuses to delete profiles in use by the active session.
- `permissions.rs` — **New.** Aggregates the macOS / Windows permission probes into a wire-friendly list.
- `tray.rs` — **New.** Tray with Open / Pause / Resume / End / Quit menu, left-click opens the main window, live countdown title updates from the timer loop.

Timer events emitted: `session://tick`, `session://completed`, `session://tamper`.

### 2.6 React frontend (`apps/desktop-ui/src`) ✅

- `main.tsx` — Imports a real `styles.css`, hardens `QueryClient` defaults (`refetchOnWindowFocus: false`).
- `router.tsx` — Mounts `AppLayout` as the root component and adds `/profiles/new` + `/profiles/$profileId` routes for the editor.
- `AppLayout.tsx` — **New.** Persistent header with TanStack Router `<Link>` (no more `<a>` page reloads), mounts `useTimerSubscription`, hydrates the Zustand store from `getActiveSession()` on first paint.
- `hooks/useTimerSubscription.ts` — **New.** `listen('session://tick', ...)` and `'session://completed'` wired into the store.
- `store/session.ts` — Rewritten. Tracks `sessionId`, `stateKind`, `timerState`, `remainingSecs`, `elapsedSecs`, `plannedSecs`. `hydrate()` accepts a `Session` from the backend; `applyTick()` accepts a `TimerTick`.
- `pages/Dashboard.tsx` — Rewritten. Profile selector, four preset durations, live countdown with progress bar, error banner, Pause / Resume / End controls.
- `pages/Profiles.tsx` — Rewritten. Real list with rule counts, inline confirm-to-delete, link to editor.
- `pages/ProfileEditor.tsx` — **New.** RHF + zod form with `useFieldArray` for block rules, kind selector with sensible placeholders, used by both `/profiles/new` and `/profiles/$profileId`.
- `pages/Settings.tsx` — Rewritten. Live permission probe with colored status pills, recent tamper events feed, refetches every 5 min / 1 min respectively.
- `commands/index.ts` — Now imports types from `@zen-mode/ui-contracts`. Covers all 12 IPC commands.

### 2.7 Shared contracts (`packages/ui-contracts`) ✅

Single source of truth for cross-boundary TypeScript types: `Session`, `SessionState` (tagged union), `Profile`, `BlockRule`, `BlockRuleInput`, `StartSessionRequest`, `StartSessionResult`, `TimerTick`, `Permission`, `TamperRecord`. Exports configured for both type imports and default-mode imports. Wired into `apps/desktop-ui/package.json` via `workspace:*`.

### 2.8 `zen-cli` ✅

Rewritten. Resolves the same SQLite path the desktop app uses (or accepts `--db <path>`). Subcommands:
- `db active-session` — current session row
- `db history --limit N` — recent sessions
- `db profiles` — every profile with rule counts
- `db tamper --limit N` — recent tamper events
- `db integrity` — `PRAGMA integrity_check`
- `permissions` — calls the platform probe

### 2.9 CI ✅

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (matrix: `macos-14`, `windows-latest`)
- Frontend: `pnpm install --no-frozen-lockfile`, `pnpm typecheck`, `pnpm lint` (lint errors allowed for now until first ESLint run on real CI hardware is verified).

`.cspell.json` added to silence noise on technical vocabulary (Tauri, sqlx, objc, etc.).

---

## 3. How to verify locally

```bash
# Rust
cargo fmt --all -- --check     # passes
cargo clippy --workspace --all-targets -- -D warnings  # passes
cargo test --workspace          # 24 passing

# Frontend
pnpm install
pnpm --filter desktop-ui typecheck
pnpm --filter desktop-ui lint

# Run the desktop app
pnpm dev

# Inspect the local DB
cargo run --bin zen-cli -- db active-session
cargo run --bin zen-cli -- db history --limit 5
cargo run --bin zen-cli -- db profiles
cargo run --bin zen-cli -- permissions
```

---

## 4. What's genuinely deferred to Phase 2+

These are intentional skips, not oversights. Each is sized and scoped so the next person picking up the project can estimate it.

### 4.1 macOS — NSWorkspace observers (vs. polling)

We currently poll `sysinfo::System::processes()` every 750ms. This works, but a launched-and-instantly-killed app may flash on screen for ~half a second. The fully tamper-aware version registers `NSWorkspaceDidLaunchApplicationNotification` and terminates synchronously inside the callback. This requires either `objc2` block closures or a small Swift helper bundled into `native/macos-helper/`. Estimate: 2-3 days, plus signing review.

### 4.2 macOS — DNS / website blocking

`zen-macos::dns_proxy.rs` is still a stub. The real implementation needs either:
- A local DNS resolver bound to 127.0.0.1 with system DNS rewritten via `scutil --dns`, OR
- A Network Extension (requires special entitlement and direct-distribution-only build).

Estimate: 1-2 weeks including notarization handshake.

### 4.3 Windows enforcement

Both `app_blocker` and `wfp_blocker` still log. App-level termination via `windows-rs` is straightforward (~1 day). WFP-based network blocking requires a kernel-mode filter driver and is a multi-week effort that warrants an ADR first.

### 4.4 Reconciliation loop

The intended design (per the roadmap) is a background task that re-queries the OS for *actual* enforcement state and diffs against the *desired* plan, emitting `TamperKind::RulesDiverged` on divergence. The infrastructure is there (`SessionService::blocker()`, `BlockingAdapter::probe_health()`); the loop itself isn't wired. Estimate: 1 day once we agree on the cadence.

### 4.5 Strict-mode cancellation friction

`stop_session` works for any state today. The roadmap calls for a cooldown / secondary-auth gate when `strictness == Strict`. The `Strictness` field is plumbed through `EnforcementPlan` and the `start_session` command; the enforcement at stop-time is unimplemented. Estimate: 1 day for cooldown + 2-3 days for biometric / TouchID gating.

### 4.6 Signing, notarization, installers

`infra/signing`, `infra/installer`, `infra/update-feeds` are all empty placeholders. Tauri 2's bundler handles much of this with the right configuration; full notarization scripting + a self-update channel is roughly a week.

### 4.7 E2EE sync

`zen-sync` has the protocol skeleton and a no-op `SyncEncryptor`. Implementing AES-256-GCM envelope crypto + the device-key bootstrap is a multi-week project that should not block MVP.

### 4.8 Browser extensions

Roadmap Phase 3. Untouched.

### 4.9 Frontend lockfile

`pnpm-lock.yaml` still isn't committed (this requires running `pnpm install` in a network-connected environment). Once committed, flip CI to `--frozen-lockfile` and re-enable the pnpm cache step.

---

## 5. Open architectural decisions (still worth an ADR each)

1. **DB encryption** — SQLite via `sqlx` is plaintext on disk. SQLCipher integration affects bundling, key custody, and recovery.
2. **Single-process vs. split agent** — The README describes `zen-ui` / `zen-agent` / `zen-elevated-helper` as separate executables; today we ship one Tauri process. The split is the right end state for tamper resistance.
3. **Type sync between Rust & TS** — Manually maintained today in `dto.rs` ↔ `@zen-mode/ui-contracts`. `specta` could automate this; defer until churn justifies the investment.
4. **Snapshot cadence** — `session_snapshots` table exists but is never written. With short sessions the event log is bounded; for long-running strict-mode sessions a snapshot every N events would speed recovery.
5. **OS adapter test strategy** — `zen-macos` unit tests cover the matcher logic in isolation. Full "spawn Safari, expect terminate within 1s" tests need real macOS in CI and aren't wired yet.
