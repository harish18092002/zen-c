// TypeScript types that mirror the Rust domain contracts.
// These are kept in sync manually until specta-based code generation is wired up.

export type SessionMode = 'Focus' | 'ShortBreak' | 'LongBreak' | 'Strict'

export type SessionStateKind =
  | 'Idle'
  | 'Preparing'
  | 'Active'
  | 'Paused'
  | 'Completing'
  | 'Completed'
  | 'Aborted'
  | 'Recovering'

export interface ActiveSessionState {
  startedAt: string
  endsAt: string
}

export interface PausedSessionState {
  remainingSecs: number
}

export interface AbortedSessionState {
  reason: 'UserRequested' | 'PermissionRevoked' | 'SystemShutdown' | 'TimerCorrupted'
}

export interface Session {
  id: string
  profileId: string
  mode: SessionMode
  state: SessionStateKind
  plannedDurationSecs: number
  startedAt?: string
  completedAt?: string
}

export interface Profile {
  id: string
  name: string
  revision: number
  blockRules: BlockRule[]
  createdAt: string
  updatedAt: string
}

export interface BlockRule {
  id: string
  profileId: string
  kind: 'App' | 'Domain' | 'Category' | 'Network'
  target: string
  enabled: boolean
}

export interface EnforcementPlan {
  revision: number
  sessionId: string
  appRules: AppRule[]
  domainRules: DomainRule[]
  strictness: 'Normal' | 'Strict'
  planHash: string
}

export interface AppRule {
  bundleId?: string
  executablePath?: string
  action: BlockAction
}

export interface DomainRule {
  pattern: string
  normalized: string
  action: BlockAction
}

export type BlockAction = 'Block' | 'Allow' | { Redirect: { url: string } }

export interface PermissionState {
  name: string
  status: 'Healthy' | 'Degraded' | 'Unavailable' | 'PermissionDenied'
  detail?: string
}

export interface TamperEvent {
  kind:
    | 'HelperStopped'
    | 'ClockRollback'
    | 'UpdateMismatch'
    | 'RulesDiverged'
    | 'BinaryReplaced'
    | { PermissionRevoked: { permission: string } }
  detail?: string
  detectedAt: string
}
