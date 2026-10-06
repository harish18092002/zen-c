import { useEffect } from 'react'
import { Link, Outlet } from '@tanstack/react-router'
import { useTimerSubscription } from './hooks/useTimerSubscription'
import { getActiveSession } from './commands'
import { useSessionStore } from './store/session'

/// Root chrome shared by every page. Mounts the timer subscription once,
/// hydrates the store from any session the backend recovered on startup,
/// and renders the navigation strip.
export function AppLayout() {
  useTimerSubscription()
  const hydrate = useSessionStore((s) => s.hydrate)

  useEffect(() => {
    getActiveSession()
      .then((session) => hydrate(session))
      .catch((err) => {
        // Not fatal — frontend just shows the empty state.
        console.warn('hydrate active session failed:', err)
      })
  }, [hydrate])

  return (
    <div style={styles.shell}>
      <header style={styles.header}>
        <Link to="/" style={styles.brand}>
          Zen Mode
        </Link>
        <nav style={styles.nav}>
          <Link
            to="/"
            style={styles.navLink}
            activeProps={{ style: styles.navLinkActive }}
          >
            Dashboard
          </Link>
          <Link
            to="/profiles"
            style={styles.navLink}
            activeProps={{ style: styles.navLinkActive }}
          >
            Profiles
          </Link>
          <Link
            to="/settings"
            style={styles.navLink}
            activeProps={{ style: styles.navLinkActive }}
          >
            Settings
          </Link>
        </nav>
      </header>
      <main style={styles.main}>
        <Outlet />
      </main>
    </div>
  )
}

const styles = {
  shell: {
    minHeight: '100vh',
    fontFamily: 'system-ui, -apple-system, sans-serif',
    background: '#fafafa',
    color: '#1a1a1a',
  },
  header: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '1rem 2rem',
    borderBottom: '1px solid rgba(0,0,0,0.06)',
    background: '#fff',
  },
  brand: {
    fontSize: '1.125rem',
    fontWeight: 700,
    color: '#1a1a1a',
    textDecoration: 'none',
  },
  nav: {
    display: 'flex',
    gap: '1.5rem',
  },
  navLink: {
    color: '#555',
    textDecoration: 'none',
    fontSize: '0.875rem',
    padding: '0.5rem 0',
  } as const,
  navLinkActive: {
    color: '#1a1a1a',
    fontWeight: 600,
    borderBottom: '2px solid #1a1a1a',
  } as const,
  main: {
    padding: '2rem',
  },
}
