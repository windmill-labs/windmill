import { describe, expect, it } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { jobToToolCallDetails } from './toolCallContext.svelte'

const MODULES = [
	{
		id: 'a',
		value: {
			type: 'aiagent',
			input_transforms: {},
			tools: [
				{
					id: 'search_docs',
					value: {
						type: 'rawscript',
						language: 'bun',
						content: '',
						input_transforms: {
							query: { type: 'ai' },
							api_key: { type: 'javascript', expr: 'flow_input.api_key' },
							region: { type: 'static', value: 'eu' },
							limit: { type: 'static', value: null }
						}
					}
				},
				{
					id: 'researcher',
					value: {
						type: 'aiagent',
						tools: [],
						input_transforms: {
							user_message: { type: 'ai' },
							provider: { type: 'static', value: { kind: 'anthropic' } }
						}
					}
				}
			]
		}
	}
] as unknown as FlowModule[]

describe('jobToToolCallDetails', () => {
	it('reads nothing from the owning agent job an MCP or provider-native tool row names', () => {
		expect(
			jobToToolCallDetails(
				{
					job_kind: 'aiagent',
					script_path: 'f/chat/agent/a',
					args: { user_message: 'hi', provider: { model: 'claude-sonnet-5' } },
					result: 'the agent answer'
				},
				MODULES
			)
		).toEqual({})
	})

	// A job's args also hold what the tool's own transforms resolved, secrets included; the
	// card shows only what the model passed, as it did live.
	it('shows only the arguments the model supplied', () => {
		expect(
			jobToToolCallDetails(
				{
					job_kind: 'preview',
					script_path: 'f/chat/agent/a/tools/search_docs',
					args: { query: 'retention', api_key: 'sk-secret', region: 'eu', limit: 5 },
					result: ['a', 'b']
				},
				MODULES
			)
		).toEqual({
			toolName: 'search_docs',
			parameters: { query: 'retention', limit: 5 },
			result: ['a', 'b']
		})
	})

	it('reads a nested agent tool, which is an agent job of its own', () => {
		expect(
			jobToToolCallDetails(
				{
					job_kind: 'aiagent',
					script_path: 'f/chat/agent/a/tools/researcher',
					args: { user_message: 'find retention docs', provider: { kind: 'anthropic' } },
					result: 'found them'
				},
				MODULES
			)
		).toEqual({
			toolName: 'researcher',
			parameters: { user_message: 'find retention docs' },
			result: 'found them'
		})
	})

	it('shows no arguments for a tool the flow no longer has', () => {
		expect(
			jobToToolCallDetails(
				{
					job_kind: 'preview',
					script_path: 'f/chat/agent/a/tools/removed',
					args: { token: 'sk-secret' },
					result: 'ok'
				},
				MODULES
			).parameters
		).toBeUndefined()
	})
})
