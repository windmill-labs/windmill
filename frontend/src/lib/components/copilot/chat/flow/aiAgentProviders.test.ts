import { describe, expect, it } from 'vitest'
import {
	formatAiAgentProvidersPrompt,
	validateAiAgentProviders,
	type AiAgentProviderCatalog,
	type AiAgentProviderOption
} from './aiAgentProviders'

const ANTHROPIC: AiAgentProviderOption = {
	kind: 'anthropic',
	resourcePath: 'u/admin/anthropic',
	resourceRef: '$res:u/admin/anthropic',
	models: ['claude-sonnet-5', 'claude-opus-5'],
	modelsAreLive: true,
	customEndpoint: false
}

const COMPLETE: AiAgentProviderCatalog = { options: [ANTHROPIC], resourcesAreComplete: true }

function agentStep(provider: unknown) {
	return [{ id: 'agent', value: { type: 'aiagent', input_transforms: { provider } } }]
}

function staticProvider(value: unknown) {
	return agentStep({ type: 'static', value })
}

function anthropicProvider(model: string, resource = '$res:u/admin/anthropic') {
	return staticProvider({ kind: 'anthropic', resource, model })
}

describe('validateAiAgentProviders', () => {
	it('rejects a model the endpoint does not serve, and names the ones it does', () => {
		expect(() =>
			validateAiAgentProviders(anthropicProvider('claude-3-haiku-20240307'), COMPLETE)
		).toThrow(/claude-3-haiku-20240307.*not in the model listing.*claude-sonnet-5/s)
	})

	it('rejects a bare resource reference as the provider value, with no catalog needed', () => {
		expect(() =>
			validateAiAgentProviders(staticProvider('$res:u/admin/anthropic'), undefined)
		).toThrow(/must be an object/)
	})

	it('accepts a served model, a linked agent, and a runtime-resolved provider', () => {
		expect(() =>
			validateAiAgentProviders(anthropicProvider('claude-sonnet-5'), COMPLETE)
		).not.toThrow()
		expect(() =>
			validateAiAgentProviders(
				[{ id: 'agent', value: { type: 'aiagent', agent: 'u/admin/saved' } }],
				COMPLETE
			)
		).not.toThrow()
		expect(() =>
			validateAiAgentProviders(
				agentStep({ type: 'javascript', expr: 'flow_input.provider' }),
				COMPLETE
			)
		).not.toThrow()
	})

	it('leaves the model unchecked when the listing failed, so a stale fallback cannot block a write', () => {
		expect(() =>
			validateAiAgentProviders(anthropicProvider('claude-something-new'), {
				...COMPLETE,
				options: [{ ...ANTHROPIC, modelsAreLive: false }]
			})
		).not.toThrow()
	})

	it('reports, but does not reject, an unlisted model on a custom endpoint', () => {
		const warnings: string[] = []
		expect(() =>
			validateAiAgentProviders(
				anthropicProvider('team-sonnet'),
				{ ...COMPLETE, options: [{ ...ANTHROPIC, customEndpoint: true }] },
				warnings
			)
		).not.toThrow()
		expect(warnings).toHaveLength(1)
		expect(warnings[0]).toMatch(/team-sonnet/)
	})

	it('rejects an unknown resource only when the catalog holds every resource of the workspace', () => {
		expect(() =>
			validateAiAgentProviders(anthropicProvider('claude-sonnet-5', '$res:f/ai/other'), COMPLETE)
		).toThrow(/not one of this workspace's AI provider resources/)

		const warnings: string[] = []
		expect(() =>
			validateAiAgentProviders(
				anthropicProvider('claude-sonnet-5', '$res:f/ai/other'),
				{ ...COMPLETE, resourcesAreComplete: false },
				warnings
			)
		).not.toThrow()
		expect(warnings).toHaveLength(1)
	})
})

describe('formatAiAgentProvidersPrompt', () => {
	it('takes the workspace default only when one resource makes the choice unambiguous', () => {
		const defaultModel = { kind: 'anthropic', model: 'claude-sonnet-5' } as const
		expect(formatAiAgentProvidersPrompt({ ...COMPLETE, defaultModel })).toContain(
			'the workspace default'
		)
		expect(
			formatAiAgentProvidersPrompt({
				...COMPLETE,
				defaultModel,
				options: [ANTHROPIC, { ...ANTHROPIC, resourceRef: '$res:u/admin/other' }]
			})
		).toContain('askUserQuestion')
		expect(formatAiAgentProvidersPrompt(COMPLETE)).toContain('askUserQuestion')
	})
})
