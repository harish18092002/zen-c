# ADR 001: Local-First Architecture

**Status**: Accepted
**Date**: 2026-04-27

## Context

Zen Mode's core value is focus enforcement. This must never fail because of network loss,
server downtime, or sync latency. A cloud-dependent architecture would contradict the
product's reliability promise.

## Decision

All session state, blocking rules, profiles, and enforcement logic run entirely on-device.
SQLite is the system of record. Sync is an optional encrypted overlay added post-MVP.

## Consequences

- Blocking works offline and during network outages — always.
- Privacy defaults are stronger: sensitive behavioral data never leaves the device.
- Timer and enforcement are deterministic; no cloud round-trips on the critical path.
- Sync complexity is deferred until single-device reliability is proven.
- Cross-device sync is architecturally possible (append-only encrypted op log), just not required initially.

## Alternatives considered

**Cloud-first with local cache**: Rejected. Adds network dependency to the critical
enforcement path and significantly increases privacy risk.

**Hybrid with required account**: Rejected for MVP. Gates value on onboarding friction
and breaks the offline guarantee.
