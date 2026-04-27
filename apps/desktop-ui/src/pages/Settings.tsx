export function SettingsPage() {
  return (
    <div style={styles.container}>
      <a href="/" style={styles.back}>← Back</a>
      <h1 style={styles.heading}>Settings</h1>

      <section style={styles.section}>
        <h2 style={styles.sectionTitle}>General</h2>
        <label style={styles.row}>
          <span>Launch at login</span>
          <input type="checkbox" />
        </label>
        <label style={styles.row}>
          <span>Show in menu bar</span>
          <input type="checkbox" defaultChecked />
        </label>
      </section>

      <section style={styles.section}>
        <h2 style={styles.sectionTitle}>Privacy</h2>
        <label style={styles.row}>
          <span>Enable telemetry</span>
          <input type="checkbox" />
        </label>
      </section>

      <section style={styles.section}>
        <h2 style={styles.sectionTitle}>Permissions</h2>
        <div style={styles.permissionRow}>
          <span>Accessibility</span>
          <span style={styles.statusPending}>Check</span>
        </div>
        <div style={styles.permissionRow}>
          <span>Notifications</span>
          <span style={styles.statusOk}>Granted</span>
        </div>
      </section>
    </div>
  )
}

const styles = {
  container: { maxWidth: '600px', margin: '0 auto', padding: '2rem', fontFamily: 'system-ui, -apple-system, sans-serif' },
  back: { color: '#555', textDecoration: 'none', fontSize: '0.875rem' },
  heading: { fontSize: '1.5rem', fontWeight: 600, margin: '1rem 0 1.5rem' },
  section: { marginBottom: '2rem' },
  sectionTitle: { fontSize: '0.75rem', fontWeight: 600, textTransform: 'uppercase' as const, color: '#888', letterSpacing: '0.08em', marginBottom: '0.75rem' },
  row: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '0.75rem 0', borderBottom: '1px solid rgba(0,0,0,0.06)' },
  permissionRow: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '0.75rem 0', borderBottom: '1px solid rgba(0,0,0,0.06)' },
  statusOk: { fontSize: '0.875rem', color: '#27ae60', fontWeight: 500 },
  statusPending: { fontSize: '0.875rem', color: '#e67e22', fontWeight: 500 },
}
