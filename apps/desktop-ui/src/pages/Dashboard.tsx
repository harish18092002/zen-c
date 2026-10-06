import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  listProfiles,
  pauseSession,
  resumeSession,
  startSession,
  stopSession,
} from '../commands'
import { useSessionStore } from '../store/session'
import type { SessionMode } from '@zen-mode/ui-contracts'

const PRESETS: { label: string; secs: number; mode: SessionMode }[] = [
  { label: 'Quick — 15m', secs: 15 * 60, mode: 'Focus' },
  { label: 'Focus — 25m', secs: 25 * 60, mode: 'Focus' },
  { label: 'Deep — 50m', secs: 50 * 60, mode: 'Focus' },
  { label: 'Strict — 90m', secs: 90 * 60, mode: 'Strict' },
]

export function DashboardPage() {
  const queryClient = useQueryClient()
  const { sessionId, stateKind, timerState, remainingSecs, plannedSecs } = useSessionStore()
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null)
  const [selectedPreset, setSelectedPreset] = useState<number>(1)
  const [errorMsg, setErrorMsg] = useState<string | null>(null)

  const profilesQuery = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
  })

  const defaultProfileId =
    selectedProfileId ?? profilesQuery.data?.[0]?.id ?? null

  const start = useMutation({
    mutationFn: async () => {
      if (!defaultProfileId) throw new Error('No profile available')
      const preset = PRESETS[selectedPreset]
      return startSession({
        profileId: defaultProfileId,
        durationSecs: preset.secs,
        mode: preset.mode,
      })
    },
    onError: (err: unknown) => setErrorMsg(formatError(err)),
    onSuccess: () => {
      setErrorMsg(null)
      queryClient.invalidateQueries({ queryKey: ['active-session'] })
    },
  })

  const stop = useMutation({
    mutationFn: async () => {
      if (!sessionId) return
      await stopSession(sessionId)
    },
    onError: (err) => setErrorMsg(formatError(err)),
  })

  const pause = useMutation({
    mutationFn: async () => {
      if (!sessionId) return
      await pauseSession(sessionId)
    },
    onError: (err) => setErrorMsg(formatError(err)),
  })

  const resume = useMutation({
    mutationFn: async () => {
      if (!sessionId) return
      await resumeSession(sessionId)
    },
    onError: (err) => setErrorMsg(formatError(err)),
  })

  const minutes = Math.floor(remainingSecs / 60)
  const seconds = remainingSecs % 60
  const display = sessionId
    ? `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
    : `${String(Math.floor(PRESETS[selectedPreset].secs / 60)).padStart(2, '0')}:00`

  const percent =
    plannedSecs > 0 ? Math.min(100, Math.max(0, ((plannedSecs - remainingSecs) / plannedSecs) * 100)) : 0

  const isActive = stateKind === 'Active' || stateKind === 'Preparing'
  const isPaused = stateKind === 'Paused'
  const canStart = !sessionId || stateKind === 'Idle' || stateKind === 'Completed' || stateKind === 'Aborted'

  return (
    <div style={styles.container}>
      <div style={styles.timerCard}>
        <div style={styles.timerDisplay}>{display}</div>
        <div style={styles.status}>
          {sessionId ? labelFor(timerState, stateKind) : 'Ready'}
        </div>
        <div style={styles.progressTrack}>
          <div style={{ ...styles.progressFill, width: `${percent}%` }} />
        </div>
      </div>

      {errorMsg && <div style={styles.errorBanner}>{errorMsg}</div>}

      {canStart ? (
        <>
          <section style={styles.section}>
            <div style={styles.sectionTitle}>Profile</div>
            {profilesQuery.isLoading ? (
              <p style={styles.muted}>Loading profiles…</p>
            ) : (profilesQuery.data?.length ?? 0) === 0 ? (
              <p style={styles.muted}>
                Create a profile from the Profiles tab to start a session.
              </p>
            ) : (
              <select
                style={styles.select}
                value={defaultProfileId ?? ''}
                onChange={(e) => setSelectedProfileId(e.target.value)}
              >
                {profilesQuery.data?.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name} · {p.blockRules.length} rules
                  </option>
                ))}
              </select>
            )}
          </section>

          <section style={styles.section}>
            <div style={styles.sectionTitle}>Duration</div>
            <div style={styles.presetRow}>
              {PRESETS.map((preset, i) => (
                <button
                  key={preset.label}
                  type="button"
                  style={i === selectedPreset ? styles.presetActive : styles.preset}
                  onClick={() => setSelectedPreset(i)}
                >
                  {preset.label}
                </button>
              ))}
            </div>
          </section>

          <button
            type="button"
            style={styles.primaryButton}
            onClick={() => start.mutate()}
            disabled={start.isPending || !defaultProfileId}
          >
            {start.isPending ? 'Starting…' : 'Start Focus Session'}
          </button>
        </>
      ) : (
        <div style={styles.controlRow}>
          {isActive && (
            <button
              type="button"
              style={styles.secondaryButton}
              onClick={() => pause.mutate()}
              disabled={pause.isPending}
            >
              Pause
            </button>
          )}
          {isPaused && (
            <button
              type="button"
              style={styles.secondaryButton}
              onClick={() => resume.mutate()}
              disabled={resume.isPending}
            >
              Resume
            </button>
          )}
          <button
            type="button"
            style={styles.dangerButton}
            onClick={() => stop.mutate()}
            disabled={stop.isPending}
          >
            End Session
          </button>
        </div>
      )}
    </div>
  )
}

function labelFor(timerState: string, kind: string): string {
  if (kind === 'Paused') return 'Paused'
  if (kind === 'Active') return timerState === 'Running' ? 'Active' : timerState
  return kind
}

function formatError(err: unknown): string {
  if (err instanceof Error) return err.message
  if (typeof err === 'string') return err
  return JSON.stringify(err)
}

const styles = {
  container: {
    maxWidth: '480px',
    margin: '0 auto',
  },
  timerCard: {
    padding: '2rem',
    borderRadius: '16px',
    background: '#fff',
    border: '1px solid rgba(0,0,0,0.06)',
    marginBottom: '1.5rem',
    textAlign: 'center' as const,
  },
  timerDisplay: {
    fontSize: '4rem',
    fontWeight: 700,
    fontVariantNumeric: 'tabular-nums' as const,
    letterSpacing: '-0.02em',
  },
  status: {
    fontSize: '0.875rem',
    color: '#888',
    textTransform: 'uppercase' as const,
    letterSpacing: '0.08em',
    marginTop: '0.5rem',
  },
  progressTrack: {
    marginTop: '1.5rem',
    height: '4px',
    borderRadius: '2px',
    background: 'rgba(0,0,0,0.06)',
    overflow: 'hidden' as const,
  },
  progressFill: {
    height: '100%',
    background: '#1a1a1a',
    transition: 'width 0.4s ease-out',
  },
  section: {
    marginBottom: '1.25rem',
  },
  sectionTitle: {
    fontSize: '0.75rem',
    color: '#888',
    fontWeight: 600,
    textTransform: 'uppercase' as const,
    letterSpacing: '0.08em',
    marginBottom: '0.5rem',
  },
  select: {
    width: '100%',
    padding: '0.625rem 0.75rem',
    fontSize: '0.875rem',
    borderRadius: '8px',
    border: '1px solid rgba(0,0,0,0.1)',
    background: '#fff',
  },
  presetRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(2, 1fr)',
    gap: '0.5rem',
  },
  preset: {
    padding: '0.625rem',
    borderRadius: '8px',
    border: '1px solid rgba(0,0,0,0.1)',
    background: '#fff',
    cursor: 'pointer' as const,
    fontSize: '0.875rem',
  },
  presetActive: {
    padding: '0.625rem',
    borderRadius: '8px',
    border: '1px solid #1a1a1a',
    background: '#1a1a1a',
    color: '#fff',
    cursor: 'pointer' as const,
    fontSize: '0.875rem',
    fontWeight: 600,
  },
  primaryButton: {
    width: '100%',
    padding: '0.875rem 1.5rem',
    fontSize: '1rem',
    fontWeight: 600,
    background: '#1a1a1a',
    color: '#fff',
    border: 'none',
    borderRadius: '8px',
    cursor: 'pointer' as const,
  },
  secondaryButton: {
    flex: 1,
    padding: '0.75rem 1rem',
    fontSize: '0.9375rem',
    background: '#fff',
    border: '1px solid rgba(0,0,0,0.1)',
    borderRadius: '8px',
    cursor: 'pointer' as const,
  },
  dangerButton: {
    flex: 1,
    padding: '0.75rem 1rem',
    fontSize: '0.9375rem',
    background: '#c0392b',
    color: '#fff',
    border: 'none',
    borderRadius: '8px',
    cursor: 'pointer' as const,
  },
  controlRow: {
    display: 'flex',
    gap: '0.75rem',
  },
  errorBanner: {
    padding: '0.75rem 1rem',
    marginBottom: '1.25rem',
    borderRadius: '8px',
    background: 'rgba(192, 57, 43, 0.08)',
    color: '#a93226',
    fontSize: '0.875rem',
  },
  muted: {
    color: '#888',
    fontSize: '0.875rem',
  },
}
