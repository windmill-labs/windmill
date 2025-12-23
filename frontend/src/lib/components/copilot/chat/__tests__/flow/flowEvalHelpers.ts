import type { FlowAIChatHelpers } from '../../flow/core'
import type { FlowModule, InputTransform } from '$lib/gen'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'
import { findModuleById } from '../../shared'
import { inlineScriptStore, restoreInlineScriptReferences } from '../../flow/inlineScriptsUtils'

/**
 * Creates mock FlowAIChatHelpers for eval testing.
 * Tracks flow state in memory and allows tool functions to modify it.
 */
export function createFlowEvalHelpers(
	initialModules: FlowModule[] = [],
	initialSchema?: Record<string, any>
) {
	let flow: ExtendedOpenFlow = {
		value: { modules: structuredClone(initialModules) },
		summary: '',
		schema: initialSchema ?? {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			properties: {},
			required: [],
			type: 'object'
		}
	}

	const helpers: FlowAIChatHelpers = {
		getFlowAndSelectedId: () => ({ flow, selectedId: '' }),

		getModules: (id?: string) => {
			if (!id) return flow.value.modules
			const module = findModuleById(flow.value.modules, id)
			return module ? [module] : []
		},

		setSnapshot: () => {
			// No-op for eval - we don't need snapshot tracking
		},

		revertToSnapshot: () => {
			// No-op for eval
		},

		setCode: async (id: string, code: string) => {
			const module = findModuleById(flow.value.modules, id)
			if (module && module.value.type === 'rawscript') {
				module.value.content = code
			}
			// Keep store coherent for subsequent set_flow_json calls with references
			inlineScriptStore.set(id, code)
		},

		setFlowJson: async (
			modules: FlowModule[] | undefined,
			schema: Record<string, any> | undefined
		) => {
			if (modules) {
				// Restore inline script references back to full content
				const restoredModules = restoreInlineScriptReferences(modules)
				flow.value.modules = restoredModules
			}

			// Update schema if provided
			if (schema !== undefined) {
				flow.schema = schema
			}
		},

		getFlowInputsSchema: async () => flow.schema ?? {},

		updateExprsToSet: (_id: string, _inputTransforms: Record<string, InputTransform>) => {
			// No-op for eval - UI-only functionality
		},

		acceptAllModuleActions: () => {
			// No-op for eval
		},

		rejectAllModuleActions: () => {
			// No-op for eval
		},

		hasPendingChanges: () => false,

		selectStep: (_id: string) => {
			// No-op for eval
		},

		testFlow: async () => {
			// Return mock job ID - we don't actually run flows in eval
			return 'mock-job-id-' + Date.now()
		},

		getLintErrors: async () => {
			// Return empty lint result for eval
			return { errorCount: 0, warningCount: 0, errors: [], warnings: [] }
		}
	}

	return {
		helpers,
		getFlow: () => flow,
		getModules: () => flow.value.modules
	}
}
