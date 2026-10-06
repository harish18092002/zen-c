import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { deleteProfile, listProfiles } from '../commands'
import type { Profile } from '@zen-mode/ui-contracts'
import { useState } from 'react'

export function ProfilesPage() {
  const queryClient = useQueryClient()
  const { data: profiles, isLoading, error } = useQuery({
    queryKey: ['profiles'],
    queryFn: listProfiles,
  })
  const [confirmId, setConfirmId] = useState<string | null>(null)

  const deleteMutation = useMutation({
    mutationFn: deleteProfile,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['profiles'] })
      setConfirmId(null)
    },
  })

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <h1 style={styles.heading}>Profiles</h1>
        <Link to="/profiles/new" style={styles.addButton}>
          + New Profile
        </Link>
      </div>
      <p style={styles.description}>
        Profiles define which apps and websites are blocked during a focus
        session. You can switch profiles per session.
      </p>

      {isLoading && <p style={styles.muted}>Loading…</p>}
      {error instanceof Error && (
        <p style={styles.errorText}>Failed to load profiles: {error.message}</p>
      )}

      {profiles && profiles.length === 0 && (
        <div style={styles.emptyCard}>
          <p style={styles.muted}>No profiles yet.</p>
          <Link to="/profiles/new" style={styles.primaryLink}>
            Create your first profile →
          </Link>
        </div>
      )}

      <ul style={styles.list}>
        {profiles?.map((p) => (
          <ProfileRow
            key={p.id}
            profile={p}
            confirmDelete={confirmId === p.id}
            onAskDelete={() => setConfirmId(p.id)}
            onCancelDelete={() => setConfirmId(null)}
            onConfirmDelete={() => deleteMutation.mutate(p.id)}
            isDeleting={deleteMutation.isPending && deleteMutation.variables === p.id}
          />
        ))}
      </ul>
    </div>
  )
}

interface ProfileRowProps {
  profile: Profile
  confirmDelete: boolean
  onAskDelete: () => void
  onCancelDelete: () => void
  onConfirmDelete: () => void
  isDeleting: boolean
}

function ProfileRow({
  profile,
  confirmDelete,
  onAskDelete,
  onCancelDelete,
  onConfirmDelete,
  isDeleting,
}: ProfileRowProps) {
  return (
    <li style={styles.profileItem}>
      <div style={styles.profileMain}>
        <Link
          to="/profiles/$profileId"
          params={{ profileId: profile.id }}
          style={styles.profileLink}
        >
          <div style={styles.profileName}>{profile.name}</div>
          <div style={styles.profileMeta}>
            {profile.blockRules.length} rule
            {profile.blockRules.length === 1 ? '' : 's'} · revision {profile.revision}
          </div>
        </Link>
      </div>
      <div style={styles.profileActions}>
        {confirmDelete ? (
          <>
            <button
              type="button"
              style={styles.dangerButtonSmall}
              onClick={onConfirmDelete}
              disabled={isDeleting}
            >
              {isDeleting ? 'Deleting…' : 'Confirm'}
            </button>
            <button
              type="button"
              style={styles.secondaryButtonSmall}
              onClick={onCancelDelete}
              disabled={isDeleting}
            >
              Cancel
            </button>
          </>
        ) : (
          <button
            type="button"
            style={styles.secondaryButtonSmall}
            onClick={onAskDelete}
          >
            Delete
          </button>
        )}
      </div>
    </li>
  )
}

const styles = {
  container: { maxWidth: '720px', margin: '0 auto' },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '0.5rem',
  },
  heading: { fontSize: '1.5rem', fontWeight: 600, margin: 0 },
  description: { color: '#666', marginTop: '0.25rem', marginBottom: '1.5rem' },
  addButton: {
    padding: '0.5rem 1rem',
    borderRadius: '8px',
    background: '#1a1a1a',
    color: '#fff',
    textDecoration: 'none',
    fontSize: '0.875rem',
    fontWeight: 600,
  } as const,
  list: { listStyle: 'none', padding: 0, margin: 0 },
  profileItem: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '1rem 1.25rem',
    borderRadius: '12px',
    background: '#fff',
    border: '1px solid rgba(0,0,0,0.06)',
    marginBottom: '0.75rem',
  },
  profileMain: { flex: 1 },
  profileLink: { textDecoration: 'none', color: 'inherit', display: 'block' } as const,
  profileName: { fontWeight: 600, fontSize: '1rem' },
  profileMeta: { fontSize: '0.875rem', color: '#888', marginTop: '0.25rem' },
  profileActions: { display: 'flex', gap: '0.5rem' },
  secondaryButtonSmall: {
    padding: '0.375rem 0.75rem',
    fontSize: '0.8125rem',
    background: '#fff',
    border: '1px solid rgba(0,0,0,0.1)',
    borderRadius: '6px',
    cursor: 'pointer' as const,
  },
  dangerButtonSmall: {
    padding: '0.375rem 0.75rem',
    fontSize: '0.8125rem',
    background: '#c0392b',
    color: '#fff',
    border: 'none',
    borderRadius: '6px',
    cursor: 'pointer' as const,
  },
  emptyCard: {
    padding: '2rem',
    borderRadius: '12px',
    background: '#fff',
    border: '1px dashed rgba(0,0,0,0.12)',
    textAlign: 'center' as const,
    marginBottom: '1.5rem',
  },
  muted: { color: '#888', fontSize: '0.9375rem' },
  errorText: { color: '#a93226', fontSize: '0.875rem' },
  primaryLink: {
    color: '#1a1a1a',
    textDecoration: 'none',
    fontWeight: 600,
    fontSize: '0.9375rem',
  } as const,
}
