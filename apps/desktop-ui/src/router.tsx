import { createRouter, createRootRoute, createRoute, Outlet } from '@tanstack/react-router'
import { DashboardPage } from './pages/Dashboard'
import { ProfilesPage } from './pages/Profiles'
import { SettingsPage } from './pages/Settings'

const rootRoute = createRootRoute({
  component: () => <Outlet />,
})

const dashboardRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: DashboardPage,
})

const profilesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/profiles',
  component: ProfilesPage,
})

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/settings',
  component: SettingsPage,
})

const routeTree = rootRoute.addChildren([
  dashboardRoute,
  profilesRoute,
  settingsRoute,
])

export const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
