/**
 * Flow Diff Manager
 *
 * A reusable store for managing flow diff state, module actions, and accept/reject operations.
 * This decouples diff management from specific UI components (like AI chat) and makes it
 * available for any use case that needs to track and apply flow changes.
 */

import type { ExtendedOpenFlow } from './types'
import type { FlowModule, FlowValue } from '$lib/gen'
import type { ModuleActionInfo } from '../copilot/chat/flow/core'
import { buildFlowTimeline } from './flowDiff'
import { refreshStateStore } from '$lib/svelte5Utils.svelte'
import type { StateStore } from '$lib/utils'
import { getIndexInNestedModules } from '../copilot/chat/flow/utils'
import { dfs } from './previousResults'
import { getAllSubmodules } from './flowExplorer'
import type DiffDrawer from '../DiffDrawer.svelte'

/**
 * Options for accepting a module action (simplified)
 */
export type AcceptModuleOptions = {
	/** The current flow store (used for applying changes) */
	flowStore?: StateStore<ExtendedOpenFlow>
}

/**
 * Options for rejecting a module action (simplified)
 */
export type RejectModuleOptions = {
	/** The current flow store (used for reverting changes) */
	flowStore?: StateStore<ExtendedOpenFlow>
}

/**
 * Options for computing diff
 */
export type ComputeDiffOptions = {
	/** Mark all changes as pending (requiring user approval) */
	markAsPending?: boolean
	/** Mark removed modules as shadowed instead of removed (for visualization) */
	markRemovedAsShadowed?: boolean
}

/**
 * Creates a flow diff manager instance
 */
export function createFlowDiffManager() {
	// State: snapshot of flow before changes
	let beforeFlow = $state<ExtendedOpenFlow | undefined>(undefined)

	// State: current flow after changes
	let afterFlow = $state<FlowValue | undefined>(undefined)

	// State: merged flow containing both original and modified/removed modules
	let mergedFlow = $state<FlowValue | undefined>(undefined)

	// State: input schemas for tracking schema changes
	let beforeInputSchema = $state<Record<string, any> | undefined>(undefined)
	let afterInputSchema = $state<Record<string, any> | undefined>(undefined)

	// State: whether to mark removed modules as shadowed (for side-by-side view)
	let markRemovedAsShadowed = $state(false)

	// State: module actions tracking changes (added/modified/removed/shadowed)
	let moduleActions = $state<Record<string, ModuleActionInfo>>({})

	// State: reference to DiffDrawer component for showing module diffs
	let diffDrawer = $state<DiffDrawer | undefined>(undefined)

	// Derived: whether there are any pending changes
	const hasPendingChanges = $derived(Object.values(moduleActions).some((info) => info.pending))

	// onChange callback for notifying listeners when moduleActions change
	let onChangeCallback: ((actions: Record<string, ModuleActionInfo>) => void) | undefined

	// Auto-compute diff when beforeFlow or afterFlow changes
	$effect(() => {
		if (beforeFlow && afterFlow) {
			if (hasPendingChanges) {
				console.log('HERE: [flowDiffManager $effect] hasPendingChanges', hasPendingChanges)
				return
			}
			const timeline = buildFlowTimeline(beforeFlow.value, afterFlow, {
				markRemovedAsShadowed: markRemovedAsShadowed,
				markAsPending: true
			})

			// Store the merged flow for rendering
			mergedFlow = timeline.mergedFlow

			// Update module actions
			const newActions = { ...timeline.afterActions }

			// Check for input schema changes
			if (beforeInputSchema && afterInputSchema) {
				const schemaChanged = JSON.stringify(beforeInputSchema) !== JSON.stringify(afterInputSchema)
				if (schemaChanged) {
					newActions['Input'] = {
						action: 'modified',
						pending: true
					}
				}
			}

			updateModuleActions(newActions)
		} else if (!beforeFlow) {
			// Clear module actions and merged flow when no snapshot
			mergedFlow = undefined
			updateModuleActions({})
		}
	})

	/**
	 * Helper to update moduleActions and notify listeners
	 */
	function updateModuleActions(newActions: Record<string, ModuleActionInfo>) {
		moduleActions = newActions
		console.log('updateModuleActions', newActions)
		console.log('onChangeCallback', onChangeCallback)
		onChangeCallback?.(newActions)
	}

	/**
	 * Set the before flow snapshot for diff computation
	 */
	function setSnapshot(flow: ExtendedOpenFlow | undefined) {
		beforeFlow = flow
		if (flow) {
			beforeInputSchema = flow.schema
		} else {
			beforeInputSchema = undefined
		}
	}

	/**
	 * Set the after flow (current state) for diff computation
	 */
	function setAfterFlow(flow: FlowValue | undefined) {
		afterFlow = flow
	}

	/**
	 * Set input schemas for tracking schema changes
	 */
	function setInputSchemas(
		before: Record<string, any> | undefined,
		after: Record<string, any> | undefined
	) {
		beforeInputSchema = before
		afterInputSchema = after
	}

	/**
	 * Set whether to mark removed modules as shadowed (for side-by-side view)
	 */
	function setMarkRemovedAsShadowed(value: boolean) {
		markRemovedAsShadowed = value
	}

	/**
	 * Clear the snapshot and all module actions
	 */
	function clearSnapshot() {
		beforeFlow = undefined
		afterFlow = undefined
		mergedFlow = undefined
		beforeInputSchema = undefined
		afterInputSchema = undefined
		updateModuleActions({})
	}

	/**
	 * Get the current before flow snapshot
	 */
	function getSnapshot(): ExtendedOpenFlow | undefined {
		return beforeFlow
	}

	/**
	 * Set module actions directly (useful when actions are computed elsewhere)
	 */
	function setModuleActions(actions: Record<string, ModuleActionInfo>) {
		updateModuleActions(actions)
	}

	/**
	 * Get current module actions
	 */
	function getModuleActions(): Record<string, ModuleActionInfo> {
		return moduleActions
	}

	/**
	 * Helper to get a module from a flow by ID
	 */
	function getModuleFromFlow(id: string, flow: ExtendedOpenFlow): FlowModule | undefined {
		if (id === 'preprocessor') {
			return flow.value.preprocessor_module
		} else if (id === 'failure') {
			return flow.value.failure_module
		} else {
			return dfs(id, flow, false)[0]
		}
	}

	/**
	 * Helper to delete a module from the flow
	 */
	function deleteModuleFromFlow(
		id: string,
		flowStore: StateStore<ExtendedOpenFlow>,
		selectNextIdFn?: (id: string) => void
	) {
		selectNextIdFn?.(id)

		if (id === 'preprocessor') {
			flowStore.val.value.preprocessor_module = undefined
		} else if (id === 'failure') {
			flowStore.val.value.failure_module = undefined
		} else {
			console.log('HERE deleteModuleFromFlow', id, flowStore.val)
			const { modules } = getIndexInNestedModules(flowStore.val, id)
			const index = modules.findIndex((m) => m.id === id)
			if (index >= 0) {
				modules.splice(index, 1)
			}
		}

		refreshStateStore(flowStore)
	}

	/**
	 * Helper to remove a module and all its children from tracking
	 */
	function removeModuleAndChildren(
		id: string,
		currentActions: Record<string, ModuleActionInfo>
	): Record<string, ModuleActionInfo> {
		const newActions = { ...currentActions }

		// Remove the parent module
		delete newActions[id]

		// Get the module from the flow to find children
		const flow = mergedFlow ? { value: mergedFlow, summary: '' } : beforeFlow
		if (flow) {
			const actualId = id.startsWith('__') ? id.substring(2) : id
			const module = getModuleFromFlow(actualId, flow as ExtendedOpenFlow)

			if (module) {
				// Get all child module IDs recursively
				const childIds = getAllSubmodules(module)
					.flat()
					.map((m) => m.id)

				// Remove all children from tracking
				childIds.forEach((childId) => {
					delete newActions[childId]
					// Also try with __ prefix in case it's a shadowed/removed module
					delete newActions[`__${childId}`]
				})
			}
		}

		return newActions
	}

	/**
	 * Accept a module action (keep the changes)
	 * Removes the action from tracking after acceptance
	 */
	function acceptModule(id: string, options: AcceptModuleOptions = {}) {
		if (!beforeFlow) {
			throw new Error('Cannot accept module without a beforeFlow snapshot')
		}

		const info = moduleActions[id]
		if (!info) return

		// Handle removed modules: delete them from mergedFlow if present
		// (flowStore already has the module removed since changes are applied directly)
		if (info.action === 'removed' && options.flowStore) {
			const actualId = id.startsWith('__') ? id.substring(2) : id
			// delete from merged flow
			if (mergedFlow) {
				const { modules } = getIndexInNestedModules({ value: mergedFlow, summary: '' }, actualId)
				const index = modules.findIndex((m) => m.id === actualId)
				if (index >= 0) {
					modules.splice(index, 1)
				}
			}
		}

		// Remove the action from tracking (no longer needs user decision)
		// Also remove all children from tracking
		if (moduleActions[id]) {
			const newActions = removeModuleAndChildren(id, moduleActions)
			updateModuleActions(newActions)
		}

		// Check if all actions are decided and clear snapshot if so
		checkAndClearSnapshot()
	}

	/**
	 * Reject a module action (revert the changes)
	 * Removes the action from tracking after rejection
	 */
	function rejectModule(id: string, options: RejectModuleOptions = {}) {
		if (!beforeFlow) {
			throw new Error('Cannot reject module without a beforeFlow snapshot')
		}

		const actualId = id.startsWith('__') ? id.substring(2) : id
		const info = moduleActions[id]

		if (!info) return

		const action = info.action

		// Only perform revert operations if flowStore is provided
		if (options.flowStore) {
			// Handle different action types
			if (id === 'Input') {
				// Revert input schema changes
				options.flowStore.val.schema = beforeFlow.schema
			} else if (action === 'added') {
				// Remove the added module from flowStore
				deleteModuleFromFlow(actualId, options.flowStore)

				// ALSO remove from merged flow for immediate visual update
				if (mergedFlow) {
					const { modules } = getIndexInNestedModules({ value: mergedFlow, summary: '' }, actualId)
					const index = modules.findIndex((m) => m.id === actualId)
					if (index >= 0) {
						modules.splice(index, 1)
					}
				}
			} else if (action === 'removed') {
				// Restore the removed module from beforeFlow to flowStore
				const oldModule = getModuleFromFlow(actualId, beforeFlow)
				if (oldModule && options.flowStore) {
					// Find where to insert the module back
					const { modules } = getIndexInNestedModules(options.flowStore.val, actualId)
					// Add the module back - it was removed so we need to restore it
					// Find position from beforeFlow
					const beforeModules = beforeFlow.value.modules
					const beforeIndex = beforeModules.findIndex((m) => m.id === actualId)
					if (beforeIndex >= 0) {
						modules.splice(beforeIndex, 0, $state.snapshot(oldModule))
					} else {
						// Fallback: add at the end
						modules.push($state.snapshot(oldModule))
					}
				}
			} else if (action === 'modified') {
				// Revert to the old module state in flowStore
				const oldModule = getModuleFromFlow(actualId, beforeFlow)
				const newModule = getModuleFromFlow(actualId, options.flowStore.val)

				if (oldModule && newModule) {
					// Restore the old module state
					Object.keys(newModule).forEach((k) => delete (newModule as any)[k])
					Object.assign(newModule, $state.snapshot(oldModule))
				}
			}

			refreshStateStore(options.flowStore)
		}

		// Remove the action from tracking (no longer needs user decision)
		// Also remove all children from tracking
		if (moduleActions[id]) {
			const newActions = removeModuleAndChildren(id, moduleActions)
			updateModuleActions(newActions)
		}

		// Check if all actions are decided and clear snapshot if so
		checkAndClearSnapshot()
	}

	/**
	 * Accept all pending module actions
	 */
	function acceptAll(options: AcceptModuleOptions) {
		const ids = Object.keys(moduleActions)
		for (const id of ids) {
			if (moduleActions[id]?.pending) {
				acceptModule(id, options)
			}
		}
	}

	/**
	 * Reject all pending module actions (in reverse order for nested modules)
	 */
	function rejectAll(options: RejectModuleOptions) {
		const ids = Object.keys(moduleActions)
		// Process in reverse to handle nested modules correctly
		for (let i = ids.length - 1; i >= 0; i--) {
			if (moduleActions[ids[i]]?.pending) {
				rejectModule(ids[i], options)
			}
		}
	}

	/**
	 * Revert the entire flow to the snapshot
	 */
	function revertToSnapshot(flowStore: StateStore<ExtendedOpenFlow>) {
		if (!beforeFlow) return

		flowStore.val = beforeFlow
		refreshStateStore(flowStore)
		clearSnapshot()
	}

	/**
	 * Check if all module actions are decided (removed) and clear snapshot if so
	 */
	function checkAndClearSnapshot() {
		if (Object.keys(moduleActions).length === 0) {
			clearSnapshot()
		}
	}

	/**
	 * Set the DiffDrawer instance for showing module diffs
	 */
	function setDiffDrawer(drawer: DiffDrawer | undefined) {
		diffDrawer = drawer
	}

	/**
	 * Show diff for a specific module or Input schema
	 */
	function showModuleDiff(moduleId: string) {
		if (!diffDrawer || !beforeFlow) return

		if (moduleId === 'Input') {
			// Show input schema diff
			diffDrawer.openDrawer()
			diffDrawer.setDiff({
				mode: 'simple',
				title: 'Flow Input Schema Diff',
				original: { schema: beforeFlow.schema ?? {} },
				current: { schema: afterInputSchema ?? {} }
			})
		} else {
			// Show module diff
			const beforeModule = getModuleFromFlow(moduleId, beforeFlow)
			const afterModule = afterFlow
				? dfs(moduleId, { value: afterFlow, summary: '' }, false)[0]
				: undefined

			if (beforeModule && afterModule) {
				diffDrawer.openDrawer()
				diffDrawer.setDiff({
					mode: 'simple',
					title: `Module Diff: ${moduleId}`,
					original: beforeModule,
					current: afterModule
				})
			}
		}
	}

	return {
		// State accessors
		get beforeFlow() {
			return beforeFlow
		},
		get afterFlow() {
			return afterFlow
		},
		get mergedFlow() {
			return mergedFlow
		},
		get moduleActions() {
			return moduleActions
		},
		get hasPendingChanges() {
			return hasPendingChanges
		},
		get afterInputSchema() {
			return afterInputSchema
		},

		// Snapshot management
		setSnapshot,
		setAfterFlow,
		setInputSchemas,
		setMarkRemovedAsShadowed,
		clearSnapshot,
		getSnapshot,

		// Module actions management
		setModuleActions,
		getModuleActions,

		// Accept/reject operations
		acceptModule,
		rejectModule,
		acceptAll,
		rejectAll,
		revertToSnapshot,

		// Diff drawer management
		setDiffDrawer,
		showModuleDiff
	}
}
