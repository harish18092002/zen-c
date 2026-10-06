# Zen Mode — System Architecture & Learning Guide

> A guide written for someone who knows backend microservices (NestJS / TypeScript / PostgreSQL / Redis) but is new to Rust, Tauri, and desktop app architecture.

This document explains **what Zen Mode is**, **how it works**, and **why we built it the way we did** — using analogies to concepts you already know from your fintech / payments background.

---

## Part 0 — Quick Concept Map (NestJS World ↔ Rust/Tauri World)

If you've worked with NestJS + PostgreSQL + Redis + React, here's how everything translates:

| You know this (NestJS / TS world) | We use this (Rust / Tauri world) | Same thing? |
|-----------------------------------|----------------------------------|-------------|
| `package.json` | `Cargo.toml` | ✅ Yes — both declare deps + metadata |
| `npm install` / `pnpm install` | `cargo build` / `cargo check` | ✅ Yes — fetches deps and compiles |
| `node_modules/` | `target/` | ✅ Yes — build output cache |
| `pnpm-workspace.yaml` (monorepo) | `[workspace]` in root `Cargo.toml` | ✅ Yes — Rust workspaces work the same |
| NPM package | "Crate" | ✅ Yes — just different naming |
| TypeScript `interface` | Rust `trait` | ⚠️ Similar — traits are runtime-dispatched too |
| TypeScript `type` / `enum` | Rust `enum` (much more powerful) | ⚠️ Rust enums hold data per variant |
| `class` with methods | `struct` + `impl` block | ⚠️ Same idea, different syntax |
| `async/await` (Promises) | `async/await` (Futures) | ✅ Yes — Tokio is like Node's event loop |
| NestJS `@Injectable()` service | Rust struct passed via `Arc<dyn Trait>` | ⚠️ DI by hand, no decorators |
| NestJS module system | Rust workspace + crate boundaries | ✅ Yes |
| NestJS Controller | Tauri `#[tauri::command]` function | ✅ Yes — entry points for frontend |
| HTTP REST endpoint | **IPC command** (we'll go deep on this) | ⚠️ Same idea, no network |
| WebSocket push | Tauri event emit (`app.emit("foo", payload)`) | ✅ Yes |
| PostgreSQL | SQLite (file-based, no server) | ⚠️ SQL is the same |
| Redis cache | (We don't use a cache — local SQLite is fast enough) | ❌ |
| TypeORM / Drizzle | `sqlx` (the Rust SQL library) | ✅ Same idea |
| `npm test` (Vitest / Jest) | `cargo test` | ✅ Same idea |
| Docker container | Tauri bundle (`.dmg` / `.msi`) | ⚠️ Different — packaged for desktop, not server |
| Cloud Run service | Background agent process on user's machine | ❌ Different — runs locally, not in cloud |
| GitHub Actions YAML | Same — works for Rust too | ✅ Yes |
| Audit log / transaction journal | "Event sourcing" via `session_events` table | ✅ Same pattern |

**Key mindset shift:** in the cloud world, you trust your infrastructure (the Cloud Run container, the Postgres connection). In the desktop world, **you don't trust the user's machine** — they can kill your process, change the system clock, revoke permissions. The architecture has to defend against that.

---

## Part 1 — What is Zen Mode?

A **desktop focus app** like Cold Turkey or Freedom. The user starts a 25-minute focus session, picks a profile (e.g. "Deep Work" with Twitter, Instagram, Slack blocked), and the app blocks those apps + websites for the duration.

The hard part is not the timer. The hard part is making the blocking **reliable** even when the user actively tries to bypass it (force-quit, change clock, revoke permissions, etc.).

You can think of it like the **fraud-prevention mindset** from Surfboard Payments — except the "fraudster" is the user's distracted self trying to circumvent their own rules.

---

## Part 2 — What is Tauri? (And Why Not Just Make a Web App?)

You've built web apps with React + a NestJS backend deployed to Cloud Run. That's a **client-server** architecture: the frontend talks to a remote API.

A desktop app needs to:
1. Be installed on the user's machine (not visited via a URL)
2. Have a window with native chrome (titlebar, menu bar)
3. Talk to the operating system (kill processes, intercept network traffic, listen to global keyboard shortcuts)

There are three main ways to build this:

### Option A — Electron (Slack, VSCode, Discord)
Bundles **a full Chromium browser** inside your app and gives you Node.js APIs. Fat (~150 MB minimum), high RAM usage, but very productive.

### Option B — Native (Swift on macOS, C# on Windows)
Maximum performance and native feel, but you write the app **twice** — once for each OS.

### Option C — Tauri (what we use)
The trick: **don't bundle a browser**. Use the OS's *built-in* WebView (WKWebView on macOS, WebView2 on Windows). The frontend is React + TypeScript like usual, but it runs inside the OS WebView. The "backend" is **a Rust process running in the same binary** (not on a server).

> **New to binaries, processes, or WebViews?** Read [`HOW_OS_APPS_AND_WEBVIEWS_WORK.md`](./HOW_OS_APPS_AND_WEBVIEWS_WORK.md) first — it's a bottom-up primer that defines every term used below (binary, process, WebView, IPC, WKWebView, WebView2, Mach port, code signing, etc.).

```
┌─────────────────────────────────────────────────┐
│  Tauri App (single executable)                   │
│  ┌─────────────────────┐  ┌──────────────────┐  │
│  │ Frontend (WebView)  │◄─┤ Rust backend     │  │
│  │ React + TypeScript  │  │ (in-process)     │  │
│  │ HTML / CSS / JS     │  │                  │  │
│  └─────────────────────┘  └──────────────────┘  │
│            ▲                       ▲             │
│            │                       │             │
│            └───── IPC bridge ──────┘             │
└─────────────────────────────────────────────────┘
```

**Why Tauri specifically:**
- Bundle size: ~10 MB instead of ~150 MB
- Memory: way less than Electron (no full Chromium)
- Security: the Rust backend has explicit "capabilities" — the frontend can only call commands you've registered, like an allowlist
- Performance: Rust is fast, no GC pauses

**The mental model:**
> Tauri is like running your NestJS backend and your React frontend in **the same binary** on the user's machine, with a special in-process bridge instead of HTTP between them.

---

## Part 3 — IPC Commands Explained in Detail

This is the part you specifically asked about. Take your time here — IPC is THE pattern that connects the React UI to the Rust backend.

### What is IPC?

**IPC = Inter-Process Communication.** It's how two separate things talk to each other on the same machine.

In your existing experience:
- React frontend talks to NestJS backend over **HTTP** (REST API)
- NestJS talks to PostgreSQL over **TCP socket** (libpq protocol)
- NestJS talks to Redis over **TCP socket** (RESP protocol)

All of those are forms of IPC, just over the network.

In Tauri:
- The React frontend (running inside the WebView) talks to the Rust backend (running in the same process) using a **local message-passing bridge** that Tauri sets up for you. No HTTP. No port. No network.

### Why Not Just Use HTTP Locally?

You *could* run a tiny HTTP server inside the desktop app and have React `fetch('http://localhost:8080/api/start_session')`. Some Electron apps do this. But:

1. **Security:** anyone running on your machine could hit `localhost:8080` and start sessions. Tauri's IPC has a frontend↔backend permission system that HTTP doesn't.
2. **Performance:** HTTP has overhead (parsing headers, opening sockets). IPC is just a function call across a memory boundary.
3. **Type safety:** with HTTP you write the types twice — once in NestJS, once in React. Tauri gives you a way to share types.
4. **No port conflicts:** what if `localhost:8080` is already taken on the user's machine?

### How an IPC Command Actually Works (Step by Step)

Let's trace what happens when the user clicks **"Start Focus Session"** in our app.

#### Step 1 — Define the command on the Rust side

File: `apps/desktop-ui/src-tauri/src/commands.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct StartSessionPayload {
    pub profile_id: String,
    pub duration_secs: u64,
    pub mode: String,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub state: String,
}

#[tauri::command]
pub async fn start_session(payload: StartSessionPayload) -> Result<SessionResponse, String> {
    // ... business logic ...
    Ok(SessionResponse {
        session_id: uuid::Uuid::new_v4().to_string(),
        state: "Active".to_string(),
    })
}
```

**Translation for you:**
- `#[tauri::command]` is like NestJS's `@Post('/start-session')` decorator — it marks this function as a callable endpoint.
- `#[derive(Deserialize)]` is like a NestJS DTO with validation — Tauri auto-converts incoming JSON to this struct.
- `#[derive(Serialize)]` is for the response — auto-converts the struct to JSON for the frontend.
- `Result<SessionResponse, String>` — Rust's way of saying "this returns either a success value OR an error string". Like `Promise<SessionResponse>` that can throw, but type-safe.

#### Step 2 — Register the command with Tauri

File: `apps/desktop-ui/src-tauri/src/lib.rs`

```rust
mod commands;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::start_session,
            commands::stop_session,
            commands::get_session_state,
            commands::list_profiles,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Translation:** this is like NestJS's `controllers: [SessionController]` — telling the framework "these are the endpoints I want to expose". Anything not listed here can NOT be called from the frontend, even if it exists.

#### Step 3 — Add the permission (Tauri's allowlist)

File: `apps/desktop-ui/src-tauri/capabilities/default.json`

This file says "the main window can call these specific permissions". For our own custom commands, registering them in `invoke_handler` is enough. For built-in Tauri features (notifications, file system, etc.), you have to explicitly grant permission.

This is **important for security** — by default the frontend can't access your filesystem, can't make HTTP requests, can't read clipboard, etc. You opt in to each capability.

#### Step 4 — Call it from the frontend

File: `apps/desktop-ui/src/commands/index.ts`

```typescript
import { invoke } from '@tauri-apps/api/core'

export interface StartSessionPayload {
  profileId: string
  durationSecs: number
  mode: 'Focus' | 'ShortBreak' | 'LongBreak' | 'Strict'
}

export interface SessionResponse {
  sessionId: string
  state: string
}

export function startSession(payload: StartSessionPayload): Promise<SessionResponse> {
  return invoke<SessionResponse>('start_session', { payload })
}
```

**Translation:** this is exactly like a service file in your React apps that wraps `axios.post(...)` — except `invoke()` goes over the IPC bridge instead of HTTP.

#### Step 5 — Use it in a component

File: `apps/desktop-ui/src/pages/Dashboard.tsx`

```tsx
const handleStart = async () => {
  const res = await startSession({
    profileId: 'default',
    durationSecs: 25 * 60,
    mode: 'Focus',
  })
  setSession(res.sessionId, 'Active')
}
```

This looks identical to a fetch call in any React app.

### What Actually Happens at Runtime

```
1. User clicks "Start" button in React component
2. handleStart() runs in JavaScript inside the WebView
3. invoke('start_session', { payload }) is called
4. Tauri's JS runtime serializes the args to JSON
5. JSON goes through the IPC bridge (not HTTP — a binary message channel)
6. Rust side: Tauri deserializes JSON → StartSessionPayload struct
7. Rust calls our start_session function with the deserialized struct
8. Function returns Result<SessionResponse, String>
9. Tauri serializes the response back to JSON
10. JSON goes back over the bridge
11. React's invoke() promise resolves with the parsed response
12. setSession() updates Zustand store, React re-renders
```

Total time: **microseconds**, because there's no network round trip.

### The Three IPC Patterns

| Pattern | Direction | When to use |
|---------|-----------|-------------|
| **Command** (`invoke`) | React → Rust → React (request/response) | "Do this thing and tell me the result" — like REST POST |
| **Event** (`app.emit` / `listen`) | Rust → React (push) | "Tell the UI when something happens" — like WebSocket |
| **State** (`tauri::State`) | Shared between commands | "Multiple commands need access to the same struct" — like a NestJS singleton service |

In our app, we use:
- Commands for `start_session`, `stop_session`, `list_profiles` (request/response)
- Events for "session timer ticked" (the Rust timer pushes a `tick` event every second; React listens and updates the displayed countdown)
- State for the database connection pool and the SessionService (created once at startup, shared by all commands)

---

## Part 4 — System Design Deep Dive

### The Split-Brain Process Model

Your Surfboard backend probably runs as a few microservices: maybe an `auth-service`, a `payments-service`, a `customer-service`. They each have one job and they talk to each other.

We do the same on the desktop, with **3 process roles**:

| Process | Job | Lives where |
|---------|-----|-------------|
| `zen-ui` | Show windows, accept user input, render the UI | The Tauri shell |
| `zen-agent` | Background daemon — owns the session timer, the tray icon, schedules, enforcement | Started at login, stays running |
| `zen-elevated-helper` | Privileged operations (network blocking, deep OS hooks) | Optional, installed with consent |

**Why not just one process?**

Imagine the user force-quits the app mid-session. If everything was in one process, blocking would stop. With separate processes:
- Killing `zen-ui` → just closes the window. The agent keeps blocking.
- Killing `zen-agent` → harder to do (it's a daemon, restarts itself via `launchd` on macOS / Service Control Manager on Windows).
- The helper has the deepest hooks; killing it requires admin rights.

Each process has a **smaller, more specific job**, and the more privileged it is, the smaller and harder to kill.

> This is the same principle as **least privilege** in your fintech work — you don't run your customer-management service as a database admin. You give each service the minimum permissions it needs.

### The Layered Crate Architecture (Like Hexagonal / Clean Architecture)

NestJS encourages a **layered architecture**: Controller → Service → Repository → Database. We do the same in Rust, but enforce it via **separate crates** (NPM packages).

```
┌─────────────────────────────────────────────────────────────┐
│  apps/desktop-ui/src-tauri  (Tauri shell + IPC commands)    │
│  ├ commands.rs ← entry points (like NestJS controllers)     │
└──────────────────────────┬──────────────────────────────────┘
                           │ uses
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        ▼                  ▼                  ▼
┌──────────────┐  ┌────────────────┐  ┌──────────────────┐
│  zen-core    │  │  zen-db        │  │  zen-macos       │
│              │  │                │  │  zen-windows     │
│  Use cases,  │  │  SQLite        │  │  OS adapters     │
│  port traits │◄─┤  repos that    │  │  that implement  │
│              │  │  implement     │  │  zen-os traits   │
│              │  │  zen-core      │  │                  │
│              │  │  port traits   │  │                  │
└──────┬───────┘  └────────┬───────┘  └─────────┬────────┘
       │                   │                    │
       │ depends on        │ depends on         │ depends on
       │                   │                    │
       ▼                   ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│  zen-domain  (entities, FSM, events, policy compiler)       │
│  ZERO IO. ZERO async. PURE LOGIC.                            │
└─────────────────────────────────────────────────────────────┘
```

**Why bother with this separation?**

Because **`zen-domain` has zero IO**, you can:
- Test all the business logic with `cargo test` in **microseconds**, no DB, no network, no OS
- Reason about the FSM and policy logic without thinking about SQL or async at all

This is the same principle as **Hexagonal Architecture** (a.k.a. Ports & Adapters) that's very popular in serious backend systems. NestJS doesn't enforce it but you can do it manually. Rust's compiler enforces module boundaries strictly, so you can't accidentally violate the architecture.

> Compare to your DCC work: the *business logic* of "given a transaction in EUR, the merchant's home currency is SEK, the rate is 11.32, what does the customer see?" should be pure logic — no DB, no API calls. The bank API integration is a separate layer. We're doing the same thing here.

### The Session Finite State Machine (FSM)

A session is an entity with a **state**. Like a payment in your fintech world that goes:

```
Pending → Authorized → Captured → Settled
                    ↘ Voided
                    ↘ Refunded
```

Our session has its own state machine:

```
        Idle
         ↓ (user clicks Start)
       Preparing
         ↓ (enforcement applied)
        Active ◄─────────┐
         ↓                │ resume
       Paused ────────────┘
         ↓
       Completing
         ↓
       Completed

  + at any point: Aborted (with reason)
  + after a crash: Recovering
```

**Why an FSM?**

Because illegal transitions are silently a huge source of bugs. In a payment system, you'd never want to "Capture" a "Voided" transaction. The FSM enforces "you can only go from state X to state Y under condition Z". Anything else returns an error.

Our `SessionFsm` (in `crates/zen-domain/src/session.rs`) is just a struct with methods like `request_start()`, `start()`, `pause()`, `complete()`, `abort()`. Each method returns either a `DomainEvent` (success) or `DomainError::InvalidTransition` (failure).

### Event Sourcing (You'll Recognize This Pattern)

Every state change writes an immutable event to the `session_events` table BEFORE anything else happens:

```sql
INSERT INTO session_events (session_id, event_type, payload, occurred_at)
VALUES ('abc-123', 'SessionStarted', '{...}', '2026-04-29 12:00:00')
```

This is the **same pattern** as a financial transaction journal in payments. The events are:
- `SessionRequested`
- `SessionPreparationStarted`
- `EnforcementApplied`
- `SessionStarted`
- `SessionPaused`
- `SessionResumed`
- `SessionCompleted`
- `SessionAborted`
- `CrashRecovered`
- `TamperDetected`

**The magic:** you can rebuild any session's complete state by replaying its events. If the app crashes, on next boot we read the events, figure out what state we were in, and resume.

> In your DCC work, you probably have audit logs of "rate quoted at 12:00, customer accepted at 12:01, transaction settled at 12:05". Same idea — the log IS the source of truth, the current state is just a snapshot derived from it.

### Local-First (Why No Cloud?)

Your Surfboard backend lives in the cloud because customers connect from anywhere. Different here:

- The user's *focus session* must work even on a plane / in a tunnel / when the WiFi dies
- The user's *blocked apps and websites* are sensitive — selling that data would be terrible PR
- A cloud round-trip adds 100ms+ latency to every block decision; we need <1ms

So we use **SQLite as the system of record**, on the user's machine. No server. The optional sync layer (Phase 3) adds cross-device sync as an *encrypted append-only log on top* — but the local copy is always authoritative.

### Crash Recovery (The Defining Hard Problem)

This is the trick that makes Zen Mode actually reliable:

```
1. App is running, session is active, blocking is in effect.
2. User force-quits the app (or the app crashes, or the OS reboots).
3. App relaunches.
4. BEFORE showing any UI, the recovery path runs:
   a. Open SQLite, find any session with state=Active or state=Recovering
   b. Read the SessionStarted event to find planned_duration and started_at
   c. Compute remaining = planned_duration - (now - started_at)
   d. If remaining > 0:
      - Mark state = Recovering
      - Reapply the enforcement plan to the OS (re-block apps, re-block domains)
      - Emit CrashRecovered event
      - Mark state = Active
   e. Only THEN show the UI
5. From the user's perspective: the timer and blocking continued seamlessly.
```

> This is the same kind of **idempotent recovery** logic as a payment retry handler. If the customer hits "Pay" twice, you don't double-charge. If the app crashes during a session, you don't double-start or skip the session.

---

## Part 5 — Each Crate Explained (with NestJS analogies)

A "crate" is just a Rust package, the equivalent of an NPM package. Our project has 9 of them in `crates/`, each with one job.

### `zen-domain` — The Pure Domain Layer

**NestJS analogy:** the entities/DTOs/business rules folder, the part that doesn't import any database or framework code.

| File | What's in it | Why |
|------|--------------|-----|
| `entities.rs` | `Session`, `Profile`, `BlockRule`, `EnforcementPlan` data types | The shapes of our domain objects |
| `session.rs` | `SessionFsm` — the state machine | Enforces valid state transitions |
| `events.rs` | `DomainEvent` enum — every possible event | The vocabulary of state changes |
| `policy.rs` | `PolicyCompiler` — turns a profile into a plan | Pure transformation logic |
| `errors.rs` | `DomainError` enum | Typed errors, not strings |

**Rule:** this crate has ZERO IO. No DB. No network. No async. No file reads. This is what makes it 100% unit-testable in microseconds.

### `zen-core` — Use Cases & Ports

**NestJS analogy:** your services folder. The "verb" layer — `startSession()`, `stopSession()`. Plus the interfaces (called "ports") that say what the system needs from the outside world.

| File | What's in it | Why |
|------|--------------|-----|
| `ports.rs` | `SessionRepository`, `ProfileRepository`, `BlockingAdapter` traits | "Here's what I need; someone else implements it" |
| `session_service.rs` | `SessionService::start_session()` etc. | The orchestration |
| `policy_engine.rs` | `PolicyEngine` — fetches profile, calls compiler | A thin coordinator |

`zen-core` never imports `zen-db` or any platform crate. It only knows about traits. This is **dependency inversion** — the use case doesn't care if the repository is SQLite, PostgreSQL, or an in-memory fake (which is what we use in tests).

### `zen-db` — SQLite Implementation

**NestJS analogy:** your TypeORM/Drizzle repository implementations.

| File | What's in it |
|------|--------------|
| `connection.rs` | Open SQLite pool, configure WAL mode |
| `migrations.rs` | Run migration files at startup |
| `migrations/001_initial.sql` | The 11-table schema |
| `session_repo.rs` | `SqliteSessionRepository` — implements `SessionRepository` trait from `zen-core` |
| `profile_repo.rs` | `SqliteProfileRepository` — same idea |

We use `sqlx`, which is the most popular Rust SQL library — like Drizzle or TypeORM but with **compile-time SQL checking** (the SQL gets validated against your schema at build time, not runtime).

### `zen-os` — OS Abstraction Traits

**NestJS analogy:** like an interface for "PaymentProvider" that you can implement with Stripe, Adyen, etc. — except here the implementations are platform-specific (macOS vs Windows).

| File | What's in it |
|------|--------------|
| `adapters.rs` | `AppBlockingAdapter`, `NetworkBlockingAdapter` traits |
| `process.rs` | `ProcessMonitor` trait + `ProcessEvent` types |
| `permissions.rs` | `PermissionProbe` trait |
| `network.rs` | `DnsRule` types |

### `zen-macos` and `zen-windows` — Platform Adapters

**NestJS analogy:** if you had a `MacOSPaymentProvider` and a `WindowsPaymentProvider` implementing the same `PaymentProvider` interface — except for OS-level things instead of payment APIs.

These crates are conditionally compiled (`#[cfg(target_os = "macos")]` only compiles the code on macOS). On macOS, `zen-windows` is empty; on Windows, `zen-macos` is empty.

For each platform, we'll eventually implement:

**macOS:**
- App blocking: `NSWorkspace` notifications + Accessibility API + process termination
- Network blocking: Local DNS proxy (intercept domain queries)
- Permissions: `AXIsProcessTrusted()` to check Accessibility access

**Windows:**
- App blocking: Win32 process creation events via `CreateToolhelp32Snapshot` or ETW
- Network blocking: Windows Filtering Platform (WFP) — same tech Windows Defender uses
- Permissions: Service Control Manager status

Currently both are stubs that just `tracing::info!()` what they would do.

### `zen-sync` — Optional Cross-Device Sync (Phase 3)

**NestJS analogy:** like the multi-cloud Redis cache population system you built at Surfboard, except E2E encrypted and using a CRDT-style merge log.

The plan:
- Every change creates a `SyncOperation` with a Lamport timestamp (a logical clock that handles ordering across devices)
- Operations are encrypted client-side before being uploaded
- The server is "dumb" — it just stores ciphertext blobs
- Other devices download the operations, decrypt locally, and merge

Currently a skeleton — types defined, no implementation yet.

### `zen-telemetry` — Privacy-Safe Metrics

Disabled by default. If the user opts in, sends only aggregate metrics (sessions started, crash counts) — never domain names visited or app names blocked. Consent-gated on first run.

### `zen-cli` — Internal Diagnostic Tool

Like a Postman collection but for the developer. Run `cargo run --bin zen-cli -- db active-session` to inspect the local DB. Useful during development and for support.

---

## Part 6 — Database Design

### Why SQLite, Not PostgreSQL?

You're a Postgres expert. SQLite is similar but different in important ways:

| Feature | PostgreSQL | SQLite |
|---------|-----------|--------|
| Architecture | Client-server (separate process) | Embedded (linked into your app) |
| Storage | Server's disk | A single file on the user's machine |
| Connections | Network sockets | Function calls |
| Concurrent writes | Many | One at a time |
| Concurrent reads | Unlimited | Many (with WAL mode) |
| When to use | Multi-user, network app | Single-user local data |

For Zen Mode, SQLite is perfect: there's exactly one user (the desktop user), and we want zero setup (no "install Postgres first"). The whole DB is a file like `~/.zen-mode/zen.db`.

### WAL Mode — The PostgreSQL Equivalent

By default SQLite locks the entire file when you write. We enable **WAL mode** (Write-Ahead Logging):
- Writes go to a separate log file
- Readers can keep reading the main DB while writes happen
- The log periodically "checkpoints" back into the main DB

This is conceptually similar to **PostgreSQL's MVCC** — readers and writers don't block each other.

### The 11 Tables

```sql
-- User configuration
profiles            -- named rule bundles ("Deep Work", "Focus", etc.)
block_rules         -- individual app/domain entries per profile
schedules           -- recurring focus times ("every weekday 9-12")

-- Session state and event log
sessions            -- each focus session, current state
session_events      -- ★ append-only event log (source of truth)
session_snapshots   -- periodic compaction of events for fast recovery

-- System state
device_state        -- random key/value (last open profile, etc.)
permission_state    -- snapshot of OS permission health
enforcement_state   -- which plan revision is currently applied

-- Tamper detection
tamper_events       -- log of detected circumvention attempts

-- Future: cross-device sync
sync_ops            -- encrypted operation log for sync
```

The **star pattern** here: `session_events` is the event log; `sessions` and `session_snapshots` are derived from it. This is event sourcing applied to a desktop app.

> Compare to your DCC work: you probably have a `transactions` table (current state) and a `transaction_events` table (every status change). The events are the truth; the row is a cache.

---

## Part 7 — Frontend Architecture

You know React. Here's what's specifically different:

### The Four State Layers

Most React apps confuse state. We split it explicitly:

| Layer | Library | What it holds | Example |
|-------|---------|---------------|---------|
| **Routing** | TanStack Router | URL/page state | Which screen am I on? |
| **Server data** | TanStack Query | Cached results from IPC commands | "Profiles list, fetched 3 seconds ago" |
| **Live state** | Zustand | Real-time stream from Rust events | "Current timer: 23:45" |
| **Form drafts** | React Hook Form + Zod | In-progress edits | "User is typing a new profile name" |

This matches the patterns from professional React codebases. It avoids the "everything in Redux" mess.

### The Data Flow for a Session Tick

```
[Every 1 second on the Rust side]
  ↓
Rust agent computes remaining time (monotonic clock!)
  ↓
app.emit("session:tick", { remaining_secs: 1485 })
  ↓
React useEffect with listen("session:tick", handler)
  ↓
useSessionStore().updateRemaining(1485)
  ↓
React re-renders ONLY the components subscribed to remainingSecs
```

Important subtlety: **the timer is computed on the Rust side**, not in JavaScript. Why? Because JavaScript's `setInterval` is unreliable (drifts under heavy CPU, paused when window is hidden). Rust uses the OS's **monotonic clock** which keeps ticking accurately even during sleep/wake.

### Why Tauri Events Instead of "Just polling"

You could have React do `setInterval(() => invoke('get_remaining'), 1000)` — but then:
- The frontend is the one polling (drift)
- 1000ms = 1 second of latency for any event
- Battery drain

Events flip the direction: **the Rust side pushes updates**. React just listens. This is like the difference between long-polling and WebSockets in a chat app.

---

## Part 8 — The Implementation Plan (What's Done, What's Next)

### Phase 0 — Foundations ✅ (DONE)

You're here right now. We have:

- ✅ Monorepo set up (Cargo workspace + pnpm workspace)
- ✅ All 9 Rust crates created with proper dependencies
- ✅ Tauri 2 shell that boots
- ✅ Domain layer with FSM and policy compiler (4 unit tests passing)
- ✅ SQLite schema (11 tables) + migrations
- ✅ IPC commands wired (stubs)
- ✅ React UI with 3 pages (Dashboard, Profiles, Settings)
- ✅ CI pipeline set up

### Phase 1 — MVP (NEXT — ~6-10 weeks)

Goal: a single-device blocker that actually works.

**Order of work:**

1. **Wire up real DB writes**
   - File: `crates/zen-db/src/session_repo.rs`
   - Implement `save()`, `find_by_id()`, `find_active()`, `append_event()` using `sqlx::query!()` macros
   - Test with a real test SQLite file

2. **Wire up profile CRUD**
   - File: `crates/zen-db/src/profile_repo.rs`
   - Implement profile + block_rules CRUD
   - Add a `seed.sql` for the default "Deep Work" profile

3. **Wire SessionService into the Tauri commands**
   - File: `apps/desktop-ui/src-tauri/src/lib.rs`
   - Build a `tauri::State` that holds `Arc<SessionService>` constructed at startup
   - Update `commands.rs` to call the service instead of returning fake data

4. **Build the macOS app blocker** (since you're on macOS)
   - File: `crates/zen-macos/src/app_blocker.rs`
   - Use the `objc2` crate to bind to `NSWorkspace`
   - Subscribe to `NSWorkspaceDidLaunchApplicationNotification`
   - When a blocked bundle ID launches, terminate it via `NSRunningApplication terminate:`

5. **Build the macOS DNS proxy for website blocking**
   - File: `crates/zen-macos/src/dns_proxy.rs`
   - Run a local DNS resolver on `127.0.0.1:5353`
   - Reconfigure the system resolver to use it via `scutil` or Network Extension
   - Block listed domains by returning `NXDOMAIN`

6. **Implement the timer engine**
   - New file: `crates/zen-core/src/timer.rs`
   - Use `tokio::time::interval` with monotonic timing
   - Emit `session:tick` events to the frontend every second

7. **Implement crash recovery**
   - On app start, before `tauri::Builder` runs the UI, check for active sessions in DB and reapply
   - File: `apps/desktop-ui/src-tauri/src/lib.rs` setup hook

8. **Build the actual UI for profile editing**
   - File: `apps/desktop-ui/src/pages/Profiles.tsx`
   - Form to add/remove apps and domains
   - Wire to React Hook Form + Zod for validation
   - Use TanStack Query mutations to persist changes

9. **Add the tray icon and notifications**
   - Use Tauri's tray icon API (already configured in `tauri.conf.json`)
   - Show "25 minutes remaining" in the menu bar
   - Notification when session completes

10. **Sign + notarize a dev build**
    - Get an Apple Developer ID
    - Configure Tauri's bundler to sign the build
    - First end-to-end install test on a real machine

### Phase 2 — Hardening (8-12 weeks after MVP)

- Windows WFP-based network blocking (writing actual Windows networking code)
- Reconciliation loop: every 10 seconds, compare desired enforcement state vs actual, re-apply if drift detected
- Tamper detection: clock rollback, helper killed, permission revoked → emit `TamperDetected` event
- Strict mode: cancellation requires a cooldown / reason / secondary auth

### Phase 3 — Sync (8-14 weeks)

- E2EE sync server (could be a small backend in TypeScript on Cloud Run — your wheelhouse!)
- Browser extensions for Chrome/Edge/Firefox (precise per-tab blocking)
- Focus analytics

### Phase 4 — Production

- Code signing pipeline in CI
- Auto-updater (Tauri has a plugin)
- Crash reporting
- Beta cohort, staged rollouts, support runbooks

---

## Part 9 — Learning Path & Resources

You don't need to learn all of Rust. You need a working subset. Here's the order:

### Week 1 — Rust Basics

**Read:**
- The first 4 chapters of [The Rust Book](https://doc.rust-lang.org/book/) — variables, types, ownership
- Chapter 6 — enums and pattern matching (this is a superpower compared to TS)
- Chapter 10.2 — traits

**Then read our code in this order:**
1. `crates/zen-domain/src/errors.rs` (simplest enum)
2. `crates/zen-domain/src/entities.rs` (data types)
3. `crates/zen-domain/src/events.rs` (richer enum with data per variant)
4. `crates/zen-domain/src/session.rs` (pattern matching + state machine)
5. `crates/zen-domain/src/policy.rs` (functions that transform data)

### Week 2 — Async Rust

**Read:**
- The Rust Book Chapter 13 (closures + iterators)
- The first half of [Tokio's tutorial](https://tokio.rs/tokio/tutorial)

**Key insight:** Rust async is similar to JS async, but with one annoying difference — `Future`s are lazy. They don't run until you `.await` them. In JS, `Promise` runs immediately when constructed.

**Then read:**
- `crates/zen-core/src/ports.rs` (`async_trait` traits — like NestJS interfaces with async methods)
- `crates/zen-core/src/session_service.rs` (orchestrating async calls)

### Week 3 — Tauri Specifically

**Read:**
- [Tauri 2 Quick Start](https://tauri.app/start/)
- [Tauri's IPC docs](https://tauri.app/concept/inter-process-communication/) — this is the most important page for you

**Then read:**
- `apps/desktop-ui/src-tauri/src/commands.rs` (with the IPC docs open in another tab)
- `apps/desktop-ui/src-tauri/src/lib.rs` (the `tauri::Builder` setup)
- `apps/desktop-ui/src-tauri/tauri.conf.json` (config you might tweak)

### Week 4 — sqlx and the Database Layer

**Read:**
- [sqlx README](https://github.com/launchbadge/sqlx)

**Then implement:**
- A real `SqliteSessionRepository::save()` (currently a stub)
- Your first `sqlx::query!()` call
- Run it via `cargo run --bin zen-cli`

By this point you'll be productive. The macOS-specific stuff (Phase 1, step 4-5) is the steepest curve because you'll use `objc2` to talk to Apple's frameworks — that's another world entirely. Save it for last.

---

## Part 10 — TL;DR for When You Re-Read This

1. **Tauri = Rust backend + React frontend in one binary** (no Chromium, no server, ~10 MB)
2. **IPC commands** are like REST endpoints but in-process (no HTTP). React calls `invoke('command_name', args)`, Rust function runs, returns result.
3. **Crate split** enforces clean architecture: `zen-domain` is pure logic, `zen-core` is use cases + interfaces, `zen-db`/`zen-macos`/`zen-windows` are implementations.
4. **Event sourcing** in SQLite — every state change writes an event. Crash recovery replays events.
5. **Split-brain processes** — UI is killable, agent isn't. Like microservices but on the desktop.
6. **Local-first** — everything works offline. Sync is optional and E2E encrypted.
7. **Phase 0 done; Phase 1 = wire real DB + macOS app blocker + DNS proxy + timer engine + UI for profiles.**

When in doubt: read the file paths in this doc, in the order listed, and you'll get a complete mental model.
