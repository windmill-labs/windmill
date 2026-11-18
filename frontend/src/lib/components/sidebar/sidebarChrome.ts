// Sidebar rail background. Kept here as the single source for every host that
// renders or mimics the sidebar (app layout, kitchen-sink harness).
// Both are CSS variables (see tailwind.config.cjs `--sidebar-bg-*`) so they adapt
// across dark variants and to the instance accent tint (accentColor.ts).
export const SIDEBAR_BG = 'var(--sidebar-bg-light)'
export const SIDEBAR_BG_DARK = 'var(--sidebar-bg-dark)'
