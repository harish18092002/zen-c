# Zen Mode

A **local-first, tamper-aware, cross-platform focus application** with a Rust-native enforcement core, Tauri 2 desktop shell, and a React/TypeScript UI.

Built around one principle: **when you commit to a focus session, the app keeps that commitment even when your distracted self doesn't want it to.**

---

## What is Zen Mode?

Zen Mode is a desktop productivity app in the tradition of Cold Turkey and Freedom, rebuilt from scratch with a modern, maintainable architecture. It blocks distracting apps and websites during timed focus sessions, with enforcement that survives UI crashes, process kills, and permission changes.

### Core properties

| Property | Description |
|----------|-------------|
| **Local-first** | All enforcement runs on-device. No internet required, ever. |
| **Tamper-aware** | Blocking survives UI crash, force-quit, clock change, and process kill. |
| **Crash-recoverable** | Active sessions are reconstructed from persisted events on restart. |
| **Privacy-safe** | No visited domains, app names, or session labels leave the device by default. |
| **Cross-platform** | macOS and Windows with equivalent enforcement depth. |

---

## How It Works

### Session lifecycle

When you click **Start Focus Session**:

1. The React UI validates your input and calls a typed Tauri IPC command (`start_session`).
2. The **Rust core** (`zen-core`) receives the command and drives the **Session FSM**:
   - `Idle → Preparing → (EnforcementApplied) → Active`
3. The **Policy Engine** compiles your block profile into an **EnforcementPlan** — a versioned, hashable set of app rules, domain rules, and network rules.
4. **Platform adapters** (`zen-macos` / `zen-windows`) apply the plan to the OS:
   - macOS: NSWorkspace observer + Accessibility API + DNS proxy
   - Windows: Process creation monitor + Windows Filtering Platform (WFP)
5. The **Timer Engine** starts a monotonic countdown. The countdown is derived from elapsed monotonic time, not tick decrements — it survives sleep/resume cycles correctly.
6. Every state transition is appended as an **immutable domain event** to SQLite. Periodic snapshots compact the event log.
7. The Rust core emits state events to the frontend via Tauri's typed event bus.
8. The **Zustand store** patches UI state; React re-renders only the affected components.

### What happens if you force-quit the app

1. On next launch, the **Recovery path** runs before any UI is shown.
2. The core reads the last `Active` session snapshot from SQLite.
3. It validates the monotonic/wall-clock delta for clock-rollback tamper detection.
4. It reapplies the enforcement plan to the OS.
5. It recomputes remaining time and emits a `CrashRecovered` event.
6. The UI shows the active session — blocking was never interrupted.

### What happens if you try to bypass it

The app uses layered enforcement and tamper visibility, not a claim of absolute unbreakability:

- Enforcement lives in the background agent, not the UI process.
- All binaries are signed and validated at handshake time.
- A heartbeat table in SQLite and in-memory watchdog timers detect unexpected stops.
- Divergence between desired and actual enforcement state triggers a `TamperDetected` event and re-application.
- In **Strict Mode**, session cancellation requires a cooldown, reason code, or secondary authentication.

---

## Architecture

### Process model

```
zen-ui (Tauri shell + React UI)
  ↕ IPC
zen-agent (background daemon — enforcement, timers, tray, schedules)
  ↕ privileged IPC
zen-elevated-helper (optional — deep network/OS integration)
```

### Crate map

```
crates/
  zen-domain/        Pure domain layer: entities, FSM, events, policy compiler. Zero IO.
  zen-core/          Use-case orchestration, port traits (repository/adapter interfaces).
  zen-db/            SQLite implementations of zen-core ports (sqlx + WAL).
  zen-os/            Shared OS abstraction traits: AppBlockingAdapter, NetworkBlockingAdapter, etc.
  zen-macos/         macOS enforcement adapters: NSWorkspace, Accessibility, DNS proxy.
  zen-windows/       Windows enforcement adapters: WFP, process monitor, service.
  zen-sync/          Optional E2EE sync protocol and crypto envelope types.
  zen-telemetry/     Privacy-safe optional telemetry client (disabled by default).
  zen-cli/           Internal diagnostic CLI for inspecting DB and simulating sessions.
```

### Dependency flow

```
zen-domain  ←  zen-core  ←  zen-db
     ↑               ↑
zen-os  ←  zen-macos      (macOS only)
zen-os  ←  zen-windows    (Windows only)

apps/desktop-ui/src-tauri  ←  zen-core, zen-db
```

`zen-domain` has zero IO. `zen-core` never imports `zen-db` or any platform crate — it only defines traits. This means the entire use-case layer is unit-testable without a database or real OS.

### Data model

```sql
profiles            -- named rule bundles, versioned on edit
block_rules         -- app/domain/category/network entries per profile
schedules           -- recurring or one-shot focus windows
sessions            -- active and historical focus intervals
session_events      -- append-only event log (source of truth)
session_snapshots   -- compacted state snapshots for fast recovery
enforcement_state   -- which plan revision is currently applied
tamper_events       -- log of detected circumvention attempts
permission_state    -- per-permission health snapshots
sync_ops            -- optional encrypted cross-device operation log
```

### Frontend state layers

| Layer | Tool | Scope |
|-------|------|-------|
| Remote data and mutations | TanStack Query | Profiles, history, settings |
| Live session stream | Zustand | Real-time FSM state from backend events |
| Form drafts | React Hook Form + Zod | Settings and profile editing |
| URL/navigation | TanStack Router | Page routing |

---

## Technology Stack

| Concern | Choice | Why |
|---------|--------|-----|
| Desktop shell | Tauri 2 | OS WebView (no bundled Chromium), strong security model, small footprint |
| Core language | Rust stable | Timer correctness, OS API ergonomics, no GC pauses in enforcement paths |
| UI framework | React 19 + TypeScript 5.6 | Fastest path to polished settings-heavy desktop UX |
| Build tool | Vite 6 | Fast HMR, ESM-native |
| Routing | TanStack Router | Fully type-safe file-based routing |
| Data fetching | TanStack Query | Cache, stale-while-revalidate, mutations |
| UI state | Zustand | Minimal, no boilerplate |
| Forms | React Hook Form + Zod | Performant, typed validation |
| Database | SQLite via sqlx | WAL mode, migrations, fully local |
| Async runtime | Tokio | Standard Rust async |
| Time | `time` crate + monotonic sources | Correct timer arithmetic |
| Tracing | `tracing` + `tracing-subscriber` | Structured logs, rolling file appender |
| Package manager | pnpm | Workspaces, fast, disk-efficient |

---

## Repository Structure

```
zen-c/
  apps/
    desktop-ui/              Tauri 2 app
      src/                   React + TypeScript frontend
        commands/            Typed Tauri IPC wrappers
        pages/               Route-level page components
        store/               Zustand stores
      src-tauri/             Rust Tauri backend
        src/
          commands.rs        IPC command handlers
          lib.rs             App setup, plugin registration
        capabilities/        Tauri capability definitions
        tauri.conf.json      App config
  crates/
    zen-domain/              Domain entities, FSM, events, policy compiler
    zen-core/                Use cases, port traits
    zen-db/                  SQLite repositories, migrations
    zen-os/                  OS abstraction traits
    zen-macos/               macOS enforcement adapters
    zen-windows/             Windows enforcement adapters
    zen-sync/                Sync protocol and E2EE crypto
    zen-telemetry/           Telemetry client
    zen-cli/                 Diagnostic CLI tool
  packages/
    ui-contracts/            TypeScript types mirroring Rust domain contracts
    design-system/           Design tokens and shared components
  native/
    macos-helper/            Future: Swift/XPC helper for privileged macOS operations
    windows-service/         Future: Windows service host for WFP integration
  infra/
    signing/                 Code signing scripts and CI config
    update-feeds/            Update channel manifests
    installer/               Installer build scripts
  docs/
    architecture/            Architecture overview and diagrams
    adr/                     Architecture Decision Records
  scripts/
    setup.sh                 Developer environment bootstrap
  .github/
    workflows/
      ci.yml                 CI pipeline (lint, test, build)
```

---

## Getting Started

### Prerequisites

- **Rust** (stable, 1.86+) — install via [rustup](https://rustup.rs)
- **Node.js** (22+) and **pnpm** (9+)
- **Tauri prerequisites** — see [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/)
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft C++ Build Tools, WebView2

### Bootstrap

```bash
# Clone and enter the repo
git clone <repo-url> zen-c && cd zen-c

# Run the setup script (installs deps, checks toolchains)
bash scripts/setup.sh
```

### Run in development

```bash
pnpm dev
```

This starts the Vite dev server and the Tauri shell in hot-reload mode.

### Run Rust tests

```bash
cargo test --workspace
```

### Use the diagnostic CLI

```bash
cargo run --bin zen-cli -- --help
cargo run --bin zen-cli -- db active-session
cargo run --bin zen-cli -- permissions
cargo run --bin zen-cli -- simulate --duration-mins 25
```

### Build for distribution

```bash
pnpm build
```

This produces a signed app bundle in `apps/desktop-ui/src-tauri/target/release/bundle/`.

---

## Development Guide

### Working on the domain layer

All business logic starts in `crates/zen-domain`. The rule: no IO, no async, fully unit-testable.

```bash
cargo test -p zen-domain
```

The session FSM is in `crates/zen-domain/src/session.rs`. Every valid state transition produces a `DomainEvent` — invalid transitions return `DomainError::InvalidTransition`.

### Adding a new IPC command

1. Add the Rust handler in `apps/desktop-ui/src-tauri/src/commands.rs`.
2. Register it in `apps/desktop-ui/src-tauri/src/lib.rs` inside `generate_handler![]`.
3. Add the TypeScript wrapper in `apps/desktop-ui/src/commands/index.ts`.
4. Update the shared type contracts in `packages/ui-contracts/src/index.ts`.

### Adding a new block rule kind

1. Add the variant to `BlockRuleKind` in `crates/zen-domain/src/entities.rs`.
2. Handle it in `PolicyCompiler::compile` in `crates/zen-domain/src/policy.rs`.
3. Implement the enforcement logic in the relevant platform adapter (`zen-macos` or `zen-windows`).
4. Add the new type to `packages/ui-contracts/src/index.ts`.

### Database migrations

Add new migration files in `crates/zen-db/migrations/` using the naming convention `00N_description.sql`. Migrations are forward-only; never modify existing migration files.

---

## Testing

### Test pyramid

| Level | Tool | What it covers |
|-------|------|---------------|
| Unit | `cargo test` | FSM, policy compiler, domain normalization, schedule math |
| Property | `proptest` | Timer arithmetic, rule compilation edge cases |
| Integration | `cargo test` with SQLite | Repository behavior, DB migrations |
| Platform | Manual + harness scripts | OS adapter health, permission state, process detection |
| End-to-end | Signed test builds | Full session lifecycle on real macOS / Windows |

### Blocking reliability matrix (manual)

Before any release:

- [ ] Start session, force-quit UI, confirm enforcement persists
- [ ] Start session, reboot machine, confirm session recovers
- [ ] Change system clock backward during active session
- [ ] Revoke Accessibility permission mid-session
- [ ] Launch blocked app repeatedly (burst test)
- [ ] Enable VPN / custom DNS during session
- [ ] Upgrade app during active strict session

---

## Phased Roadmap

| Phase | Goal | Key deliverables |
|-------|------|-----------------|
| **0 — Foundations** | Architecture skeleton, DB, FSM | Monorepo, Tauri shell boots, SQLite schema, session FSM with tests |
| **1 — MVP** | Single-device blocker | Timer, profiles, app + website blocking, crash recovery, tray |
| **2 — Hardening** | Cross-platform parity, reliability | WFP on Windows, AX/NSWorkspace on macOS, reconciliation loop, signed builds |
| **3 — Advanced** | Sync, browser integration, analytics | E2EE sync, browser extensions, focus streaks, preset categories |
| **4 — Production** | Release ops, QA, distribution | Signed release feeds, upgrade testing, support tooling |

---

## License

Proprietary. All rights reserved.
