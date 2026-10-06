// Wire types shared between the Rust backend (`apps/desktop-ui/src-tauri`) and
// the React frontend. Keep these in lockstep with `dto.rs` — every field name
// and string variant must match exactly.

export type SessionMode = 'Focus' | 'ShortBreak' | 'LongBreak' | 'Strict'

export type Strictness = 'Normal' | 'Strict'

export type AbortReason =
  | 'UserRequested'
  | 'PermissionRevoked'
  | 'SystemShutdown'
  | 'TimerCorrupted'

export type SessionStateKind =
  | 'Idle'
  | 'Preparing'
  | 'Active'
  | 'Paused'
  | 'Completing'
  | 'Completed'
  | 'Aborted'
  | 'Recovering'

export type SessionState =
  | { kind: 'Idle' }
  | { kind: 'Preparing' }
  | { kind: 'Active'; startedAt: string; endsAt: string }
  | { kind: 'Paused'; remainingSecs: number }
  | { kind: 'Completing' }
  | { kind: 'Completed' }
  | { kind: 'Aborted'; reason: AbortReason }
  | { kind: 'Recovering' }

export interface Session {
  id: string
  profileId: string
  mode: SessionMode
  state: SessionState
  plannedDurationSecs: number
  createdAt: string
}

export type BlockRuleKind = 'App' | 'Domain' | 'Category' | 'Network'

export interface BlockRule {
  id: string
  kind: BlockRuleKind
  target: string
  enabled: boolean
}

export interface BlockRuleInput {
  kind: BlockRuleKind
  target: string
  enabled: boolean
}

export interface Profile {
  id: string
  name: string
  revision: number
  blockRules: BlockRule[]
  createdAt: string
  updatedAt: string
}

export interface ProfileUpsertRequest {
  id?: string
  name: string
  blockRules: BlockRuleInput[]
}

export interface StartSessionRequest {
  profileId: string
  durationSecs: number
  mode: SessionMode
  strictness?: Strictness
}

export interface StartSessionResult {
  sessionId: string
  startedAt: string
  endsAt: string
  planRevision: number
  planHash: string
}

export type TimerStateString = 'Running' | 'Paused' | 'Completed' | 'Idle'

export interface TimerTick {
  sessionId: string
  remainingSecs: number
  elapsedSecs: number
  plannedSecs: number
  state: TimerStateString
}

export type PermissionStatusString =
  | 'Healthy'
  | 'Degraded'
  | 'Unavailable'
  | 'PermissionDenied'

export interface Permission {
  name: string
  status: PermissionStatusString
  detail?: string
}

export interface TamperRecord {
  eventType: string
  detail?: string
  detectedAt: string
}
