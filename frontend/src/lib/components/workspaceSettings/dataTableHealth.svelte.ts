import { WorkspaceService, type DataTableHealth } from '$lib/gen'

/**
 * Connection health for every data table in the workspace.
 *
 * One request rather than one per row: the backend probes them concurrently and
 * caps each, so an unreachable database costs the page a bounded wait instead of
 * however long its driver takes to give up.
 */
export function useDataTableHealth(workspace: () => string | undefined) {
	let health = $state<Record<string, DataTableHealth> | undefined>(undefined)
	let loading = $state(false)

	async function load() {
		const ws = workspace()
		if (!ws) return
		loading = true
		try {
			health = await WorkspaceService.dataTableHealth({ workspace: ws })
		} catch {
			// A failed probe run is not a failed page: the rows still render, they
			// just have nothing to say about health.
			health = undefined
		} finally {
			loading = false
		}
	}

	$effect(() => {
		workspace()
		load()
	})

	return {
		get loading() {
			return loading
		},
		get current() {
			return health
		},
		/** After the wizard finishes, or a rename, or a repoint. */
		refetch: load
	}
}
