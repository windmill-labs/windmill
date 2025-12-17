import type { Flow, FlowModule, InputTransform } from '$lib/gen'
import { dfs } from './dfs'

/**
 * Migrates legacy messages_context_length to new history format
 * for AI agent modules in a flow.
 *
 * Legacy format:
 *   input_transforms: { messages_context_length: { type: 'static', value: 10 } }
 *
 * New format:
 *   input_transforms: { history: { type: 'static', value: { kind: 'auto', context_length: 10 } } }
 */
export function migrateFlowLegacyFormats(flow: Flow): Flow {
	if (!flow.value?.modules) return flow

	dfs(flow.value.modules, (module) => {
		migrateAiAgentModule(module)
	})

	return flow
}

function migrateAiAgentModule(module: FlowModule): void {
	console.log('[here] migrateAiAgentModule', module)
	const value = module.value
	if (value.type !== 'aiagent') return

	const inputTransforms = value.input_transforms as Record<string, InputTransform> | undefined
	if (!inputTransforms) return

	// Check if this has the legacy format
	if ('messages_context_length' in inputTransforms && !('history' in inputTransforms)) {
		const legacyValue = inputTransforms.messages_context_length

		// Create new history field with auto mode
		if (legacyValue.type === 'static') {
			inputTransforms.history = {
				type: 'static',
				value: {
					kind: 'auto',
					context_length: legacyValue.value ?? 0
				}
			}
		} else if (legacyValue.type === 'javascript') {
			// For dynamic expressions, wrap in the new format
			inputTransforms.history = {
				type: 'javascript',
				expr: `{ kind: 'auto', context_length: ${legacyValue.expr} }`
			}
		}

		// Remove the legacy field
		delete inputTransforms.messages_context_length
	}
}
