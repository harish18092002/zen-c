import { invoke } from '@tauri-apps/api/core'
import type {
  Permission,
  Profile,
  ProfileUpsertRequest,
  Session,
  StartSessionRequest,
  StartSessionResult,
  TamperRecord,
} from '@zen-mode/ui-contracts'

export function startSession(payload: StartSessionRequest): Promise<StartSessionResult> {
  return invoke<StartSessionResult>('start_session', { payload })
}

export function stopSession(sessionId: string): Promise<void> {
  return invoke<void>('stop_session', { sessionId })
}

export function pauseSession(sessionId: string): Promise<number> {
  return invoke<number>('pause_session', { sessionId })
}

export function resumeSession(sessionId: string): Promise<string> {
  return invoke<string>('resume_session', { sessionId })
}

export function getSessionState(sessionId: string): Promise<Session | null> {
  return invoke<Session | null>('get_session_state', { sessionId })
}

export function getActiveSession(): Promise<Session | null> {
  return invoke<Session | null>('get_active_session')
}

export function listProfiles(): Promise<Profile[]> {
  return invoke<Profile[]>('list_profiles')
}

export function getProfile(profileId: string): Promise<Profile | null> {
  return invoke<Profile | null>('get_profile', { profileId })
}

export function upsertProfile(payload: ProfileUpsertRequest): Promise<Profile> {
  return invoke<Profile>('upsert_profile', { payload })
}

export function deleteProfile(profileId: string): Promise<boolean> {
  return invoke<boolean>('delete_profile', { profileId })
}

export function probePermissions(): Promise<Permission[]> {
  return invoke<Permission[]>('probe_permissions')
}

export function listRecentTamper(limit?: number): Promise<TamperRecord[]> {
  return invoke<TamperRecord[]>('list_recent_tamper', { limit })
}
