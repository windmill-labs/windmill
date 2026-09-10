import type { FlowModule, InputTransform } from '$lib/gen'
import { parseExpressionAt } from 'acorn'

/**
 * The flow's own AI agent steps, including those inside loops and branches but never one
 * carried as another agent's tool.
 *
 * getAllModules walks an agent's tools as if they were child steps (flowTree.ts), which is
 * right for the graph and wrong here: a tool agent's provider belongs to the agent that
 * calls it, not to the chat. Counting it would let a nested agent's fixed model defeat the
 * composer's model control on the step the reader is actually talking to.
 */
function agentSteps(modules: FlowModule[] | undefined): FlowModule[] {
	const found: FlowModule[] = []
	const walk = (mods: FlowModule[]) => {
		for (const module of mods) {
			const value = module.value as any
			if (value?.type === 'aiagent') {
				found.push(module)
				continue
			}
			if (value?.type === 'forloopflow' || value?.type === 'whileloopflow') {
				walk(value.modules ?? [])
			} else if (value?.type === 'branchone') {
				walk(value.default ?? [])
				for (const branch of value.branches ?? []) walk(branch.modules ?? [])
			} else if (value?.type === 'branchall') {
				for (const branch of value.branches ?? []) walk(branch.modules ?? [])
			}
		}
	}
	walk(modules ?? [])
	return found
}

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
	const names = new Set([...transform.expr.matchAll(FLOW_INPUT_REF)].map((match) => match[1]))
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
	/**
	 * One of the agents names no resource or no model of its own and no flow input feeds
	 * it, so that agent's run fails whatever the others do. Held apart from the fields,
	 * which describe what the composer may offer.
	 */
	someAgentCannotRun?: boolean
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

/** How one agent supplies a provider field: from an input, as a literal, or not at all. */
type FieldSupply =
	| { kind: 'wired'; name: string }
	| { kind: 'fixed'; value: any }
	| { kind: 'absent' }

/** Whether one agent supplies a field with nothing usable: no input, and no literal. */
function agentFieldEmpty(wiring: AgentModelWiring, field: ProviderField): boolean {
	if (wiring.fields[field] !== undefined) return false
	const value = wiring.fixed[field]
	return value === undefined || value === ''
}

function fieldSupply(wiring: AgentModelWiring, field: ProviderField): FieldSupply {
	const name = wiring.fields[field]
	if (name !== undefined) return { kind: 'wired', name }
	const value = wiring.fixed[field]
	if (value !== undefined) return { kind: 'fixed', value }
	return { kind: 'absent' }
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
	const agents = agentSteps(modules)
	const parsed = agents.map((agent) =>
		parseProviderTransform((agent.value as any).input_transforms?.['provider'])
	)
	if (parsed.length === 0) return undefined
	// An agent whose provider cannot be read is an agent the composer cannot speak for:
	// dropping it would let the rest declare a control that governs only some of them.
	if (parsed.some((wiring) => wiring === undefined)) return undefined
	const wirings = parsed as AgentModelWiring[]
	// Whether any single agent has nothing to call, which stays true however the others
	// are wired — the gap message is about that agent, not about their agreement.
	const someAgentCannotRun = wirings.some(
		(wiring) =>
			!wiring.whole && (agentFieldEmpty(wiring, 'resource') || agentFieldEmpty(wiring, 'model'))
	)
	if (wirings.length === 1) return { ...wirings[0], someAgentCannotRun }

	const wholes = new Set(wirings.map((w) => w.whole))
	if (wholes.size === 1 && !wholes.has(undefined)) {
		return { whole: [...wholes][0], fields: {}, fixed: {}, someAgentCannotRun }
	}
	if (wirings.some((w) => w.whole !== undefined)) return undefined

	const fields: AgentModelWiring['fields'] = {}
	const fixed: AgentModelWiring['fixed'] = {}
	for (const field of PROVIDER_FIELDS) {
		// Every agent has to supply the field the same way for the composer to speak for
		// them all. One wired name among agents that otherwise fix it is not agreement:
		// the control would move that one agent and leave the others where they are.
		const supplies = new Set(wirings.map((w) => JSON.stringify(fieldSupply(w, field))))
		// Disagreement leaves the field neither editable nor known: a control offered here
		// would govern one agent while the rest ran on something else.
		if (supplies.size > 1) continue
		const supply: FieldSupply = JSON.parse([...supplies][0])
		if (supply.kind === 'wired') fields[field] = supply.name
		else if (supply.kind === 'fixed') fixed[field] = supply.value
	}
	return { fields, fixed, someAgentCannotRun }
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
	// Asked of each agent rather than of what they agree on: agents that merely disagree
	// about the model all have one, and the message would be false — while an agent with
	// an empty model still cannot run, however well the others are configured.
	return wiring.someAgentCannotRun
		? 'Pick a provider and model on the AI agent step to use this chat.'
		: undefined
}

/**
 * The flow inputs the model button actually writes, so the modal does not ask for them a
 * second time — and, just as much, so it still asks for the ones the button cannot reach.
 *
 * `kind` is the one to watch: the button writes it only alongside a resource, since a
 * provider is picked as a pair. A flow that wires `kind` to an input while fixing the
 * resource leaves the button nothing to write it with, and hiding it would leave the run
 * without a provider kind and no way to supply one.
 *
 * `reasoning_effort` is unconditional by contrast. Once a model is chosen the button has a
 * control for it — the slider, or the row saying the model cannot think — and on a model
 * that cannot, the value is not a choice but a fact the button writes itself. Offering it in
 * the modal as well would invite setting a level the provider then rejects. Before a model
 * is chosen it is offered nowhere, which is the honest answer: an effort means nothing
 * until there is something to spend it on, and picking a model writes one.
 */
export function agentModelWiringInputs(wiring: AgentModelWiring | undefined): string[] {
	if (!wiring) return []
	if (wiring.whole) return [wiring.whole]
	const driven: ProviderField[] = ['resource', 'model', 'reasoning_effort']
	if (wiring.fields.resource !== undefined) driven.push('kind')
	return driven.map((field) => wiring.fields[field]).filter((name): name is string => !!name)
}

/** Whether a schema entry holds an s3 file, as the flow input editor recognises one. */
function holdsS3File(property: Record<string, any> | undefined): boolean {
	return (
		property?.format === 'resource-s3_object' ||
		property?.resourceType === 's3object' ||
		property?.resourceType === 's3_object'
	)
}

/**
 * Where the composer's attachments go, or nothing when there is nowhere they fit.
 *
 * The agent reads `user_attachments` through a transform that may reshape what it takes, so
 * the flow input feeding it is not necessarily an s3 field: an expression building the s3
 * object itself promotes a plain string. Writing `{ s3, filename }` into that input fails at
 * run time, so the paperclip appears only where the schema says the value belongs.
 */
export function attachmentsTargetFor(
	input: AgentChatInput | undefined
): { name: string; multiple: boolean } | undefined {
	if (!input) return undefined
	if (holdsS3File(input.property)) return { name: input.name, multiple: false }
	return input.property?.type === 'array' && holdsS3File(input.property.items)
		? { name: input.name, multiple: true }
		: undefined
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

	// One input per key, and only when every agent reading that key reads the same one:
	// the composer writes a single flow input, so promoting one of two would feed one
	// agent and leave the other with nothing — while hiding both from the modal, where
	// the reader could at least have filled them in.
	const namesPerKey = new Map<AgentChatInputKey, Set<string | undefined>>()
	for (const module of agentSteps(modules)) {
		const transforms = (module.value as any).input_transforms ?? {}
		for (const key of AGENT_CHAT_INPUT_KEYS) {
			const transform = transforms[key]
			// An agent that feeds the key from anything but a flow input — a literal, another
			// step's result, or the empty placeholder every agent step carries for the keys of
			// AI_AGENT_SCHEMA — is not reading an input, so it has no say in which one the
			// composer drives.
			if (transform?.type !== 'javascript' || !transform.expr.includes('flow_input')) continue
			const name = flowInputRef(transform)
			// A name the schema doesn't declare has no field to promote, and one expression
			// reading two inputs names none: either way this agent reads something the
			// composer cannot drive, which is what disagreement means here.
			const usable = name && name in properties ? name : undefined
			const names = namesPerKey.get(key) ?? new Set<string | undefined>()
			names.add(usable)
			namesPerKey.set(key, names)
		}
	}

	const keyOf = new Map<string, AgentChatInputKey>()
	for (const [key, names] of namesPerKey) {
		if (names.size !== 1) continue
		const name = [...names][0]
		if (name === undefined || keyOf.has(name)) continue
		keyOf.set(name, key)
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
