import type {
	AppAIChatHelpers,
	AppFiles,
	BackendRunnable,
	LintResult,
	SelectedContext
} from '../../app/core'

/**
 * Creates an empty lint result (no errors or warnings).
 */
function createEmptyLintResult(): LintResult {
	return {
		errorCount: 0,
		warningCount: 0,
		errors: { frontend: {}, backend: {} },
		warnings: { frontend: {}, backend: {} }
	}
}

/**
 * Creates mock AppAIChatHelpers for eval testing.
 * Tracks app files state in memory and allows tool functions to modify it.
 */
export function createAppEvalHelpers(
	initialFrontend: Record<string, string> = {},
	initialBackend: Record<string, BackendRunnable> = {}
) {
	// In-memory state
	let frontend: Record<string, string> = { ...initialFrontend }
	let backend: Record<string, BackendRunnable> = { ...initialBackend }
	let snapshotId = 0
	const snapshots: Map<number, { frontend: Record<string, string>; backend: Record<string, BackendRunnable> }> = new Map()

	const helpers: AppAIChatHelpers = {
		// Frontend file operations
		listFrontendFiles: () => Object.keys(frontend),

		getFrontendFile: (path: string) => frontend[path],

		getFrontendFiles: () => ({ ...frontend }),

		setFrontendFile: (path: string, content: string) => {
			frontend[path] = content
			// Return mock lint result - in real usage this would validate the file
			return createEmptyLintResult()
		},

		deleteFrontendFile: (path: string) => {
			delete frontend[path]
		},

		// Backend runnable operations
		listBackendRunnables: () => {
			return Object.entries(backend).map(([key, runnable]) => ({
				key,
				name: runnable.name
			}))
		},

		getBackendRunnable: (key: string) => backend[key],

		getBackendRunnables: () => ({ ...backend }),

		setBackendRunnable: async (key: string, runnable: BackendRunnable) => {
			backend[key] = runnable
			// Return mock lint result - in real usage this would validate the runnable
			return createEmptyLintResult()
		},

		deleteBackendRunnable: (key: string) => {
			delete backend[key]
		},

		// Combined view
		getFiles: (): AppFiles => ({
			frontend: { ...frontend },
			backend: { ...backend }
		}),

		getSelectedContext: (): SelectedContext => ({
			type: 'none'
		}),

		// Snapshot management
		snapshot: () => {
			const id = ++snapshotId
			snapshots.set(id, {
				frontend: { ...frontend },
				backend: { ...backend }
			})
			return id
		},

		revertToSnapshot: (id: number) => {
			const snap = snapshots.get(id)
			if (snap) {
				frontend = { ...snap.frontend }
				backend = { ...snap.backend }
			}
		},

		// Linting
		lint: () => {
			// Return mock lint result - no actual linting in eval
			return createEmptyLintResult()
		},

		// Data table operations (mock implementation for testing)
		getDatatables: async () => {
			// Return empty array for eval testing - no real datatables in test context
			return []
		},

		getAvailableDatatableNames: () => {
			// Return empty array for eval testing - no real datatables in test context
			return []
		},

		execDatatableSql: async (
			_datatableName: string,
			_sql: string,
			_newTable?: { schema: string; name: string }
		) => {
			// Return success with empty result for eval testing
			return { success: true, result: [] }
		},

		addTableToWhitelist: (
			_datatableName: string,
			_schemaName: string,
			_tableName: string
		) => {
			// No-op for eval testing - tables are not tracked in test context
		}
	}

	return {
		helpers,
		getFiles: (): AppFiles => ({
			frontend: { ...frontend },
			backend: { ...backend }
		}),
		getFrontend: () => ({ ...frontend }),
		getBackend: () => ({ ...backend })
	}
}
