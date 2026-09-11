import { describe, expect, it } from 'vitest'
import { buildTranscript } from './agentTranscript'
import type { AgentMessage } from './aiAgentResult'

// The worker splits one tool call across two messages: the assistant message
// carries the arguments and no action tag, the `tool` message answering it
// carries the result and the `tool_call` tag naming the job. Joining them on
// `tool_call_id` is the whole contract, and reading it off the wrong message
// yields a row with no parameters.
const messages: AgentMessage[] = [
	{ role: 'system', content: 'You are an SRE assistant.' },
	{ role: 'user', content: 'Which region is broken?' },
	{
		role: 'assistant',
		tool_calls: [
			{
				id: 'call_1',
				type: 'function',
				function: { name: 'query_metrics', arguments: '{"w":"30m"}' }
			}
		]
	},
	{
		role: 'tool',
		tool_call_id: 'call_1',
		content: '{"eu-central-1":0.184}',
		agent_action: {
			type: 'tool_call',
			job_id: '0199-job',
			module_id: 'b',
			function_name: 'query_metrics'
		}
	},
	{ role: 'assistant', content: 'eu-central-1 is down.', agent_action: { type: 'message' } }
]

describe('buildTranscript', () => {
	it('joins a tool call to the arguments on the message that requested it', () => {
		expect(buildTranscript(messages)).toEqual([
			{ kind: 'system', content: 'You are an SRE assistant.' },
			{ kind: 'user', content: 'Which region is broken?' },
			{
				kind: 'tool',
				name: 'query_metrics',
				args: '{"w":"30m"}',
				result: '{"eu-central-1":0.184}',
				jobId: '0199-job'
			},
			{ kind: 'assistant', content: 'eu-central-1 is down.', sources: undefined }
		])
	})

	it('keeps an MCP call, whose arguments live on the action itself', () => {
		const entries = buildTranscript([
			{
				role: 'tool',
				content: 'sunny',
				agent_action: {
					type: 'mcp_tool_call',
					call_id: 'c1',
					function_name: 'get_weather',
					resource_path: 'f/mcp/weather',
					arguments: { city: 'Paris' }
				}
			}
		])
		expect(entries).toEqual([
			{
				kind: 'tool',
				name: 'get_weather',
				args: '{\n  "city": "Paris"\n}',
				result: 'sunny',
				resourcePath: 'f/mcp/weather'
			}
		])
	})

	it('carries web search citations onto the entry', () => {
		const entries = buildTranscript([
			{
				role: 'assistant',
				content: 'Postgres 17 changed the default.',
				annotations: [{ url: 'https://postgresql.org/docs', title: 'Release notes' }],
				agent_action: { type: 'web_search' }
			}
		])
		expect(entries).toEqual([
			{
				kind: 'search',
				content: 'Postgres 17 changed the default.',
				sources: [{ url: 'https://postgresql.org/docs', title: 'Release notes' }]
			}
		])
	})

	// Memory replays messages back without their tags, and an assistant message
	// that only asked for a tool has no text of its own.
	it('drops messages with nothing to show', () => {
		expect(
			buildTranscript([
				{ role: 'assistant', tool_calls: [{ id: 'c1', function: { name: 'x', arguments: '{}' } }] },
				{ role: 'assistant', content: '' }
			])
		).toEqual([])
	})
})
