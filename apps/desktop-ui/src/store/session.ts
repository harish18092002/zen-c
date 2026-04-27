import { create } from 'zustand'

export type SessionStateKind =
  | 'Idle'
  | 'Preparing'
  | 'Active'
  | 'Paused'
  | 'Completing'
  | 'Completed'
  | 'Aborted'
  | 'Recovering'

interface SessionStore {
  sessionId: string | null
  state: SessionStateKind
  remainingSecs: number
  setSession: (id: string, state: SessionStateKind) => void
  updateState: (state: SessionStateKind) => void
  updateRemaining: (secs: number) => void
  clearSession: () => void
}

export const useSessionStore = create<SessionStore>((set) => ({
  sessionId: null,
  state: 'Idle',
  remainingSecs: 0,
  setSession: (id, state) => set({ sessionId: id, state }),
  updateState: (state) => set({ state }),
  updateRemaining: (secs) => set({ remainingSecs: secs }),
  clearSession: () => set({ sessionId: null, state: 'Idle', remainingSecs: 0 }),
}))
