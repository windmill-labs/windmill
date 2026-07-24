export type DarkModeVariant = 'default' | 'github'

const DARK_MODE_VARIANT_KEY = 'dark-mode-variant'

export function getDarkModeVariant(): DarkModeVariant {
	return window.localStorage.getItem(DARK_MODE_VARIANT_KEY) === 'github' ? 'github' : 'default'
}

// The `github-dark` class only takes effect when `dark` is also present (see tailwind.config.cjs).
// It lives on <html> permanently so it survives the many independent `dark` toggle sites.
export function applyDarkModeVariant(variant: DarkModeVariant = getDarkModeVariant()): void {
	document.documentElement.classList.toggle('github-dark', variant === 'github')
}

export function setDarkModeVariant(variant: DarkModeVariant): void {
	window.localStorage.setItem(DARK_MODE_VARIANT_KEY, variant)
	applyDarkModeVariant(variant)
}
