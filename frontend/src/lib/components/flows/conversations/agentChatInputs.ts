import type { AIProvider, FlowModule, InputTransform } from '$lib/gen'
import { getReasoningCapability } from '$lib/components/copilot/reasoningRegistry'
import { parseExpressionAt } from 'acorn'

/**
 * The flow's own AI agent steps, including those inside loops and branches but never one
 * carried as another agent's tool.
 *
 * The graph walks an agent's tools as if they were child steps (flowTree.ts), which is
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

/** Block and line comments removed, so what is left is only what affects the value. */
function withoutComments(source: string): string {
	return source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/[^\n]*/g, '')
}

/**
 * An expression wrapped so acorn will read it: parenthesised, because a leading `{` would
 * otherwise parse as a block, and on its own line, because a trailing `// comment` would
 * otherwise swallow the closing paren and make the whole thing unparseable.
 */
function parenthesised(expr: string): string {
	return `(\n${expr}\n)`
}

/** The flow input the server requires on a chat-enabled flow, and stores as the message. */
const MESSAGE_INPUT = 'user_message'

/**
 * The agents the reader is talking to: the ones the chat's message is fed to.
 *
 * A flow commonly runs one agent on the message and others on work of their own — a
 * critic reading `results.x`, a classifier in a branch. Those never see what was typed,
 * so what they run on is not a setting this conversation has: letting one of them differ
 * on the model would take the model control away from the agent that does answer.
 *
 * The message need not be the whole prompt — an author wraps it in context freely — so
 * this asks whether the expression reads it at all. A flow where no agent reads it is one
 * shaped in some way this cannot speak for, and every agent counts again rather than none.
 */
function chatFacingAgents(modules: FlowModule[] | undefined): FlowModule[] {
	const agents = agentSteps(modules)
	const facing = agents.filter((module) => {
		const transform = (module.value as any).input_transforms?.[MESSAGE_INPUT]
		return transform?.type === 'javascript' && readsFlowInput(transform.expr, MESSAGE_INPUT)
	})
	return facing.length > 0 ? facing : agents
}

/**
 * Whether an expression reads `flow_input.<name>` anywhere in it.
 *
 * Parsed rather than matched: the author may write `flow_input['user_message']` as readily
 * as the dot form the editor emits, and a mention inside a comment or a string is not a
 * read. Reading two inputs is still a read of each, which is why this is not the question
 * "which single input feeds a field" that the composer asks of a wired field.
 */
function readsFlowInput(expr: string, name: string): boolean {
	let root: unknown
	try {
		root = parseExpressionAt(parenthesised(expr), 0, { ecmaVersion: 'latest' })
	} catch {
		return false
	}
	let found = false
	const visit = (node: any) => {
		if (found || !node || typeof node !== 'object') return
		if (Array.isArray(node)) {
			node.forEach(visit)
			return
		}
		if (flowInputName(node) === name) {
			found = true
			return
		}
		for (const key of Object.keys(node)) {
			if (key === 'type' || key === 'start' || key === 'end') continue
			visit(node[key])
		}
	}
	visit(root)
	return found
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

	// The author's own text may already be wrapped, so any balanced surround is fine — what
	// the span check rejects is an expression with something else beside it.
	const source = parenthesised(transform.expr)
	let node: any
	try {
		node = parseExpressionAt(source, 0, { ecmaVersion: 'latest' })
	} catch {
		return undefined
	}
	// A comment beside the expression is not another expression: the author annotating their
	// own provider must not cost them the model control. Dropped before the check so what
	// remains is only what would change the value — and so a paren inside a comment is not
	// counted as one of the wrapping pair.
	const before = withoutComments(source.slice(0, node.start))
	const after = withoutComments(source.slice(node.end))
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
	const agents = chatFacingAgents(modules)
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
 * The provider fields the model button edits. Everything else wired to an input is left to
 * the Configure-inputs modal, and the button draws no control for it.
 *
 * A field is the button's only where its control can be used, which each field makes depend
 * on the one before it. An input promoted without a usable control is one nothing can edit.
 * - `resource` is always usable: the submenu lists the workspace's AI resources.
 * - `kind` is written only alongside a resource, since a provider is picked as a pair. Wired
 *   with the resource fixed, the button has nothing to write it with.
 * - `model` needs a provider to list models for and to gate the typed entry: a wired resource
 *   or kind, or one fixed kind. Agents that fix different kinds leave none.
 * - `reasoning_effort` needs a model to place the effort on: a driven model or one fixed
 *   model. Agents that fix different models leave none.
 */
export function composerDrivenFields(wiring: AgentModelWiring): Set<ProviderField> {
	if (wiring.whole) return new Set(PROVIDER_FIELDS)
	const wired = (field: ProviderField) => wiring.fields[field] !== undefined
	const driven = new Set<ProviderField>()
	if (wired('resource')) {
		driven.add('resource')
		if (wired('kind')) driven.add('kind')
	}
	const providerKnown = wired('resource') || wired('kind') || fixedOne(wiring, 'kind')
	if (wired('model') && providerKnown) driven.add('model')
	const modelKnown = driven.has('model') || fixedOne(wiring, 'model')
	if (wired('reasoning_effort') && modelKnown) driven.add('reasoning_effort')
	return driven
}

/** Whether every agent fixes the field to the same non-empty literal. */
function fixedOne(wiring: AgentModelWiring, field: ProviderField): boolean {
	const value = wiring.fixed[field]
	return value !== undefined && value !== ''
}

/** The flow inputs the model button writes, so the modal does not ask for them twice. */
export function agentModelWiringInputs(wiring: AgentModelWiring | undefined): string[] {
	if (!wiring) return []
	if (wiring.whole) return [wiring.whole]
	return [...composerDrivenFields(wiring)]
		.map((field) => wiring.fields[field])
		.filter((name): name is string => !!name)
}

/**
 * Whether the composer draws a model button at all: something to write, or a fixed model to
 * name. Agents that fix different models leave neither.
 */
export function showsModelButton(wiring: AgentModelWiring | undefined): boolean {
	if (!wiring) return false
	return agentModelWiringInputs(wiring).length > 0 || fixedOne(wiring, 'model')
}

/**
 * The flow inputs the composer edits, and therefore the ones the Configure-inputs modal must
 * not ask for. The modal is whatever is left, so this is the single answer to "who edits
 * this" rather than a list kept in step with the controls that render.
 *
 * Only the shape of the flow decides it. Whether a control can act *right now* — no rules
 * for a provider's thinking levels, say — is a state that control shows, not a reason to
 * hand the input to an editor that would be no more able. The attachments target is the
 * composer's other owner: the input its paperclip uploads into, where the flow has one.
 */
export function composerOwnedInputs(
	wiring: AgentModelWiring | undefined,
	attachmentsTarget: { name: string } | undefined
): string[] {
	return [...agentModelWiringInputs(wiring), ...(attachmentsTarget ? [attachmentsTarget.name] : [])]
}

/**
 * The run's inputs with a reasoning effort the chosen model cannot take removed.
 *
 * The model button reconciles the two when the reader switches model, which covers the only
 * way the composer can put them out of step. It is not the only way they get out of step:
 * a value stored from an earlier visit, a default the flow author wrote, or a model chosen
 * before the effort was, all arrive already mismatched — and the provider answers a
 * mismatch with a 400 that names neither input ("adaptive thinking is not supported on this
 * model"). Checked here, where the run's arguments are settled, so every route is covered.
 *
 * Only where the registry positively knows the model rejects it. An unknown family keeps
 * whatever the author wrote: dropping a value on a guess would override their own default.
 */
export function withoutRejectedEffort(
	wiring: AgentModelWiring | undefined,
	values: Record<string, any>
): Record<string, any> {
	if (!wiring) return values

	// One input carrying the whole provider object: the three fields are read from it and
	// the effort is cleared inside it, since that is where the agent will look for them.
	if (wiring.whole) {
		const provider = values[wiring.whole]
		if (!provider || typeof provider !== 'object') return values
		if (!rejectsEffort(provider.kind, provider.model, provider.reasoning_effort)) return values
		return { ...values, [wiring.whole]: { ...provider, reasoning_effort: '' } }
	}

	const effortInput = wiring.fields.reasoning_effort
	if (!effortInput) return values
	const kindInput = wiring.fields.kind
	const modelInput = wiring.fields.model
	const rejected = rejectsEffort(
		kindInput ? values[kindInput] : wiring.fixed.kind,
		modelInput ? values[modelInput] : wiring.fixed.model,
		values[effortInput]
	)
	return rejected ? { ...values, [effortInput]: '' } : values
}

/** Whether the registry positively says this model will not take this effort. */
function rejectsEffort(provider: unknown, model: unknown, effort: unknown): boolean {
	if (typeof effort !== 'string' || effort === '') return false
	if (typeof provider !== 'string' || typeof model !== 'string' || !provider || !model) {
		return false
	}
	const capability = getReasoningCapability(provider as AIProvider, model)
	return capability.known && !capability.supported
}
