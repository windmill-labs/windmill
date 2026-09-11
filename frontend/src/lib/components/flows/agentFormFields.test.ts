import { describe, expect, it } from 'vitest'
import { AI_AGENT_SCHEMA } from './flowInfers'
import {
	AGENT_FIELD_BY_KEY,
	AGENT_FIELDS,
	agentFieldIsSet,
	agentStreamingEnabled,
	initialVisibleAgentFields
} from './agentFormFields'

const schemaProperties = AI_AGENT_SCHEMA.properties ?? {}

function set(key: string, transform: unknown): boolean {
	return agentFieldIsSet(AGENT_FIELD_BY_KEY[key], transform)
}

describe('agentFieldIsSet', () => {
	it('reads the seeded placeholder as unset', () => {
		expect(set('temperature', { type: 'static', value: undefined })).toBe(false)
		expect(set('temperature', undefined)).toBe(false)
		// The same placeholder after a round trip through the API.
		expect(set('temperature', { type: 'static', value: null })).toBe(false)
	})

	it('treats a value the runtime cannot tell from absence as unset', () => {
		expect(set('output_type', { type: 'static', value: 'text' })).toBe(false)
		expect(set('max_iterations', { type: 'static', value: 10 })).toBe(false)
		expect(set('memory', { type: 'static', value: { kind: 'off' } })).toBe(false)
		expect(set('user_attachments', { type: 'static', value: [] })).toBe(false)
	})

	it('reads streaming as unset while it is on, which is what an absent key does', () => {
		expect(schemaProperties.streaming?.default).toBe(true)
		expect(set('streaming', { type: 'static', value: true })).toBe(false)
		expect(set('streaming', { type: 'static', value: false })).toBe(true)
	})

	it('reads anything the user authored as set', () => {
		expect(set('temperature', { type: 'static', value: 0 })).toBe(true)
		expect(set('output_type', { type: 'static', value: 'image' })).toBe(true)
		expect(set('memory', { type: 'static', value: { kind: 'auto', context_length: 5 } })).toBe(true)
		expect(set('max_iterations', { type: 'javascript', expr: 'flow_input.loops' })).toBe(true)
		expect(set('max_iterations', { type: 'javascript', expr: '' })).toBe(false)
	})
})

describe('initialVisibleAgentFields', () => {
	it('shows only the core fields for a step that materialized every default', () => {
		// What the pre-redesign form wrote on first render of a blank agent step.
		const legacy = {
			provider: { type: 'static', value: { kind: 'openai', resource: '$res:u/admin/openai' } },
			output_type: { type: 'static', value: 'text' },
			user_message: { type: 'static', value: undefined },
			system_prompt: { type: 'static', value: undefined },
			streaming: { type: 'static', value: true },
			memory: { type: 'static', value: { kind: 'off' } },
			output_schema: { type: 'static', value: null },
			user_attachments: { type: 'static', value: [] },
			max_completion_tokens: { type: 'static', value: null },
			temperature: { type: 'static', value: null },
			max_iterations: { type: 'static', value: 10 }
		}
		expect([...initialVisibleAgentFields(legacy, schemaProperties)].sort()).toEqual([
			'provider',
			'system_prompt',
			'tools',
			'user_message'
		])
	})

	it('collapses a linked step to its flow-local inputs', () => {
		const linkedSchema = {
			user_message: schemaProperties.user_message,
			user_attachments: schemaProperties.user_attachments
		}
		expect([...initialVisibleAgentFields({}, linkedSchema)]).toEqual(['user_message'])
	})

	it('covers every schema key, so no field can only be reached through the raw doc', () => {
		const registered = new Set(AGENT_FIELDS.map((f) => f.key))
		expect(Object.keys(schemaProperties).filter((k) => !registered.has(k))).toEqual([])
	})
})

// Three chat surfaces decide whether to consume a stream from this, and the worker decides whether
// to send one from `streaming.unwrap_or(true)`. They agree only while absent means on here.
describe('agentStreamingEnabled', () => {
	const step = (input_transforms: Record<string, any>, rest: Record<string, any> = {}) => ({
		type: 'aiagent',
		input_transforms,
		...rest
	})

	it('reads an unwritten field as streaming', () => {
		expect(agentStreamingEnabled(step({}))).toBe(true)
		// What the API returns for the `{"type":"static"}` placeholder the schema backfill seeds.
		expect(agentStreamingEnabled(step({ streaming: { type: 'static', value: null } }))).toBe(true)
		expect(agentStreamingEnabled(step({ streaming: { type: 'static', value: true } }))).toBe(true)
	})

	it('only an explicit false holds the answer back', () => {
		expect(agentStreamingEnabled(step({ streaming: { type: 'static', value: false } }))).toBe(false)
	})

	it('reads off what the step cannot answer for', () => {
		// An image answer never streams, whatever `streaming` says.
		expect(
			agentStreamingEnabled(
				step({
					streaming: { type: 'static', value: true },
					output_type: { type: 'static', value: 'image' }
				})
			)
		).toBe(false)
		// A linked step carries no brain: the agent's own `streaming: false` is invisible here.
		expect(agentStreamingEnabled(step({}, { agent: 'u/admin/a' }))).toBe(false)
		// An expression has no value until the run it would decide is already under way, on either
		// of the two fields the answer depends on.
		expect(
			agentStreamingEnabled(step({ streaming: { type: 'javascript', expr: 'flow_input.s' } }))
		).toBe(false)
		expect(
			agentStreamingEnabled(step({ output_type: { type: 'javascript', expr: 'flow_input.o' } }))
		).toBe(false)
	})
})
