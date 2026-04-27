import { useState } from 'react'
import { startSession, stopSession } from '../commands'
import { useSessionStore } from '../store/session'

export function DashboardPage() {
  const { sessionId, state, remainingSecs, setSession, clearSession } = useSessionStore()
  const [loading, setLoading] = useState(false)

  const handleStart = async () => {
    setLoading(true)
    try {
      const res = await startSession({
        profileId: 'default',
        durationSecs: 25 * 60,
        mode: 'Focus',
      })
      setSession(res.sessionId, 'Active')
    } finally {
      setLoading(false)
    }
  }

  const handleStop = async () => {
    if (!sessionId) return
    setLoading(true)
    try {
      await stopSession(sessionId)
      clearSession()
    } finally {
      setLoading(false)
    }
  }

  const minutes = Math.floor(remainingSecs / 60)
  const seconds = remainingSecs % 60

  return (
    <div style={styles.container}>
      <h1 style={styles.heading}>Zen Mode</h1>

      <div style={styles.timerCard}>
        <div style={styles.timerDisplay}>
          {state === 'Active'
            ? `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
            : '25:00'}
        </div>
        <div style={styles.status}>{state}</div>
      </div>

      <div style={styles.controls}>
        {state === 'Idle' || state === 'Completed' || state === 'Aborted' ? (
          <button style={styles.primaryButton} onClick={handleStart} disabled={loading}>
            {loading ? 'Starting...' : 'Start Focus Session'}
          </button>
        ) : (
          <button style={styles.dangerButton} onClick={handleStop} disabled={loading}>
            {loading ? 'Stopping...' : 'End Session'}
          </button>
        )}
      </div>

      <nav style={styles.nav}>
        <a href="/profiles" style={styles.navLink}>Profiles</a>
        <a href="/settings" style={styles.navLink}>Settings</a>
      </nav>
    </div>
  )
}

const styles = {
  container: {
    maxWidth: '480px',
    margin: '0 auto',
    padding: '2rem',
    fontFamily: 'system-ui, -apple-system, sans-serif',
    textAlign: 'center' as const,
  },
  heading: { fontSize: '1.5rem', fontWeight: 600, marginBottom: '2rem' },
  timerCard: {
    padding: '2rem',
    borderRadius: '16px',
    background: 'rgba(0,0,0,0.04)',
    marginBottom: '2rem',
  },
  timerDisplay: { fontSize: '4rem', fontWeight: 700, fontVariantNumeric: 'tabular-nums' },
  status: { fontSize: '0.875rem', color: '#666', marginTop: '0.5rem' },
  controls: { marginBottom: '2rem' },
  primaryButton: {
    padding: '0.75rem 2rem',
    fontSize: '1rem',
    fontWeight: 600,
    background: '#1a1a1a',
    color: '#fff',
    border: 'none',
    borderRadius: '8px',
    cursor: 'pointer',
  },
  dangerButton: {
    padding: '0.75rem 2rem',
    fontSize: '1rem',
    fontWeight: 600,
    background: '#c0392b',
    color: '#fff',
    border: 'none',
    borderRadius: '8px',
    cursor: 'pointer',
  },
  nav: { display: 'flex', gap: '1.5rem', justifyContent: 'center' },
  navLink: { color: '#555', textDecoration: 'none', fontSize: '0.875rem' },
}
