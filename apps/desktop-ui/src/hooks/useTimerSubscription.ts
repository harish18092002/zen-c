import { useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import type { TimerTick } from '@zen-mode/ui-contracts'
import { useSessionStore } from '../store/session'

/// Subscribe to backend timer events as soon as the component mounts and
/// keep the Zustand store in sync. Mount once at the app root so every page
/// reads from the same authoritative state.
export function useTimerSubscription() {
  const applyTick = useSessionStore((s) => s.applyTick)
  const clearSession = useSessionStore((s) => s.clearSession)

  useEffect(() => {
    let unlistenTick: (() => void) | undefined
    let unlistenCompleted: (() => void) | undefined

    listen<TimerTick>('session://tick', (event) => {
      applyTick(event.payload)
    }).then((un) => {
      unlistenTick = un
    })

    listen<string>('session://completed', () => {
      clearSession()
    }).then((un) => {
      unlistenCompleted = un
    })

    return () => {
      unlistenTick?.()
      unlistenCompleted?.()
    }
  }, [applyTick, clearSession])
}
