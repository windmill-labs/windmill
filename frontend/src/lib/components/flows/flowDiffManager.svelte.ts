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
import { buildFlowTimeline, type FlowTimeline } from './flowDiff'
import { refreshStateStore, type StateStore } from '$lib/svelte5Utils.svelte'
import { getIndexInNestedModules } from '../copilot/chat/flow/utils'
import { dfs } from './previousResults'

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

	// State: input schemas for tracking schema changes
	let beforeInputSchema = $state<Record<string, any> | undefined>(undefined)
	let afterInputSchema = $state<Record<string, any> | undefined>(undefined)

	// State: module actions tracking changes (added/modified/removed/shadowed)
	let moduleActions = $state<Record<string, ModuleActionInfo>>({})

	// Derived: whether there are any pending changes
	const hasPendingChanges = $derived(Object.values(moduleActions).some((info) => info.pending))

	// onChange callback for notifying listeners when moduleActions change
	let onChangeCallback: ((actions: Record<string, ModuleActionInfo>) => void) | undefined

	// Auto-compute diff when beforeFlow or afterFlow changes
	$effect(() => {
		if (beforeFlow && afterFlow) {
			const timeline = buildFlowTimeline(beforeFlow.value, afterFlow, {
				markRemovedAsShadowed: false,
				markAsPending: true
			})

			// Update module actions
			const newActions = { ...timeline.afterActions }

			// Check for input schema changes
			if (beforeInputSchema && afterInputSchema) {
				const schemaChanged = JSON.stringify(beforeInputSchema) !== JSON.stringify(afterInputSchema)
				if (schemaChanged) {
					newActions['Input'] = {
						action: 'modified',
						pending: true,
						description: 'Input schema changed'
					}
				}
			}

			updateModuleActions(newActions)
		} else if (!beforeFlow) {
			// Clear module actions when no snapshot
			updateModuleActions({})
		}
	})

	/**
	 * Register a callback to be notified when moduleActions change
	 */
	function setOnChange(callback: (actions: Record<string, ModuleActionInfo>) => void) {
		onChangeCallback = callback
	}

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
	function setInputSchemas(before: Record<string, any> | undefined, after: Record<string, any> | undefined) {
		beforeInputSchema = before
		afterInputSchema = after
	}

	/**
	 * Clear the snapshot and all module actions
	 */
	function clearSnapshot() {
		beforeFlow = undefined
		afterFlow = undefined
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
	 * Compute diff between before flow and after flow, updating module actions
	 */
	function computeDiff(
		afterFlow: FlowValue,
		options: ComputeDiffOptions = {}
	): FlowTimeline | null {
		console.log('computeDiff', beforeFlow, afterFlow)
		if (!beforeFlow) {
			return null
		}

		const timeline = buildFlowTimeline(beforeFlow.value, afterFlow, {
			markRemovedAsShadowed: options.markRemovedAsShadowed ?? false,
			markAsPending: options.markAsPending ?? true
		})

		// Update module actions with the computed diff

		console.log('timeline.afterActions', timeline.afterActions)
		updateModuleActions(timeline.afterActions)

		return timeline
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
			const { modules } = getIndexInNestedModules(flowStore.val, id)
			const index = modules.findIndex((m) => m.id === id)
			if (index >= 0) {
				modules.splice(index, 1)
			}
		}

		refreshStateStore(flowStore)
	}

	/**
	 * Accept a module action (keep the changes)
	 * Simplified version - just marks as not pending
	 */
	function acceptModule(id: string, options: AcceptModuleOptions = {}) {
		if (!beforeFlow) {
			throw new Error('Cannot accept module without a beforeFlow snapshot')
		}

		const info = moduleActions[id]
		if (!info) return

		// Handle removed modules: delete them from the flow if flowStore is provided
		if (info.action === 'removed' && options.flowStore) {
			const actualId = id.startsWith('__') ? id.substring(2) : id
			deleteModuleFromFlow(actualId, options.flowStore)
		}

		// Mark as decided (no longer pending)
		if (moduleActions[id]) {
			updateModuleActions({
				...moduleActions,
				[id]: { ...moduleActions[id], pending: false }
			})
		}

		// Check if all actions are decided and clear snapshot if so
		checkAndClearSnapshot()
	}

	/**
	 * Reject a module action (revert the changes)
	 * Simplified version - reverts changes and marks as not pending
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
				// Remove the added module
				deleteModuleFromFlow(actualId, options.flowStore)
			} else if (action === 'removed') {
				// For removed modules, we would need to restore from snapshot
				// This is complex and might require full flow revert
				console.warn('Reverting removed module - requires full flow restore')
			} else if (action === 'modified') {
				// Revert to the old module state
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

		// Mark as decided
		if (moduleActions[id]) {
			updateModuleActions({
				...moduleActions,
				[id]: { ...moduleActions[id], pending: false }
			})
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
	 * Check if all module actions are decided (not pending) and clear snapshot if so
	 */
	function checkAndClearSnapshot() {
		const allDecided = Object.values(moduleActions).every((info) => !info.pending)
		if (allDecided) {
			clearSnapshot()
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
		get moduleActions() {
			return moduleActions
		},
		get hasPendingChanges() {
			return hasPendingChanges
		},

		// Snapshot management
		setSnapshot,
		setAfterFlow,
		setInputSchemas,
		clearSnapshot,
		getSnapshot,

		// Diff computation
		computeDiff,

		// Module actions management
		setModuleActions,
		getModuleActions,
		setOnChange,

		// Accept/reject operations
		acceptModule,
		rejectModule,
		acceptAll,
		rejectAll,
		revertToSnapshot
	}
}
