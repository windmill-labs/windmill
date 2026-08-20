import { describe, expect, it } from 'vitest'
import {
	formatAiAgentProvidersPrompt,
	validateAiAgentProviders,
	type AiAgentProviderOption
} from './aiAgentProviders'

const ANTHROPIC: AiAgentProviderOption = {
	kind: 'anthropic',
	resourcePath: 'u/admin/anthropic',
	resourceRef: '$res:u/admin/anthropic',
	models: ['claude-sonnet-5', 'claude-opus-5'],
	modelsAreLive: true,
	customEndpoint: false,
	configuredModels: ['claude-sonnet-5']
}

function agentModule(provider: unknown) {
	return [{ id: 'agent', value: { type: 'aiagent', input_transforms: { provider } } }]
}

function staticProvider(value: unknown) {
	return agentModule({ type: 'static', value })
}

describe('validateAiAgentProviders', () => {
	it('rejects a model the provider does not serve, and names the ones it does', () => {
		expect(() =>
			validateAiAgentProviders(
				staticProvider({
					kind: 'anthropic',
					resource: '$res:u/admin/anthropic',
					model: 'claude-3-haiku-20240307'
				}),
				[ANTHROPIC]
			)
		).toThrow(/claude-3-haiku-20240307.*not in the model listing.*claude-sonnet-5/s)
	})

	it('rejects a bare resource reference as the provider value, with no catalog needed', () => {
		expect(() => validateAiAgentProviders(staticProvider('$res:u/admin/anthropic'), [])).toThrow(
			/must be an object/
		)
	})

	it('rejects a resource that is not an AI provider resource of the workspace', () => {
		expect(() =>
			validateAiAgentProviders(
				staticProvider({ kind: 'openai', resource: '$res:f/ai/openai', model: 'gpt-5.6-sol' }),
				[ANTHROPIC]
			)
		).toThrow(/not an AI provider resource/)
	})

	it('accepts a served model, a linked agent, and a runtime-resolved provider', () => {
		expect(() =>
			validateAiAgentProviders(
				staticProvider({
					kind: 'anthropic',
					resource: '$res:u/admin/anthropic',
					model: 'claude-sonnet-5'
				}),
				[ANTHROPIC]
			)
		).not.toThrow()
		expect(() =>
			validateAiAgentProviders(
				[{ id: 'agent', value: { type: 'aiagent', agent: 'u/admin/saved' } }],
				[ANTHROPIC]
			)
		).not.toThrow()
		expect(() =>
			validateAiAgentProviders(agentModule({ type: 'javascript', expr: 'flow_input.provider' }), [
				ANTHROPIC
			])
		).not.toThrow()
	})

	it('reports, but does not reject, an unlisted model on a proxied resource', () => {
		const warnings: string[] = []
		expect(() =>
			validateAiAgentProviders(
				staticProvider({
					kind: 'anthropic',
					resource: '$res:u/admin/anthropic',
					model: 'team-sonnet'
				}),
				[{ ...ANTHROPIC, customEndpoint: true }],
				warnings
			)
		).not.toThrow()
		expect(warnings).toHaveLength(1)
		expect(warnings[0]).toMatch(/team-sonnet/)
	})

	it('leaves the model unchecked when the listing failed, so a stale fallback cannot block a write', () => {
		expect(() =>
			validateAiAgentProviders(
				staticProvider({
					kind: 'anthropic',
					resource: '$res:u/admin/anthropic',
					model: 'claude-something-new'
				}),
				[{ ...ANTHROPIC, modelsAreLive: false }]
			)
		).not.toThrow()
	})
})

describe('formatAiAgentProvidersPrompt', () => {
	it('takes the workspace default when one resource makes the choice unambiguous', () => {
		const prompt = formatAiAgentProvidersPrompt({
			options: [ANTHROPIC],
			defaultModel: { kind: 'anthropic', model: 'claude-sonnet-5' }
		})
		expect(prompt).toContain('the workspace default')
		expect(prompt).not.toContain('askUserQuestion')
	})

	it('asks the user when several resources compete, or no default model is set', () => {
		const twoResources = formatAiAgentProvidersPrompt({
			options: [ANTHROPIC, { ...ANTHROPIC, resourcePath: 'u/admin/other', resourceRef: '$res:u/admin/other' }],
			defaultModel: { kind: 'anthropic', model: 'claude-sonnet-5' }
		})
		expect(twoResources).toContain('askUserQuestion')
		expect(formatAiAgentProvidersPrompt({ options: [ANTHROPIC] })).toContain('askUserQuestion')
	})
})
