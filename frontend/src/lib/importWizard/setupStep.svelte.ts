import { WorkspaceService } from '$lib/gen'
import type { ImportExecution } from './execution.svelte'

/**
 * Whether a finished import leaves a setup step behind it, and whether that is still
 * being decided.
 *
 * Known only once the run has fetched the export and the destination's data tables can
 * be compared against it, so it is false for the whole wizard until the import
 * finishes — which is exactly when it is first read. `undecided` matters as much as
 * `needed`: without it the run reads as finished with no fourth step, and Finish leaves
 * before the check comes back and discovers a data table that is missing.
 *
 * Shared by the wizard route and the in-workspace modal so the two cannot disagree
 * about whether an import is over.
 */
export function useSetupStep(
	getExecution: () => ImportExecution | undefined,
	getWorkspace: () => string | undefined
) {
	let needed = $state(false)
	let undecided = $state(false)

	$effect(() => {
		const execution = getExecution()
		const names = execution?.datatableNames ?? []
		const workspace = getWorkspace()
		if (!execution?.done || !workspace) {
			needed = false
			undecided = false
			return
		}
		// Every resource the project ships arrives as an empty stub, so any project with
		// resources has something to fill in. The step itself re-checks and shows only
		// what is genuinely outstanding, which is what makes a re-import quiet.
		if (execution.resourceCount > 0) {
			needed = true
			undecided = false
			return
		}
		if (names.length === 0) {
			needed = false
			undecided = false
			return
		}
		let cancelled = false
		undecided = true
		void WorkspaceService.listDataTables({ workspace })
			.then((tables) => {
				if (cancelled) return
				const present = new Set(tables.map((t) => t.name))
				needed = names.some((n) => !present.has(n))
			})
			.catch(() => {
				// Can't tell — don't invent a step the user then cannot complete.
				if (!cancelled) needed = false
			})
			.finally(() => {
				if (!cancelled) undecided = false
			})
		return () => (cancelled = true)
	})

	return {
		get needed() {
			return needed
		},
		get undecided() {
			return undecided
		}
	}
}
