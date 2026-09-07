import type { FlowModule, InputTransform } from '$lib/gen'
import { getAllModules } from '../flowExplorer'
import { parseExpressionAt } from 'acorn'

/**
 * AI agent inputs the chat composer can drive, in footer display order.
 *
 * The composer never edits the flow: an agent field is reachable only when the author
 * wired a flow input to it, so what the chip writes is a run input like any other. That
 * also keeps it working on the deployed chat, where the reader has no write access, and
 * on a step linked to an `ai_agent` resource, where every field but `user_message` /
 * `user_attachments` comes from the resource and is not overridable at all.
 *
 * Only what the person chatting legitimately owns turn to turn is here: the files they
 * attach, and the model they are talking to. `system_prompt`, `temperature` and
 * `max_completion_tokens` shape how the agent behaves for everyone who runs the flow —
 * surfacing them per conversation invites tuning the flow from the chat instead of
 * fixing it in the editor. They stay flow settings, reachable through Configure inputs
 * when the author deliberately exposes them. `max_iterations` is absent for the same
 * reason, and because it caps the tool-use loop rather than a single generation.
 */
export const AGENT_CHAT_INPUT_KEYS = ['user_attachments'] as const

export type AgentChatInputKey = (typeof AGENT_CHAT_INPUT_KEYS)[number]

/** `user_attachments` is per-turn by definition; the rest are conversation settings. */
export const PER_TURN_AGENT_CHAT_INPUT_KEY: AgentChatInputKey = 'user_attachments'

export type AgentChatInput = {
	/** Flow input property feeding the agent field. */
	name: string
	key: AgentChatInputKey
	/** The flow input's own schema entry — the chip renders it with the same editor the modal would. */
	property: Record<string, any>
	required: boolean
}

const FLOW_INPUT_REF = /flow_input\??\.([A-Za-z_$][\w$]*)/g

/**
 * The flow input a transform is fed by, when exactly one feeds it.
 *
 * The expression need not be a bare pass-through — a step commonly reshapes what it
 * reads, e.g. `(flow_input.files || []).map(f => ({ bucket: f.storage, key: f.s3 }))`.
 * Writing that input is still right, because the expression consumes it. Two or more
 * inputs are ambiguous: the composer would have no way to say which one it is editing.
 */
export function flowInputRef(transform: InputTransform | undefined): string | undefined {
	if (transform?.type !== 'javascript') return undefined
	const names = new Set(
		[...transform.expr.matchAll(FLOW_INPUT_REF)].map((match) => match[1])
	)
	return names.size === 1 ? [...names][0] : undefined
}

/** A provider value as the agent stores it. */
export type AgentModel = { kind?: string; model?: string; reasoning_effort?: string }

/** The provider fields the composer can read or drive. */
const PROVIDER_FIELDS = ['kind', 'resource', 'model', 'reasoning_effort'] as const
export type ProviderField = (typeof PROVIDER_FIELDS)[number]

/**
 * How an AI agent step's `provider` is supplied, field by field.
 *
 * The agent takes one `provider` object, so an author who wants the chat to choose only
 * the model writes the rest as literals around it:
 *
 *   { kind: 'anthropic', resource: '$res:u/admin/claude', model: flow_input.model }
 *
 * `fields` names the flow input behind each field the author exposed, `fixed` holds the
 * literals, and `whole` covers the plain `flow_input.x` case where one input carries the
 * entire object. Reading them apart is what lets the composer offer exactly the knobs the
 * flow exposed — and stops a partial expression from being mistaken for a whole-object
 * one, which would write a provider object into an input the flow reads as a model name.
 */
export type AgentModelWiring = {
	whole?: string
	fields: Partial<Record<ProviderField, string>>
	fixed: Partial<Record<ProviderField, any>>
}

/** The flow input behind `flow_input.x`, `flow_input?.x` or `flow_input['x']`. */
function flowInputName(node: any): string | undefined {
	const member = node?.type === 'ChainExpression' ? node.expression : node
	if (member?.type !== 'MemberExpression') return undefined
	if (member.object?.type !== 'Identifier' || member.object.name !== 'flow_input') return undefined
	if (!member.computed && member.property?.type === 'Identifier') return member.property.name
	if (member.computed && member.property?.type === 'Literal') {
		return typeof member.property.value === 'string' ? member.property.value : undefined
	}
	return undefined
}

/** A property's name, for the plain `key:` and `'key':` forms only. */
function propertyKey(property: any): string | undefined {
	if (property?.type !== 'Property' || property.computed) return undefined
	if (property.key?.type === 'Identifier') return property.key.name
	if (property.key?.type === 'Literal' && typeof property.key.value === 'string') {
		return property.key.value
	}
	return undefined
}

/**
 * Read a `provider` input transform. Anything this cannot account for in full returns
 * undefined rather than a guess: the composer then leaves the field alone instead of
 * writing into an expression it does not understand.
 */
export function parseProviderTransform(
	transform: InputTransform | undefined
): AgentModelWiring | undefined {
	if (transform?.type === 'static') {
		const value = transform.value
		if (!value || typeof value !== 'object') return undefined
		const fixed: AgentModelWiring['fixed'] = {}
		for (const field of PROVIDER_FIELDS) {
			if (value[field] !== undefined) fixed[field] = value[field]
		}
		return { fields: {}, fixed }
	}
	if (transform?.type !== 'javascript') return undefined

	// Parenthesised so a leading `{` reads as an object literal rather than a block. The
	// author's own text may already be wrapped that way, so any balanced surround is fine —
	// what the span check rejects is an expression with something else beside it.
	const source = `(${transform.expr})`
	let node: any
	try {
		node = parseExpressionAt(source, 0, { ecmaVersion: 'latest' })
	} catch {
		return undefined
	}
	const before = source.slice(0, node.start)
	const after = source.slice(node.end)
	if (!/^[\s(]*$/.test(before) || !/^[\s)]*$/.test(after)) return undefined
	if ((before.match(/\(/g)?.length ?? 0) !== (after.match(/\)/g)?.length ?? 0)) return undefined

	const whole = flowInputName(node)
	if (whole) return { whole, fields: {}, fixed: {} }
	if (node.type !== 'ObjectExpression') return undefined

	const fields: AgentModelWiring['fields'] = {}
	const fixed: AgentModelWiring['fixed'] = {}
	for (const property of node.properties) {
		const key = propertyKey(property)
		// A spread or a computed key could supply any field, so nothing here is knowable.
		if (!key) return undefined
		if (!(PROVIDER_FIELDS as readonly string[]).includes(key)) continue
		const name = flowInputName(property.value)
		if (name) {
			fields[key as ProviderField] = name
		} else if (property.value?.type === 'Literal') {
			fixed[key as ProviderField] = property.value.value
		} else {
			return undefined
		}
	}
	return { fields, fixed }
}

/**
 * The provider wiring the chat can act on, across every AI agent in the flow.
 *
 * With several agents a field is drivable when they agree on it: one flow input feeding
 * it, or one literal fixing it. Where they disagree there is no single value to show or
 * write, so that field is dropped and the others still work. A flow mixing whole-object
 * and field-by-field wiring is ambiguous throughout and yields nothing.
 */
export function resolveAgentModelWiring(
	modules: FlowModule[] | undefined
): AgentModelWiring | undefined {
	const wirings = getAllModules(modules ?? [])
		.filter((m) => m.value.type === 'aiagent')
		.map((agent) => parseProviderTransform((agent.value as any).input_transforms?.['provider']))
		.filter((wiring): wiring is AgentModelWiring => wiring !== undefined)
	if (wirings.length === 0) return undefined
	if (wirings.length === 1) return wirings[0]

	const wholes = new Set(wirings.map((w) => w.whole))
	if (wholes.size === 1 && !wholes.has(undefined)) {
		return { whole: [...wholes][0], fields: {}, fixed: {} }
	}
	if (wirings.some((w) => w.whole !== undefined)) return undefined

	const fields: AgentModelWiring['fields'] = {}
	const fixed: AgentModelWiring['fixed'] = {}
	for (const field of PROVIDER_FIELDS) {
		const inputs = new Set(wirings.map((w) => w.fields[field]).filter((n) => n !== undefined))
		if (inputs.size === 1) {
			fields[field] = [...inputs][0]
			continue
		}
		// One agent reading it from an input while another fixes it: no single answer.
		if (inputs.size > 1) continue
		const literals = new Set(
			wirings.map((w) => w.fixed[field]).filter((v) => v !== undefined).map((v) => JSON.stringify(v))
		)
		if (literals.size === 1) fixed[field] = JSON.parse([...literals][0])
	}
	return { fields, fixed }
}

/**
 * Why the chat cannot run, when the agent's own provider is incomplete.
 *
 * A freshly added agent carries `{ kind: 'openai', model: '', resource: '' }`, so it names
 * a provider kind while having nothing to call — the run fails and the chat can do nothing
 * about it, because no flow input feeds either field. Saying so beats a dead model button.
 * A field the flow exposes is never a gap: the reader picks it in the composer.
 */
export function agentModelGap(wiring: AgentModelWiring | undefined): string | undefined {
	// No agent, several of them, or an expression we cannot read: not ours to judge.
	if (!wiring || wiring.whole) return undefined
	const missing = (field: ProviderField) =>
		wiring.fields[field] === undefined &&
		(wiring.fixed[field] === undefined || wiring.fixed[field] === '')
	return missing('resource') || missing('model')
		? 'Pick a provider and model on the AI agent step to use this chat.'
		: undefined
}

/** Every flow input the wiring reads, so the modal does not ask for them a second time. */
export function agentModelWiringInputs(wiring: AgentModelWiring | undefined): string[] {
	if (!wiring) return []
	return [...(wiring.whole ? [wiring.whole] : []), ...Object.values(wiring.fields)]
}

export function isEmptyAgentChatInputValue(value: any): boolean {
	if (value === undefined || value === null || value === '') return true
	return Array.isArray(value) && value.length === 0
}

/**
 * The flow inputs that an AI agent step reads directly into one of its chat-relevant
 * fields. Several agents may resolve to the same flow input; it is one chip either way,
 * and one that stays unambiguous however many agents read it.
 */
export function resolveAgentChatInputs(
	modules: FlowModule[] | undefined,
	additionalInputsSchema: Record<string, any> | undefined
): AgentChatInput[] {
	const properties = additionalInputsSchema?.properties
	if (!modules || !properties) return []
	const required: string[] = Array.isArray(additionalInputsSchema?.required)
		? additionalInputsSchema.required
		: []

	const keyOf = new Map<string, AgentChatInputKey>()
	for (const module of getAllModules(modules)) {
		if (module.value.type !== 'aiagent') continue
		const transforms = module.value.input_transforms ?? {}
		for (const key of AGENT_CHAT_INPUT_KEYS) {
			const name = flowInputRef(transforms[key])
			// A name the schema doesn't declare has no field to promote, and `user_message`
			// is already the composer itself.
			if (!name || !(name in properties) || keyOf.has(name)) continue
			keyOf.set(name, key)
		}
	}

	return [...keyOf.entries()]
		.map(([name, key]) => ({
			name,
			key,
			property: properties[name],
			required: required.includes(name)
		}))
		.sort((a, b) => AGENT_CHAT_INPUT_KEYS.indexOf(a.key) - AGENT_CHAT_INPUT_KEYS.indexOf(b.key))
}
