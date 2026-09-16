import { describe, expect, it } from 'vitest'
import {
	agentModelGap,
	agentModelWiringInputs,
	composerOwnedInputs,
	parseProviderTransform,
	resolveAgentModelWiring,
	showsModelButton,
	withoutRejectedEffort
} from './agentChatInputs'
import type { FlowModule } from '$lib/gen'

function agent(expr: string): FlowModule {
	return {
		id: 'a',
		value: {
			type: 'aiagent',
			tools: [],
			input_transforms: { provider: { type: 'javascript', expr } }
		}
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
		['{ ...base, model: flow_input.model }', 'a spread could supply any field'],
		['{ model: pickModel(flow_input.x) }', 'a call is not classifiable'],
		['flow_input.model + 1', 'not a bare reference'],
		['{ model: ', 'unparseable']
	])('gives up on %s', (expr) => {
		expect(parseProviderTransform({ type: 'javascript', expr } as any)).toBeUndefined()
	})
})

describe('agentModelWiringInputs', () => {
	// The button writes `kind` only alongside a resource, since a provider is picked as a
	// pair. Hiding a kind input it cannot write would leave the run without one.
	it('keeps a kind input the model button cannot write', () => {
		const wiring = resolveAgentModelWiring([
			agent(`({ kind: flow_input.k, "resource": "$res:u/admin/claude", model: flow_input.m })`)
		])
		expect(agentModelWiringInputs(wiring)).toEqual(['m'])
	})

	it('hides a kind input it writes with the resource', () => {
		const wiring = resolveAgentModelWiring([
			agent(`({ kind: flow_input.k, resource: flow_input.r, model: flow_input.m })`)
		])
		expect(agentModelWiringInputs(wiring)?.sort()).toEqual(['k', 'm', 'r'])
	})

	// A thinking control is usable whatever the registry knows about the model — a ladder, a
	// typed token, or why there is neither — so a wired effort is the button's there.
	it('claims a wired reasoning_effort whatever the provider', () => {
		const custom = resolveAgentModelWiring([
			agent(
				`({ "kind": "customai", "resource": "$res:u/admin/custom", model: flow_input.m, reasoning_effort: flow_input.thinking })`
			)
		])
		expect(agentModelWiringInputs(custom)?.sort()).toEqual(['m', 'thinking'])

		const known = resolveAgentModelWiring([
			agent(
				`({ "kind": "openai", "resource": "$res:u/admin/oai", "model": "gpt-4o", reasoning_effort: flow_input.thinking })`
			)
		])
		expect(agentModelWiringInputs(known)).toEqual(['thinking'])
	})

	// Agents on different fixed models leave the button no model to place the effort on, so
	// it would say "Pick a model first" with nothing to pick. The modal keeps the input.
	it('leaves a shared effort to the modal when the agents fix different models', () => {
		const resource = `"kind": "anthropic", "resource": "$res:u/admin/claude"`
		const wiring = resolveAgentModelWiring([
			agent(`({ ${resource}, "model": "claude-sonnet-5", reasoning_effort: flow_input.thinking })`),
			agent(`({ ${resource}, "model": "claude-opus-5", reasoning_effort: flow_input.thinking })`)
		])
		expect(wiring?.fields.reasoning_effort).toBe('thinking')
		expect(agentModelWiringInputs(wiring)).toEqual([])
		expect(showsModelButton(wiring)).toBe(false)
	})

	// The same one level up: agents on different providers leave the model menu nothing to
	// list and no provider to gate a typed id, so the shared model input stays in the modal.
	it('leaves a shared model to the modal when the agents fix different providers', () => {
		const wiring = resolveAgentModelWiring([
			agent(`({ "kind": "openai", "resource": "$res:u/admin/oai", model: flow_input.model })`),
			agent(
				`({ "kind": "azure_openai", "resource": "$res:u/admin/azure", model: flow_input.model })`
			)
		])
		expect(wiring?.fields.model).toBe('model')
		expect(agentModelWiringInputs(wiring)).toEqual([])
		expect(showsModelButton(wiring)).toBe(false)
	})
})

// The modal is whatever this does not return, so the two cannot disagree about an input.
describe('composerOwnedInputs', () => {
	const wiring = () =>
		resolveAgentModelWiring([
			agent(`({ "kind": "openai", "resource": "$res:u/admin/oai", model: flow_input.m })`)
		])

	it('claims the model wiring and the attachments target together', () => {
		expect(composerOwnedInputs(wiring(), { name: 'files' })?.sort()).toEqual(['files', 'm'])
	})

	it('claims the attachments input whatever the workspace can do with it', () => {
		// No object storage is a state the paperclip shows, not a handover: the modal could
		// not upload either, and the run would fail fetching a key typed there.
		expect(composerOwnedInputs(undefined, { name: 'files' })).toEqual(['files'])
	})

	it('leaves an input no control fits to the modal', () => {
		expect(composerOwnedInputs(wiring(), undefined)).toEqual(['m'])
	})
})

describe('resolveAgentModelWiring', () => {
	const fixedResource = `"kind": "anthropic", "resource": "$res:u/admin/claude"`

	it('drives a field every agent reads from the same input', () => {
		const wiring = resolveAgentModelWiring([
			agent(
				`({ ${fixedResource}, "model": "claude-sonnet-5", reasoning_effort: flow_input.thinking })`
			),
			agent(
				`({ ${fixedResource}, "model": "claude-opus-5", reasoning_effort: flow_input.thinking })`
			)
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
	it("ignores an agent carried as another agent's tool", () => {
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
		expect(resolveAgentModelWiring([parent])).toEqual({
			whole: 'model',
			fields: {},
			fixed: {},
			someAgentCannotRun: false
		})
	})

	it('finds an agent inside a loop or a branch', () => {
		const inner = agent(`({ ${fixedResource}, model: flow_input.model })`)
		const loop = {
			id: 'loop',
			value: { type: 'forloopflow', modules: [inner] }
		} as unknown as FlowModule
		const branch = {
			id: 'branch',
			value: { type: 'branchone', default: [], branches: [{ modules: [inner] }] }
		} as unknown as FlowModule
		expect(resolveAgentModelWiring([loop])?.fields.model).toBe('model')
		expect(resolveAgentModelWiring([branch])?.fields.model).toBe('model')
	})

	// The control writes one flow input; an agent that fixes the field instead never reads
	// it, so offering the control would move one agent and leave the other where it was.
	it('does not offer a field one agent wires and another fixes', () => {
		const wiring = resolveAgentModelWiring([
			agent(`({ ${fixedResource}, model: flow_input.model })`),
			agent(`({ ${fixedResource}, "model": "claude-opus-5" })`)
		])
		expect(wiring?.fields.model).toBeUndefined()
		expect(wiring?.fixed.model).toBeUndefined()
	})

	// Disagreeing about the model is not the same as having no model: the flow runs, on a
	// different one per agent, and the composer has nothing to fix.
	it('says nothing about a model the agents merely disagree about', () => {
		const wiring = resolveAgentModelWiring([
			agent(`({ ${fixedResource}, "model": "claude-sonnet-5" })`),
			agent(`({ ${fixedResource}, "model": "claude-opus-5" })`)
		])
		expect(agentModelGap(wiring)).toBeUndefined()
	})

	it('still reports an agent with nothing to call', () => {
		expect(
			agentModelGap(resolveAgentModelWiring([agent(`({ "kind": "openai", "model": "" })`)]))
		).toBe('Pick a provider and model on the AI agent step to use this chat.')
	})

	// An expression the parser cannot account for could supply anything, so the agents it
	// belongs to cannot be spoken for either.
	it("offers nothing when one agent's provider cannot be read", () => {
		expect(
			resolveAgentModelWiring([
				agent(`({ ${fixedResource}, model: flow_input.model })`),
				agent(`({ ...base, model: flow_input.model })`)
			])
		).toBeUndefined()
	})

	// Disagreement is not the same as absence, but an agent with an empty model still
	// cannot run, however well its neighbour is configured.
	it('keeps warning when one agent has no model and another does', () => {
		const wiring = resolveAgentModelWiring([
			agent(`({ ${fixedResource}, "model": "claude-sonnet-5" })`),
			agent(`({ ${fixedResource}, "model": "" })`)
		])
		expect(agentModelGap(wiring)).toBe(
			'Pick a provider and model on the AI agent step to use this chat.'
		)
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

describe('withoutRejectedEffort', () => {
	const wiring = (fields: Record<string, string>, fixed: Record<string, any> = {}) =>
		({ fields, fixed }) as any

	// The live 400 this guards: Anthropic turns any effort into adaptive thinking, which
	// Haiku rejects outright ("adaptive thinking is not supported on this model").
	it('drops an effort the chosen model rejects', () => {
		const values = { model: 'claude-haiku-4-5-20251001', reasoning_effort: 'high' }
		expect(
			withoutRejectedEffort(
				wiring({ model: 'model', reasoning_effort: 'reasoning_effort' }, { kind: 'anthropic' }),
				values
			)
		).toEqual({ model: 'claude-haiku-4-5-20251001', reasoning_effort: '' })
	})

	it('keeps an effort the model takes', () => {
		const values = { model: 'claude-sonnet-5', reasoning_effort: 'high' }
		expect(
			withoutRejectedEffort(
				wiring({ model: 'model', reasoning_effort: 'reasoning_effort' }, { kind: 'anthropic' }),
				values
			)
		).toBe(values)
	})

	// Clearing on a guess would override the author's own default.
	it('leaves the value alone for a family the registry cannot speak for', () => {
		const values = { model: 'some-model', reasoning_effort: 'high' }
		expect(
			withoutRejectedEffort(
				wiring({ model: 'model', reasoning_effort: 'reasoning_effort' }, { kind: 'customai' }),
				values
			)
		).toBe(values)
	})

	// A flow that wires `provider` as one object keeps the effort inside it, so reading
	// `fields.reasoning_effort` finds nothing and the 400 would go out unchecked.
	it('clears the effort inside a whole-object provider input', () => {
		const wiring = resolveAgentModelWiring([agent('flow_input.provider')])
		const values = {
			provider: {
				kind: 'anthropic',
				model: 'claude-haiku-4-5-20251001',
				reasoning_effort: 'high'
			}
		}
		expect(withoutRejectedEffort(wiring, values)).toEqual({
			provider: {
				kind: 'anthropic',
				model: 'claude-haiku-4-5-20251001',
				reasoning_effort: ''
			}
		})
	})

	it('leaves a whole-object provider alone when the model takes the effort', () => {
		const wiring = resolveAgentModelWiring([agent('flow_input.provider')])
		const values = {
			provider: { kind: 'anthropic', model: 'claude-sonnet-5', reasoning_effort: 'high' }
		}
		expect(withoutRejectedEffort(wiring, values)).toBe(values)
	})

	// A model that reasons can still refuse a particular token, and the provider answers with a
	// 400 on every turn.
	it('drops a level or off token a reasoning model does not take', () => {
		const openai = wiring({ model: 'model', reasoning_effort: 'effort' }, { kind: 'openai' })
		expect(withoutRejectedEffort(openai, { model: 'gpt-5', effort: 'none' }).effort).toBe('')
		expect(withoutRejectedEffort(openai, { model: 'gpt-5.1', effort: 'xhigh' }).effort).toBe('')
		const kept = { model: 'gpt-5.1', effort: 'none' }
		expect(withoutRejectedEffort(openai, kept)).toBe(kept)
	})
})

/**
 * The shape agent chat is usually built in: one agent answers the reader, others do work of
 * their own in branches. A sub-agent never sees the message, so what it runs on is not a
 * setting this conversation has.
 */
describe('agents that do not read the message', () => {
	const answerer = (provider: string) =>
		({
			id: 'answerer',
			value: {
				type: 'aiagent',
				tools: [],
				input_transforms: {
					user_message: { type: 'javascript', expr: 'flow_input.user_message' },
					provider: { type: 'javascript', expr: provider }
				}
			}
		}) as unknown as FlowModule
	const subAgent = (provider: string, message = "'critique: ' + results.answerer") =>
		({
			id: 'critic',
			value: {
				type: 'aiagent',
				tools: [],
				input_transforms: {
					user_message: { type: 'javascript', expr: message },
					provider: { type: 'javascript', expr: provider }
				}
			}
		}) as unknown as FlowModule

	const wired = `({ kind: 'anthropic', resource: '$res:u/admin/c', model: flow_input.model })`
	const fixed = `({ kind: 'anthropic', resource: '$res:u/admin/c', model: 'claude-sonnet-5' })`

	it('keeps the model control when only a sub-agent fixes its own model', () => {
		const wiring = resolveAgentModelWiring([answerer(wired), subAgent(fixed)])
		expect(wiring?.fields.model).toBe('model')
	})

	// Two agents both answering the reader still have to agree: either might be the one
	// that replies, so a control moving one of them would be a lie about the other.
	it('still needs agreement among the agents that do read the message', () => {
		const wiring = resolveAgentModelWiring([
			answerer(wired),
			subAgent(fixed, 'flow_input.user_message')
		])
		expect(wiring?.fields.model).toBeUndefined()
	})

	// Nothing to scope to means the flow is shaped in some way this cannot read, so every
	// agent counts again rather than none.
	it('falls back to every agent when none reads the message', () => {
		const wiring = resolveAgentModelWiring([subAgent(wired), subAgent(fixed)])
		expect(wiring?.fields.model).toBeUndefined()
	})

	// The editor writes the dot form, but an author may hand-edit either. A shape this does
	// not recognise drops that agent out of the unanimity check it should be part of.
	it.each([
		["flow_input['user_message']", 'bracket access'],
		['flow_input?.user_message', 'optional chaining'],
		["'Answer politely: ' + flow_input.user_message", 'embedded in a prompt'],
		['flow_input.user_message + flow_input.tone', 'read alongside another input'],
		['flow_input.user_message // the message', 'a trailing comment'],
		['flow_input.user_message\n// why', 'a comment on its own last line']
	])('treats %s as reading the message', (message) => {
		const wiring = resolveAgentModelWiring([answerer(wired), subAgent(fixed, message as string)])
		expect(wiring?.fields.model).toBeUndefined()
	})

	it.each([
		['// see flow_input.user_message', 'a mention in a comment'],
		["'flow_input.user_message'", 'a mention in a string'],
		['flow_input.user_message_extra', 'a different input with the same prefix']
	])('does not treat %s as reading the message', (message) => {
		const wiring = resolveAgentModelWiring([answerer(wired), subAgent(fixed, message as string)])
		expect(wiring?.fields.model).toBe('model')
	})
})

// An author annotating their own provider must not lose the model control for it: a
// comment beside the expression is not another expression.
describe('parseProviderTransform with comments', () => {
	it.each([
		['flow_input.provider // the one to use', 'a trailing line comment'],
		['flow_input.provider /* the one to use */', 'a trailing block comment'],
		['/* pick one */ flow_input.provider', 'a leading comment']
	])('reads a whole-object reference despite %s', (expr) => {
		expect(parseProviderTransform({ type: 'javascript', expr })?.whole).toBe('provider')
	})

	it('reads an object literal with a comment inside it', () => {
		const wiring = parseProviderTransform({
			type: 'javascript',
			expr: `({ kind: 'anthropic', /* fixed */ resource: '$res:u/admin/c', model: flow_input.model })`
		})
		expect(wiring?.fields.model).toBe('model')
	})

	// The check exists to reject an expression with something else beside it; a second
	// expression is still something else.
	it('still refuses a second expression beside it', () => {
		expect(
			parseProviderTransform({ type: 'javascript', expr: 'flow_input.provider, 1' })
		).toBeUndefined()
	})
})
