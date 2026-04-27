import { useQuery } from '@tanstack/react-query'
import { listProfiles } from '../commands'

export function ProfilesPage() {
  const { data: profiles, isLoading } = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
  })

  return (
    <div style={styles.container}>
      <a href="/" style={styles.back}>← Back</a>
      <h1 style={styles.heading}>Profiles</h1>
      <p style={styles.description}>
        Create blocking profiles to define which apps and websites are blocked during your focus sessions.
      </p>

      {isLoading ? (
        <p>Loading...</p>
      ) : (
        <ul style={styles.list}>
          {profiles?.map((p) => (
            <li key={p.id} style={styles.profileItem}>
              <div style={styles.profileName}>{p.name}</div>
              <div style={styles.profileMeta}>{p.blockRules.length} rules · revision {p.revision}</div>
            </li>
          ))}
        </ul>
      )}

      <button style={styles.addButton}>+ New Profile</button>
    </div>
  )
}

const styles = {
  container: { maxWidth: '600px', margin: '0 auto', padding: '2rem', fontFamily: 'system-ui, -apple-system, sans-serif' },
  back: { color: '#555', textDecoration: 'none', fontSize: '0.875rem' },
  heading: { fontSize: '1.5rem', fontWeight: 600, margin: '1rem 0 0.5rem' },
  description: { color: '#666', marginBottom: '1.5rem' },
  list: { listStyle: 'none', padding: 0, margin: '0 0 1.5rem' },
  profileItem: { padding: '1rem', borderRadius: '8px', background: 'rgba(0,0,0,0.04)', marginBottom: '0.75rem' },
  profileName: { fontWeight: 600 },
  profileMeta: { fontSize: '0.875rem', color: '#666', marginTop: '0.25rem' },
  addButton: { padding: '0.75rem 1.5rem', borderRadius: '8px', border: '1.5px dashed #aaa', background: 'transparent', cursor: 'pointer' },
}
