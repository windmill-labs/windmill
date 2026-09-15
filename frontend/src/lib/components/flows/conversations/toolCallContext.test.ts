import { describe, expect, it } from 'vitest'
import { jobToToolCallDetails } from './toolCallContext.svelte'

/**
 * Every conversation row names a job so retention can collect it, which means a tool that
 * ran inside the agent — MCP, or provider-native web search — points at the agent's own
 * job. Reading that job for the call would show the agent's configuration as the tool's
 * arguments and the agent's answer as its result.
 */
describe('jobToToolCallDetails', () => {
	it('reads nothing from the agent job a jobless tool points at', () => {
		expect(
			jobToToolCallDetails({
				job_kind: 'aiagent',
				script_path: 'f/chat/agent',
				args: { user_message: 'hi', provider: { model: 'claude-sonnet-5' } },
				result: 'the agent answer'
			})
		).toEqual({})
	})

	it('reads the call from a tool that has a job of its own', () => {
		expect(
			jobToToolCallDetails({
				job_kind: 'script',
				script_path: 'f/tools/search_docs',
				args: { query: 'retention' },
				result: ['a', 'b']
			})
		).toEqual({ toolName: 'search_docs', parameters: { query: 'retention' }, result: ['a', 'b'] })
	})

	it('treats an argument-less call as having none rather than an empty object', () => {
		expect(
			jobToToolCallDetails({ job_kind: 'script', script_path: 'f/t/now', args: {} }).parameters
		).toBeUndefined()
	})
})
