import { create } from 'zustand'
import type {
  Session,
  SessionStateKind,
  TimerStateString,
  TimerTick,
} from '@zen-mode/ui-contracts'

interface SessionStore {
  sessionId: string | null
  stateKind: SessionStateKind
  timerState: TimerStateString
  remainingSecs: number
  elapsedSecs: number
  plannedSecs: number
  hydrate: (session: Session | null) => void
  setSession: (id: string, kind: SessionStateKind, plannedSecs: number) => void
  applyTick: (tick: TimerTick) => void
  setStateKind: (kind: SessionStateKind) => void
  clearSession: () => void
}

export const useSessionStore = create<SessionStore>((set) => ({
  sessionId: null,
  stateKind: 'Idle',
  timerState: 'Idle',
  remainingSecs: 0,
  elapsedSecs: 0,
  plannedSecs: 0,

  hydrate: (session) => {
    if (!session) {
      set({
        sessionId: null,
        stateKind: 'Idle',
        timerState: 'Idle',
        remainingSecs: 0,
        elapsedSecs: 0,
        plannedSecs: 0,
      })
      return
    }
    const remaining = computeRemainingFromState(session)
    set({
      sessionId: session.id,
      stateKind: session.state.kind,
      timerState: session.state.kind === 'Paused' ? 'Paused' : 'Running',
      remainingSecs: remaining,
      elapsedSecs: session.plannedDurationSecs - remaining,
      plannedSecs: session.plannedDurationSecs,
    })
  },

  setSession: (id, kind, plannedSecs) =>
    set({
      sessionId: id,
      stateKind: kind,
      timerState: 'Running',
      plannedSecs,
      remainingSecs: plannedSecs,
      elapsedSecs: 0,
    }),

  applyTick: (tick) =>
    set({
      sessionId: tick.sessionId,
      timerState: tick.state,
      remainingSecs: tick.remainingSecs,
      elapsedSecs: tick.elapsedSecs,
      plannedSecs: tick.plannedSecs,
      stateKind:
        tick.state === 'Paused'
          ? 'Paused'
          : tick.state === 'Completed'
            ? 'Completed'
            : 'Active',
    }),

  setStateKind: (kind) => set({ stateKind: kind }),

  clearSession: () =>
    set({
      sessionId: null,
      stateKind: 'Idle',
      timerState: 'Idle',
      remainingSecs: 0,
      elapsedSecs: 0,
      plannedSecs: 0,
    }),
}))

function computeRemainingFromState(session: Session): number {
  switch (session.state.kind) {
    case 'Paused':
      return session.state.remainingSecs
    case 'Active': {
      const ends = Date.parse(session.state.endsAt)
      if (Number.isNaN(ends)) return session.plannedDurationSecs
      const secs = Math.max(0, Math.floor((ends - Date.now()) / 1000))
      return Math.min(secs, session.plannedDurationSecs)
    }
    case 'Idle':
    case 'Preparing':
    case 'Recovering':
      return session.plannedDurationSecs
    default:
      return 0
  }
}
