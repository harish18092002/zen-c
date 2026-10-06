import {
  createRouter,
  createRootRoute,
  createRoute,
} from '@tanstack/react-router'
import { AppLayout } from './AppLayout'
import { DashboardPage } from './pages/Dashboard'
import { ProfilesPage } from './pages/Profiles'
import { ProfileEditor } from './pages/ProfileEditor'
import { SettingsPage } from './pages/Settings'

const rootRoute = createRootRoute({
  component: AppLayout,
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

const profileNewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/profiles/new',
  component: () => <ProfileEditor mode="create" />,
})

const profileEditRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/profiles/$profileId',
  component: () => <ProfileEditor mode="edit" />,
})

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/settings',
  component: SettingsPage,
})

const routeTree = rootRoute.addChildren([
  dashboardRoute,
  profilesRoute,
  profileNewRoute,
  profileEditRoute,
  settingsRoute,
])

export const router = createRouter({ routeTree })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
