export const SVELVET_CONTEXT_KEY = 'svelvet_settings_context' as const

export type SvelvetSettingsContext = {
	/** Sets the height of the canvas to `100%` instead of an arbitrary value in pixels */
	fullHeight: boolean
}
