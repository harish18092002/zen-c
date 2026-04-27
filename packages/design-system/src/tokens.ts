export const colors = {
  background: {
    primary: 'var(--color-bg-primary)',
    secondary: 'var(--color-bg-secondary)',
    elevated: 'var(--color-bg-elevated)',
  },
  text: {
    primary: 'var(--color-text-primary)',
    secondary: 'var(--color-text-secondary)',
    muted: 'var(--color-text-muted)',
  },
  accent: {
    focus: 'var(--color-accent-focus)',
    break: 'var(--color-accent-break)',
    strict: 'var(--color-accent-strict)',
    warning: 'var(--color-accent-warning)',
    error: 'var(--color-accent-error)',
  },
} as const

export const spacing = {
  xs: '4px',
  sm: '8px',
  md: '16px',
  lg: '24px',
  xl: '32px',
  xxl: '48px',
} as const

export const radii = {
  sm: '4px',
  md: '8px',
  lg: '16px',
  full: '9999px',
} as const

export const typography = {
  fontFamily: {
    sans: 'system-ui, -apple-system, BlinkMacSystemFont, sans-serif',
    mono: 'ui-monospace, SFMono-Regular, monospace',
  },
  fontSize: {
    xs: '12px',
    sm: '14px',
    md: '16px',
    lg: '20px',
    xl: '24px',
    xxl: '32px',
    hero: '56px',
  },
  fontWeight: {
    normal: '400',
    medium: '500',
    semibold: '600',
    bold: '700',
  },
  lineHeight: {
    tight: '1.2',
    normal: '1.5',
    relaxed: '1.75',
  },
} as const

export const transitions = {
  fast: '100ms ease',
  normal: '200ms ease',
  slow: '350ms ease',
} as const
