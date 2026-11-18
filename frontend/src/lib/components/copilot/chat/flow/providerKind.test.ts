import { describe, expect, it } from 'vitest'
import { flowModulesSchema } from './openFlowZod.gen'

// Guards against the generated copilot flow Zod schema (openFlowZod.gen.ts)
// drifting from the AIProviderKind enum in openflow.openapi.yaml. A missing
// provider kind here silently rejects AI-generated flow edits for that provider
// in the copilot flow-editing path (validateFlowModules -> flowModulesSchema).
function aiAgentModuleWithProviderKind(kind: string) {
	return {
		id: 'agent',
		value: {
			type: 'aiagent',
			tools: [],
			input_transforms: {
				provider: {
					type: 'static',
					value: { kind, resource: '$res:u/admin/foundry', model: 'gpt-4o' }
				},
				user_message: { type: 'static', value: 'hello' },
				output_type: { type: 'static', value: 'text' }
			}
		}
	}
}

describe('copilot flow module validation - AI agent provider kind', () => {
	it('accepts azure_foundry (and the existing azure_openai baseline)', () => {
		expect(
			flowModulesSchema.safeParse([aiAgentModuleWithProviderKind('azure_openai')]).success
		).toBe(true)
		expect(
			flowModulesSchema.safeParse([aiAgentModuleWithProviderKind('azure_foundry')]).success
		).toBe(true)
	})

	it('still rejects an unknown provider kind (enum is actually enforced)', () => {
		expect(
			flowModulesSchema.safeParse([aiAgentModuleWithProviderKind('not_a_real_provider')]).success
		).toBe(false)
	})
})
