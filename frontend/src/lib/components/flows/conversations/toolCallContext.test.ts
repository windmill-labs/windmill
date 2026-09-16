import { describe, expect, it } from 'vitest'
import { jobToToolCallDetails } from './toolCallContext.svelte'

describe('jobToToolCallDetails', () => {
	it('reads nothing from the owning agent job an MCP or provider-native tool row names', () => {
		expect(
			jobToToolCallDetails({
				job_kind: 'aiagent',
				script_path: 'f/chat/agent/a',
				args: { user_message: 'hi', provider: { model: 'claude-sonnet-5' } },
				result: 'the agent answer'
			})
		).toEqual({})
	})

	it('reads the call from a nested agent tool, which is an agent job of its own', () => {
		expect(
			jobToToolCallDetails({
				job_kind: 'aiagent',
				script_path: 'f/chat/agent/a/tools/researcher',
				args: { user_message: 'find retention docs' },
				result: 'found them'
			})
		).toEqual({
			toolName: 'researcher',
			parameters: { user_message: 'find retention docs' },
			result: 'found them'
		})
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
