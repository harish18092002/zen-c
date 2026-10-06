import { useQuery } from '@tanstack/react-query'
import { listRecentTamper, probePermissions } from '../commands'

export function SettingsPage() {
  const permissions = useQuery({
    queryKey: ['permissions'],
    queryFn: probePermissions,
    refetchInterval: 5 * 60 * 1000,
  })

  const tamper = useQuery({
    queryKey: ['tamper'],
    queryFn: () => listRecentTamper(10),
    refetchInterval: 60 * 1000,
  })

  return (
    <div style={styles.container}>
      <h1 style={styles.heading}>Settings</h1>

      <section style={styles.section}>
        <h2 style={styles.sectionTitle}>Permissions</h2>
        {permissions.isLoading && <p style={styles.muted}>Probing…</p>}
        {permissions.error instanceof Error && (
          <p style={styles.errorText}>Failed: {permissions.error.message}</p>
        )}
        {permissions.data?.map((p) => (
          <div key={p.name} style={styles.row}>
            <div>
              <div style={styles.rowTitle}>{p.name}</div>
              {p.detail && <div style={styles.rowDetail}>{p.detail}</div>}
            </div>
            <span style={styleForStatus(p.status)}>{p.status}</span>
          </div>
        ))}
      </section>

      <section style={styles.section}>
        <h2 style={styles.sectionTitle}>Tamper Events</h2>
        {tamper.isLoading && <p style={styles.muted}>Loading…</p>}
        {tamper.data?.length === 0 && (
          <p style={styles.muted}>No tamper events recorded. ✅</p>
        )}
        {tamper.data?.map((t, i) => (
          <div key={`${t.eventType}-${i}`} style={styles.row}>
            <div>
              <div style={styles.rowTitle}>{t.eventType}</div>
              {t.detail && <div style={styles.rowDetail}>{t.detail}</div>}
            </div>
            <span style={styles.timestamp}>{shorten(t.detectedAt)}</span>
          </div>
        ))}
      </section>

      <section style={styles.section}>
        <h2 style={styles.sectionTitle}>About</h2>
        <p style={styles.muted}>
          Zen Mode runs entirely on your device. No focus session data ever
          leaves your machine.
        </p>
      </section>
    </div>
  )
}

function shorten(iso: string): string {
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

function styleForStatus(status: string): React.CSSProperties {
  const base: React.CSSProperties = {
    fontSize: '0.8125rem',
    fontWeight: 600,
    padding: '0.25rem 0.625rem',
    borderRadius: '999px',
  }
  switch (status) {
    case 'Healthy':
      return { ...base, background: 'rgba(39, 174, 96, 0.12)', color: '#1e8449' }
    case 'Degraded':
      return { ...base, background: 'rgba(230, 126, 34, 0.12)', color: '#a05a16' }
    case 'PermissionDenied':
      return { ...base, background: 'rgba(192, 57, 43, 0.12)', color: '#a93226' }
    default:
      return { ...base, background: 'rgba(0, 0, 0, 0.06)', color: '#555' }
  }
}

const styles = {
  container: { maxWidth: '720px', margin: '0 auto' },
  heading: { fontSize: '1.5rem', fontWeight: 600, margin: '0 0 1.5rem' },
  section: { marginBottom: '2rem' },
  sectionTitle: {
    fontSize: '0.75rem',
    fontWeight: 600,
    textTransform: 'uppercase' as const,
    color: '#888',
    letterSpacing: '0.08em',
    marginBottom: '0.75rem',
  },
  row: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '0.875rem 1rem',
    background: '#fff',
    border: '1px solid rgba(0,0,0,0.06)',
    borderRadius: '8px',
    marginBottom: '0.5rem',
  },
  rowTitle: { fontWeight: 600, fontSize: '0.9375rem' },
  rowDetail: { fontSize: '0.8125rem', color: '#888', marginTop: '0.25rem' },
  timestamp: { fontSize: '0.8125rem', color: '#888' },
  muted: { color: '#888', fontSize: '0.9375rem' },
  errorText: { color: '#a93226', fontSize: '0.875rem' },
}
