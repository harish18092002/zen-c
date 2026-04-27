# Architecture Overview

## Split-brain desktop model

Zen Mode uses three executable roles:

| Role | Description |
|------|-------------|
| `zen-ui` | Tauri desktop shell — windows, settings, history, onboarding |
| `zen-agent` | Background agent — schedules, session state machine, tray/menu bar, enforcement coordination |
| `zen-elevated-helper` | Optional privileged component for deep network/OS enforcement |

## Crate dependency graph

```
zen-domain  (pure domain logic, zero IO)
    │
    ├── zen-core  (use cases, ports/traits, policy engine)
    │       │
    │       └── zen-db  (SQLite implementations of ports)
    │
    ├── zen-os  (shared OS adapter traits)
    │       │
    │       ├── zen-macos  (macOS enforcement adapters)
    │       └── zen-windows  (Windows enforcement adapters)
    │
    ├── zen-sync  (E2EE sync protocol, crypto)
    ├── zen-telemetry  (optional telemetry client)
    └── zen-cli  (diagnostic CLI)

apps/desktop-ui/src-tauri  (Tauri shell, wires everything together)
```

## Module boundary rules

- `zen-domain`: zero IO, no async, fully unit-testable
- `zen-core`: orchestrates use cases through traits (ports), no OS assumptions
- `zen-db`: persistence only — the SQLite implementation of `zen-core` ports
- `zen-macos` / `zen-windows`: impure OS adapters, platform-specific, no business logic
- `apps/desktop-ui`: no business logic beyond form validation and view state

## Data flow

```
[User action in UI]
  → Tauri IPC command
  → SessionService (zen-core)
  → PolicyCompiler (zen-domain)
  → BlockingAdapter apply (zen-macos / zen-windows)
  → SessionRepository save (zen-db)
  → Domain event appended
  → State emitted to frontend via Tauri event
  → Zustand store patched
  → React re-renders
```

## Timer correctness

- Countdown derived from monotonic elapsed time, never from tick decrements
- Wall clock used only for schedule windows and UI display
- On sleep/resume: `remaining = planned_duration - monotonic_elapsed`
- On crash: reconstruct from `SessionStarted` event + last snapshot + current clock pair
