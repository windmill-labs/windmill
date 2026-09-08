import { describe, expect, it } from 'vitest'
import { parseProviderTransform, resolveAgentModelWiring } from './agentChatInputs'
import type { FlowModule } from '$lib/gen'

function agent(expr: string): FlowModule {
	return {
		id: 'a',
		value: { type: 'aiagent', tools: [], input_transforms: { provider: { type: 'javascript', expr } } }
	} as unknown as FlowModule
}

describe('parseProviderTransform', () => {
	it('reads a whole-object reference', () => {
		expect(parseProviderTransform({ type: 'javascript', expr: 'flow_input.model' })).toEqual({
			whole: 'model',
			fields: {},
			fixed: {}
		})
	})

	it('splits a partial expression into wired and fixed fields', () => {
		const wiring = parseProviderTransform({
			type: 'javascript',
			expr: "{ kind: 'anthropic', resource: '$res:u/admin/claude', model: flow_input.model, reasoning_effort: flow_input.thinking }"
		})
		expect(wiring).toEqual({
			fields: { model: 'model', reasoning_effort: 'thinking' },
			fixed: { kind: 'anthropic', resource: '$res:u/admin/claude' }
		})
	})

	// The regression this detector exists for: counting flow_input references would read
	// this as one input carrying the whole provider object, and the composer would write
	// {kind, resource, model} into an input the expression uses as the model name.
	it('does not mistake a single-reference partial expression for a whole-object one', () => {
		const wiring = parseProviderTransform({
			type: 'javascript',
			expr: "{ kind: 'anthropic', resource: '$res:u/admin/claude', model: flow_input.model }"
		})
		expect(wiring?.whole).toBeUndefined()
		expect(wiring?.fields).toEqual({ model: 'model' })
	})

	// The flow editor's JS field commonly holds a parenthesised object, which is how an
	// author writes one without it reading as a block.
	it('accepts a parenthesised object expression', () => {
		const wiring = parseProviderTransform({
			type: 'javascript',
			expr: `({
    "kind": "anthropic",
    "resource": "$res:u/admin/anthropic_windmill_codegen",
    "model": "claude-sonnet-5",
    "reasoning_effort": flow_input.thinking
})`
		})
		expect(wiring).toEqual({
			fields: { reasoning_effort: 'thinking' },
			fixed: {
				kind: 'anthropic',
				resource: '$res:u/admin/anthropic_windmill_codegen',
				model: 'claude-sonnet-5'
			}
		})
	})

	it('treats a static provider as entirely fixed', () => {
		expect(
			parseProviderTransform({
				type: 'static',
				value: { kind: 'openai', resource: '$res:u/admin/oai', model: 'gpt-5.6' }
			})
		).toEqual({
			fields: {},
			fixed: { kind: 'openai', resource: '$res:u/admin/oai', model: 'gpt-5.6' }
		})
	})

	it.each([
		["{ ...base, model: flow_input.model }", 'a spread could supply any field'],
		["{ model: pickModel(flow_input.x) }", 'a call is not classifiable'],
		['flow_input.model + 1', 'not a bare reference'],
		['{ model: ', 'unparseable']
	])('gives up on %s', (expr) => {
		expect(parseProviderTransform({ type: 'javascript', expr } as any)).toBeUndefined()
	})
})

describe('resolveAgentModelWiring', () => {
	const fixedResource = `"kind": "anthropic", "resource": "$res:u/admin/claude"`

	it('drives a field every agent reads from the same input', () => {
		const wiring = resolveAgentModelWiring([
			agent(`({ ${fixedResource}, "model": "claude-sonnet-5", reasoning_effort: flow_input.thinking })`),
			agent(`({ ${fixedResource}, "model": "claude-opus-5", reasoning_effort: flow_input.thinking })`)
		])
		expect(wiring?.fields).toEqual({ reasoning_effort: 'thinking' })
		// The agents run different models, so there is no single one to name.
		expect(wiring?.fixed).toEqual({ kind: 'anthropic', resource: '$res:u/admin/claude' })
	})

	it('drops a field the agents disagree about', () => {
		const wiring = resolveAgentModelWiring([
			agent(`({ ${fixedResource}, reasoning_effort: flow_input.thinking })`),
			agent(`({ ${fixedResource}, reasoning_effort: flow_input.other })`)
		])
		expect(wiring?.fields.reasoning_effort).toBeUndefined()
	})

	// A nested agent is the parent agent's tool, not a step the reader is talking to: the
	// graph walks it as a child module, and counting it here would defeat the chat's own
	// model control (this is the shape of the all-tools example flow).
	it('ignores an agent carried as another agent\'s tool', () => {
		const parent = agent('flow_input.model')
		;(parent.value as any).tools = [
			{
				id: 'summarize',
				value: {
					tool_type: 'flowmodule',
					type: 'aiagent',
					tools: [],
					input_transforms: {
						provider: { type: 'static', value: { kind: 'anthropic', model: 'claude-sonnet-5' } }
					}
				}
			}
		]
		expect(resolveAgentModelWiring([parent])).toEqual({ whole: 'model', fields: {}, fixed: {} })
	})

	it('refuses a flow mixing whole-object and field-by-field wiring', () => {
		expect(
			resolveAgentModelWiring([
				agent('flow_input.provider'),
				agent(`({ ${fixedResource}, model: flow_input.model })`)
			])
		).toBeUndefined()
	})
})
