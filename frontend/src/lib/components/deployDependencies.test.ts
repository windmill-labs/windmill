import { describe, expect, it } from 'vitest'
import { agentResourceDependencies, aiAgentModuleDependencies } from './deployDependencies'

const res = (path: string) => ({ type: 'static', value: `$res:${path}` })

describe('aiAgentModuleDependencies', () => {
	it('queues the linked agent resource', () => {
		expect(aiAgentModuleDependencies({ agent: '$res:f/team/my_agent' })).toEqual([
			{ kind: 'resource', path: 'f/team/my_agent' }
		])
	})

	// A tool_inputs override replaces the resource tool's default at runtime, so the target
	// workspace needs the overriding value, not the saved default.
	it('follows tool_inputs overrides, which shadow the saved defaults', () => {
		expect(
			aiAgentModuleDependencies({
				agent: 'f/team/my_agent',
				tool_inputs: {
					fetch: { db: res('f/prod/db'), token: { type: 'static', value: '$var:f/prod/tok' } },
					other: { expr: { type: 'javascript', expr: 'flow_input.x' } }
				}
			})
		).toEqual([
			{ kind: 'resource', path: 'f/team/my_agent' },
			{ kind: 'resource', path: 'f/prod/db' },
			{ kind: 'variable', path: 'f/prod/tok' }
		])
	})
})

describe('agentResourceDependencies', () => {
	it('collects a saved agent tools bare paths and transform refs', () => {
		expect(
			agentResourceDependencies({
				tools: [
					{
						id: 'a',
						value: {
							type: 'script',
							path: 'f/lib/tool',
							input_transforms: { db: res('f/prod/db') }
						}
					},
					{ id: 'b', value: { type: 'script', path: 'hub/1/thing' } },
					{ id: 'c', value: { tool_type: 'mcp', resource_path: '$res:f/mcp/server' } }
				]
			})
		).toEqual([
			{ kind: 'resource', path: 'f/prod/db' },
			{ kind: 'script', path: 'f/lib/tool' },
			{ kind: 'resource', path: 'f/mcp/server' }
		])
	})

	it('recurses into an inline nested agent but links a nested one', () => {
		expect(
			agentResourceDependencies({
				tools: [
					{
						id: 'inline',
						value: {
							type: 'aiagent',
							tools: [{ id: 'x', value: { type: 'flow', path: 'f/lib/sub' } }]
						}
					},
					{ id: 'linked', value: { type: 'aiagent', agent: 'f/team/other' } }
				]
			})
		).toEqual([
			{ kind: 'flow', path: 'f/lib/sub' },
			{ kind: 'resource', path: 'f/team/other' }
		])
	})
})

describe('inline agent dependencies', () => {
	// A provider is an object-valued static transform holding its credential under `resource`, so a
	// top-level string check misses it.
	it('finds a provider credential nested in an object-valued transform', () => {
		expect(
			aiAgentModuleDependencies({
				input_transforms: {
					provider: { type: 'static', value: { kind: 'openai', model: 'gpt-4o' } }
				},
				tools: []
			})
		).toEqual([])
		expect(
			aiAgentModuleDependencies({
				input_transforms: {
					provider: {
						type: 'static',
						value: { kind: 'openai', model: 'gpt-4o', resource: '$res:f/ai/openai' }
					}
				},
				tools: []
			})
		).toEqual([{ kind: 'resource', path: 'f/ai/openai' }])
	})

	// The worker resolves $jsonvar: alongside $var:/$res:, so a secret referenced only that way has
	// to deploy with the agent or the linked step fails in the target workspace.
	it('collects $jsonvar: references', () => {
		expect(
			aiAgentModuleDependencies({
				input_transforms: { key: { type: 'static', value: '$jsonvar:f/prod/cfg' } },
				tools: []
			})
		).toEqual([{ kind: 'variable', path: 'f/prod/cfg' }])
	})

	it('walks an inline agent own tools, which the flow module walk does not reach', () => {
		expect(
			aiAgentModuleDependencies({
				tools: [{ id: 'm', value: { tool_type: 'mcp', resource_path: 'f/mcp/server' } }]
			})
		).toEqual([{ kind: 'resource', path: 'f/mcp/server' }])
	})
})
