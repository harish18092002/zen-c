# ADR 002: Tauri 2 + Rust Core + React/TypeScript UI

**Status**: Accepted
**Date**: 2026-04-27

## Context

We need a cross-platform desktop shell with deep OS integration for enforcement,
a small footprint, strong security primitives, and a fast path to a polished UI.

## Decision

- **Shell**: Tauri 2 (uses OS WebView, not bundled Chromium)
- **Core + enforcement**: Rust stable (type safety, systems access, no GC pauses in enforcement paths)
- **UI**: React 19 + TypeScript 5.6+ + Vite 6
- **Routing**: TanStack Router
- **Data**: TanStack Query + Zustand
- **Forms**: React Hook Form + Zod

## Consequences

- Bundle is significantly smaller than Electron (no bundled Chromium)
- Rust handles enforcement correctness and OS integration without depending on a JS runtime
- Tauri 2's capabilities/scopes model gives a strong security boundary between UI and privileged code
- Requires Rust competency in addition to TypeScript
- macOS/Windows WebView rendering may differ slightly from a unified Chromium baseline

## Alternatives considered

| Option | Reason rejected |
|--------|----------------|
| Electron | Much larger bundle; higher idle RAM; no benefit over Tauri for this use case |
| Flutter Desktop | Excellent UI but weaker OS enforcement ergonomics than Rust/Tauri |
| Rust + Wry/Tao directly | Maximum flexibility but much higher plumbing cost |
| Qt | License complexity; heavier runtime |
