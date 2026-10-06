import { useEffect } from 'react'
import { useForm, useFieldArray, Controller } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { Link, useNavigate, useParams } from '@tanstack/react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { BlockRuleKind } from '@zen-mode/ui-contracts'
import { getProfile, upsertProfile } from '../commands'

const ruleSchema = z.object({
  kind: z.enum(['App', 'Domain', 'Category', 'Network']),
  target: z
    .string()
    .min(1, 'Required')
    .max(256, 'Too long')
    .transform((s) => s.trim()),
  enabled: z.boolean(),
})

const profileSchema = z.object({
  name: z
    .string()
    .min(1, 'Name is required')
    .max(64, 'Name is too long')
    .transform((s) => s.trim()),
  blockRules: z.array(ruleSchema).max(500, 'Too many rules'),
})

type ProfileFormValues = z.infer<typeof profileSchema>

interface ProfileEditorProps {
  mode: 'create' | 'edit'
}

const RULE_KINDS: BlockRuleKind[] = ['App', 'Domain', 'Category', 'Network']

export function ProfileEditor({ mode }: ProfileEditorProps) {
  const params = useParams({ strict: false })
  const profileId = mode === 'edit' ? (params as { profileId?: string }).profileId : undefined
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const existing = useQuery({
    queryKey: ['profile', profileId],
    queryFn: () => (profileId ? getProfile(profileId) : Promise.resolve(null)),
    enabled: mode === 'edit' && !!profileId,
  })

  const {
    control,
    handleSubmit,
    register,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<ProfileFormValues>({
    resolver: zodResolver(profileSchema),
    defaultValues: { name: '', blockRules: [] },
  })

  const { fields, append, remove } = useFieldArray({ control, name: 'blockRules' })

  useEffect(() => {
    if (mode === 'edit' && existing.data) {
      reset({
        name: existing.data.name,
        blockRules: existing.data.blockRules.map((r) => ({
          kind: r.kind,
          target: r.target,
          enabled: r.enabled,
        })),
      })
    }
  }, [mode, existing.data, reset])

  const save = useMutation({
    mutationFn: (values: ProfileFormValues) =>
      upsertProfile({
        id: profileId,
        name: values.name,
        blockRules: values.blockRules,
      }),
    onSuccess: (profile) => {
      queryClient.invalidateQueries({ queryKey: ['profiles'] })
      queryClient.setQueryData(['profile', profile.id], profile)
      navigate({ to: '/profiles' })
    },
  })

  const onSubmit = handleSubmit((values) => save.mutate(values))

  if (mode === 'edit' && existing.isLoading) {
    return <p style={styles.muted}>Loading profile…</p>
  }
  if (mode === 'edit' && !existing.data && !existing.isLoading) {
    return (
      <div style={styles.container}>
        <p style={styles.muted}>Profile not found.</p>
        <Link to="/profiles" style={styles.backLink}>
          ← Back to profiles
        </Link>
      </div>
    )
  }

  return (
    <form style={styles.container} onSubmit={onSubmit}>
      <Link to="/profiles" style={styles.backLink}>
        ← Back to profiles
      </Link>
      <h1 style={styles.heading}>{mode === 'create' ? 'New Profile' : 'Edit Profile'}</h1>

      <section style={styles.section}>
        <label style={styles.label}>
          Name
          <input
            type="text"
            placeholder="e.g. Deep Work"
            style={styles.input}
            {...register('name')}
          />
        </label>
        {errors.name && <span style={styles.fieldError}>{errors.name.message}</span>}
      </section>

      <section style={styles.section}>
        <div style={styles.sectionTitleRow}>
          <span style={styles.sectionTitle}>Block Rules</span>
          <button
            type="button"
            style={styles.addRuleButton}
            onClick={() => append({ kind: 'Domain', target: '', enabled: true })}
          >
            + Add Rule
          </button>
        </div>
        {fields.length === 0 ? (
          <p style={styles.muted}>No rules yet. Add an app bundle ID or a website.</p>
        ) : (
          <ul style={styles.ruleList}>
            {fields.map((field, index) => (
              <li key={field.id} style={styles.ruleRow}>
                <Controller
                  control={control}
                  name={`blockRules.${index}.kind` as const}
                  render={({ field }) => (
                    <select style={styles.kindSelect} {...field}>
                      {RULE_KINDS.map((k) => (
                        <option key={k} value={k}>
                          {k}
                        </option>
                      ))}
                    </select>
                  )}
                />
                <input
                  type="text"
                  placeholder={placeholderFor(fields[index].kind)}
                  style={styles.input}
                  {...register(`blockRules.${index}.target` as const)}
                />
                <label style={styles.toggle}>
                  <input
                    type="checkbox"
                    {...register(`blockRules.${index}.enabled` as const)}
                  />
                  <span style={styles.toggleLabel}>On</span>
                </label>
                <button
                  type="button"
                  style={styles.removeButton}
                  onClick={() => remove(index)}
                  aria-label="Remove rule"
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      {save.error instanceof Error && (
        <div style={styles.errorBanner}>{save.error.message}</div>
      )}

      <div style={styles.footer}>
        <Link to="/profiles" style={styles.cancelLink}>
          Cancel
        </Link>
        <button type="submit" style={styles.saveButton} disabled={isSubmitting || save.isPending}>
          {save.isPending ? 'Saving…' : mode === 'create' ? 'Create Profile' : 'Save Changes'}
        </button>
      </div>
    </form>
  )
}

function placeholderFor(kind: BlockRuleKind): string {
  switch (kind) {
    case 'App':
      return 'com.apple.Safari or path to executable'
    case 'Domain':
      return 'twitter.com'
    case 'Category':
      return 'social-media'
    case 'Network':
      return '203.0.113.0/24'
  }
}

const styles = {
  container: { maxWidth: '720px', margin: '0 auto' },
  heading: { fontSize: '1.5rem', fontWeight: 600, margin: '1rem 0 1.5rem' },
  backLink: {
    color: '#555',
    textDecoration: 'none',
    fontSize: '0.875rem',
  } as const,
  section: { marginBottom: '1.5rem' },
  sectionTitleRow: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '0.75rem',
  },
  sectionTitle: {
    fontSize: '0.75rem',
    fontWeight: 600,
    color: '#888',
    textTransform: 'uppercase' as const,
    letterSpacing: '0.08em',
  },
  label: {
    display: 'block',
    fontSize: '0.875rem',
    fontWeight: 600,
    color: '#333',
  },
  input: {
    width: '100%',
    padding: '0.625rem 0.75rem',
    fontSize: '0.9375rem',
    borderRadius: '8px',
    border: '1px solid rgba(0,0,0,0.1)',
    background: '#fff',
    marginTop: '0.375rem',
  },
  fieldError: {
    display: 'block',
    color: '#c0392b',
    fontSize: '0.8125rem',
    marginTop: '0.25rem',
  },
  addRuleButton: {
    padding: '0.375rem 0.75rem',
    fontSize: '0.8125rem',
    background: '#fff',
    border: '1px solid rgba(0,0,0,0.15)',
    borderRadius: '6px',
    cursor: 'pointer' as const,
  },
  ruleList: { listStyle: 'none', padding: 0, margin: 0 },
  ruleRow: {
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
    padding: '0.5rem',
    background: '#fff',
    border: '1px solid rgba(0,0,0,0.06)',
    borderRadius: '8px',
    marginBottom: '0.5rem',
  },
  kindSelect: {
    padding: '0.5rem 0.625rem',
    fontSize: '0.875rem',
    borderRadius: '6px',
    border: '1px solid rgba(0,0,0,0.1)',
    background: '#fff',
    flexShrink: 0,
  },
  toggle: {
    display: 'flex',
    alignItems: 'center',
    gap: '0.25rem',
    fontSize: '0.8125rem',
    color: '#666',
    flexShrink: 0,
  },
  toggleLabel: { fontSize: '0.75rem' },
  removeButton: {
    width: '32px',
    height: '32px',
    border: 'none',
    background: 'transparent',
    color: '#999',
    fontSize: '1.25rem',
    cursor: 'pointer' as const,
    borderRadius: '6px',
    flexShrink: 0,
  },
  footer: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginTop: '2rem',
    paddingTop: '1.5rem',
    borderTop: '1px solid rgba(0,0,0,0.06)',
  },
  cancelLink: {
    color: '#666',
    textDecoration: 'none',
    fontSize: '0.875rem',
  } as const,
  saveButton: {
    padding: '0.625rem 1.5rem',
    fontSize: '0.9375rem',
    fontWeight: 600,
    background: '#1a1a1a',
    color: '#fff',
    border: 'none',
    borderRadius: '8px',
    cursor: 'pointer' as const,
  },
  muted: { color: '#888', fontSize: '0.9375rem' },
  errorBanner: {
    padding: '0.75rem 1rem',
    marginTop: '1rem',
    borderRadius: '8px',
    background: 'rgba(192, 57, 43, 0.08)',
    color: '#a93226',
    fontSize: '0.875rem',
  },
}
