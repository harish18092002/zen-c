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

export interface Profile {
  id: string
  name: string
  revision: number
  blockRules: BlockRule[]
}

export interface BlockRule {
  id: string
  kind: 'App' | 'Domain' | 'Category' | 'Network'
  target: string
  enabled: boolean
}

export function startSession(payload: StartSessionPayload): Promise<SessionResponse> {
  return invoke<SessionResponse>('start_session', { payload })
}

export function stopSession(sessionId: string): Promise<void> {
  return invoke<void>('stop_session', { sessionId })
}

export function getSessionState(sessionId: string): Promise<SessionResponse> {
  return invoke<SessionResponse>('get_session_state', { sessionId })
}

export function listProfiles(): Promise<Profile[]> {
  return invoke<Profile[]>('list_profiles')
}
