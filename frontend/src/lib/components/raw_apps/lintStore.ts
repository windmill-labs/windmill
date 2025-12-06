import { writable, get } from 'svelte/store'

export interface MonacoLintError {
	message: string
	severity: 'error' | 'warning'
	startLineNumber: number
	startColumn: number
	endLineNumber: number
	endColumn: number
}

export interface RawAppLintStore {
	/** Whether lint collection mode is enabled (true when in RawAppEditor context) */
	enabled: boolean
	/** Errors grouped by runnable key */
	errors: Record<string, MonacoLintError[]>
	/** Warnings grouped by runnable key */
	warnings: Record<string, MonacoLintError[]>
}

function createRawAppLintStore() {
	const { subscribe, set, update } = writable<RawAppLintStore>({
		enabled: false,
		errors: {},
		warnings: {}
	})

	return {
		subscribe,

		/** Enable lint collection mode */
		enable() {
			update((s) => ({ ...s, enabled: true }))
		},

		/** Disable lint collection mode and clear all diagnostics */
		disable() {
			set({ enabled: false, errors: {}, warnings: {} })
		},

		/** Check if lint collection is enabled */
		isEnabled(): boolean {
			return get({ subscribe }).enabled
		},

		/** Set diagnostics for a specific runnable key */
		setDiagnostics(runnableKey: string, diagnostics: MonacoLintError[]) {
			update((s) => {
				const errors: MonacoLintError[] = []
				const warnings: MonacoLintError[] = []

				for (const diag of diagnostics) {
					if (diag.severity === 'error') {
						errors.push(diag)
					} else {
						warnings.push(diag)
					}
				}

				return {
					...s,
					errors: { ...s.errors, [runnableKey]: errors },
					warnings: { ...s.warnings, [runnableKey]: warnings }
				}
			})
		},

		/** Clear diagnostics for a specific runnable key */
		clearDiagnostics(runnableKey: string) {
			update((s) => {
				const newErrors = { ...s.errors }
				const newWarnings = { ...s.warnings }
				delete newErrors[runnableKey]
				delete newWarnings[runnableKey]
				return { ...s, errors: newErrors, warnings: newWarnings }
			})
		},

		/** Clear all diagnostics */
		clearAll() {
			update((s) => ({ ...s, errors: {}, warnings: {} }))
		},

		/** Get current snapshot of diagnostics */
		getSnapshot(): RawAppLintStore {
			return get({ subscribe })
		}
	}
}

export const rawAppLintStore = createRawAppLintStore()
