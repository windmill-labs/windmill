import { describe, expect, it } from 'vitest'
import { buildAgentTrace } from './agentTrace'
import { parseAgentErrorMessages } from './aiAgentResult'
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

describe('buildAgentTrace', () => {
	it('joins a tool call to the arguments on the message that requested it', () => {
		expect(buildAgentTrace(messages)).toEqual([
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
		const entries = buildAgentTrace([
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

	// The worker splits a search the same way: a `tool` message tagged web_search
	// carrying a constant sentence, then the assistant turn that carries the
	// citations. The search row therefore has nothing of its own to show.
	it('records the search and puts its citations on the turn that follows', () => {
		const entries = buildAgentTrace([
			{
				role: 'tool',
				content: 'Used websearch tool successfully',
				agent_action: { type: 'web_search' }
			},
			{
				role: 'assistant',
				content: 'Postgres 17 changed the default.',
				annotations: [{ url: 'https://postgresql.org/docs', title: 'Release notes' }],
				agent_action: { type: 'message' }
			}
		])
		expect(entries).toEqual([
			{ kind: 'search' },
			{
				kind: 'assistant',
				content: 'Postgres 17 changed the default.',
				sources: [{ url: 'https://postgresql.org/docs', title: 'Release notes' }]
			}
		])
	})

	// The prompt and the question are the step's inputs, shown as inputs. A replayed
	// turn comes back from memory without its tag, and crediting this run with an
	// answer a previous one gave would be a lie about what happened.
	it('traces only what this run did', () => {
		expect(
			buildAgentTrace([
				{ role: 'system', content: 'You are an SRE assistant.' },
				{ role: 'user', content: 'Which region is broken?' },
				{ role: 'assistant', content: 'Answered in an earlier turn, replayed from memory.' },
				{ role: 'assistant', tool_calls: [{ id: 'c1', function: { name: 'x', arguments: '{}' } }] },
				{ role: 'assistant', content: '', agent_action: { type: 'message' } }
			])
		).toEqual([])
	})
})

// A run stopped by max_iterations serializes its partial messages itself rather
// than reusing the success envelope's writer. `agent_action` is `skip_serializing`
// on `OpenAIMessage`, so if that path ever stops wrapping them the tags vanish and
// this trace silently empties — which is the one run worth reading.
describe('the max-iterations path', () => {
	it('traces the partial messages the error carries', () => {
		const partial = parseAgentErrorMessages({
			error: {
				name: 'ExecutionErr',
				message: 'AI agent reached max iterations (10)',
				step_id: 'd',
				result: { messages }
			}
		})
		expect(partial).toBeDefined()
		expect(buildAgentTrace(partial!)).toEqual([
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
})
