import { untrack } from 'svelte'
import type { FlowModule } from '$lib/gen'
import { type FlowGroup, type GraphGroup } from './groupEditor.svelte'
import type { StateStore } from '$lib/utils'
import { getAllModules } from '../flows/flowExplorer'
import { computeGroupModuleIds } from './groupDetectionUtils'
import { stateSnapshot } from '$lib/svelte5Utils.svelte'
import {
	buildStructureTree,
	deriveGroupsFromStructure,
	applyStructureToModules,
	removeEmptyGroups,
	type FlowStructureNode
} from './flowStructure'

export type ExtendedOpenFlow = {
	value: {
		modules: FlowModule[]
		groups?: FlowGroup[]
		[key: string]: any
	}
	[key: string]: any
}

/**
 * Reactive read-only view of the flow structure tree.
 * The tree is always derived from flowStore (single source of truth).
 * Mutations go through prepareMutation: snapshot → mutate → clean empty groups → commit.
 */
export class GroupedModulesProxy {
	#items = $state<FlowStructureNode[]>([])
	#error = $state<unknown>(undefined)
	#flowStore: StateStore<ExtendedOpenFlow>

	constructor(flowStore: StateStore<ExtendedOpenFlow>) {
		this.#flowStore = flowStore
		this.rebuild()

		// Rebuild tree whenever store changes (undo/load/mutation)
		$effect(() => {
			void flowStore.val.value.modules
			void flowStore.val.value.groups
			untrack(() => this.rebuild())
		})
	}

	/** Reactive access to the structure tree (read-only view) */
	get items(): FlowStructureNode[] {
		return this.#items
	}

	/** Reactive access to build errors */
	get error(): unknown {
		return this.#error
	}

	/**
	 * Prepare a structural mutation without writing to the store yet.
	 * Returns the list of groups that became empty (already removed from the snapshot)
	 * and a `commit` function that writes the result to the store.
	 *
	 * If no groups were emptied, the caller can commit immediately.
	 * If groups were emptied, the caller should show a confirmation modal
	 * and call commit() only on user confirmation.
	 */
	prepareMutation(
		mutate: (tree: FlowStructureNode[]) => void,
		extraModules?: FlowModule[]
	): { emptiedGroups: FlowGroup[]; commit: () => void } {
		const snapshot = $state.snapshot(this.#items) as FlowStructureNode[]
		mutate(snapshot)

		// Clean up empty groups and collect which ones were removed
		const emptiedGroups = removeEmptyGroups(snapshot)

		const commit = () => {
			// Build moduleMap lazily at commit time so it reflects the latest store state
			const moduleMap = new Map<string, FlowModule>()
			for (const m of getAllModules(this.#flowStore.val.value.modules)) {
				moduleMap.set(m.id, m)
			}
			if (extraModules) {
				for (const m of extraModules) {
					moduleMap.set(m.id, m)
				}
			}
			this.#flowStore.val.value.modules = applyStructureToModules(snapshot, moduleMap)
			this.#flowStore.val.value.groups = deriveGroupsFromStructure(snapshot)
		}

		return { emptiedGroups, commit }
	}

	/**
	 * Convenience: prepare + auto-commit. Only use for mutations that cannot
	 * empty groups (e.g. inserts). Throws if groups are unexpectedly emptied.
	 * For mutations that may empty groups, use prepareMutation() directly.
	 */
	applyTreeMutation(
		mutate: (tree: FlowStructureNode[]) => void,
		extraModules?: FlowModule[]
	): void {
		const { emptiedGroups, commit } = this.prepareMutation(mutate, extraModules)
		if (emptiedGroups.length > 0) {
			console.error('applyTreeMutation: unexpected empty groups', emptiedGroups)
		}
		commit()
	}

	/** Rebuild from flowStore */
	private rebuild(): void {
		const modules = stateSnapshot(this.#flowStore.val.value.modules) as FlowModule[]
		const allGroups = this.#flowStore.val.value.groups ?? []
		const allModules = getAllModules(modules)
		const graphGroups: GraphGroup[] = allGroups.map((g) => ({
			...g,
			moduleIds: computeGroupModuleIds(g.start_id, g.end_id, allModules)
		}))
		try {
			this.#items = buildStructureTree(modules, graphGroups)
			this.#error = undefined
		} catch (e) {
			this.#error = e
		}
	}
}
