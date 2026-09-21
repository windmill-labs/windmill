import type { FlowModule, InputTransform } from '$lib/gen'
import { parse, parseExpressionAt } from 'acorn'

/**
 * The flow's own AI agent steps, including those inside loops and branches. An agent carried
 * as another agent's tool is left out, unlike in the graph (flowTree.ts): its inputs come from
 * the agent calling it, not from the chat.
 */
export function agentSteps(modules: FlowModule[] | undefined): FlowModule[] {
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
			} else if (value?.type === 'branchone' || value?.type === 'aidecision') {
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
 * AI agent inputs the chat composer drives, through the flow input the author wired to each:
 * the composer writes run inputs, never the flow. Only what the person chatting owns turn to
 * turn belongs here. `system_prompt`, `temperature` and the like shape the agent for everyone
 * who runs the flow, so they stay in the Configure-inputs modal.
 */
export const AGENT_CHAT_INPUT_KEYS = ['user_attachments'] as const

export type AgentChatInputKey = (typeof AGENT_CHAT_INPUT_KEYS)[number]

/** The key that rides one message rather than the conversation, cleared on send. */
export const PER_TURN_AGENT_CHAT_INPUT_KEY: AgentChatInputKey = 'user_attachments'

export type AgentChatInput = {
	/** Flow input property feeding the agent field. */
	name: string
	key: AgentChatInputKey
	/** The flow input's own schema entry. */
	property: Record<string, any>
}

/**
 * The `flow_input` properties an expression reads, from its syntax tree rather than its text,
 * so a mention in a comment, a string or a nested path (`results.a.flow_input.x`) is not a read.
 * A transform may be a statement body with `return`. Undefined when it does not parse.
 */
export function flowInputReads(expr: string): Set<string> | undefined {
	let root: unknown
	try {
		root = parseExpressionAt(`(\n${expr}\n)`, 0, { ecmaVersion: 'latest' })
	} catch {
		try {
			root = parse(expr, {
				ecmaVersion: 'latest',
				allowReturnOutsideFunction: true,
				allowAwaitOutsideFunction: true
			})
		} catch {
			return undefined
		}
	}
	const names = new Set<string>()
	const visit = (node: any) => {
		if (!node || typeof node !== 'object') return
		if (Array.isArray(node)) return node.forEach(visit)
		if (
			node.type === 'MemberExpression' &&
			node.object?.type === 'Identifier' &&
			node.object.name === 'flow_input'
		) {
			if (!node.computed && node.property?.type === 'Identifier') names.add(node.property.name)
			else if (node.computed && typeof node.property?.value === 'string')
				names.add(node.property.value)
		}
		for (const key in node) if (key !== 'type') visit(node[key])
	}
	visit(root)
	return names
}

/**
 * The flow input a transform reads, when it reads exactly one. The expression may reshape it
 * (`(flow_input.files || []).map(...)`) and still counts. With `declared`, only the flow's own
 * inputs count, so a loop's `flow_input.iter` does not make a read ambiguous.
 */
export function flowInputRef(
	transform: InputTransform | undefined,
	declared?: Record<string, unknown>
): string | undefined {
	if (transform?.type !== 'javascript') return undefined
	const reads = flowInputReads(transform.expr)
	if (!reads) return undefined
	const names = declared ? [...reads].filter((name) => name in declared) : [...reads]
	return names.length === 1 ? names[0] : undefined
}

/** Whether a schema entry holds an s3 file, as the flow input editor recognises one. */
function holdsS3File(property: Record<string, any> | undefined): boolean {
	return (
		property?.format === 'resource-s3_object' ||
		property?.resourceType === 's3object' ||
		property?.resourceType === 's3_object'
	)
}

/** The flow input the composer's attachments feed, whether it holds a list, and whether the
 * flow requires it (so a message without a file cannot run). */
export type AttachmentsTarget = { name: string; multiple: boolean; required?: boolean }

/**
 * Where the composer's attachments go: the promoted input, only when its schema holds s3
 * objects. A transform may build the s3 object from a plain string input, and writing
 * `{ s3, filename }` into that input would fail at run time.
 */
export function attachmentsTargetFor(
	input: AgentChatInput | undefined
): AttachmentsTarget | undefined {
	if (!input) return undefined
	if (holdsS3File(input.property)) return { name: input.name, multiple: false }
	return input.property?.type === 'array' && holdsS3File(input.property.items)
		? { name: input.name, multiple: true }
		: undefined
}

/**
 * The flow inputs that an AI agent step reads directly into one of its chat-relevant
 * fields. Several agents may read the same flow input; it is promoted once.
 */
export function resolveAgentChatInputs(
	modules: FlowModule[] | undefined,
	additionalInputsSchema: Record<string, any> | undefined
): AgentChatInput[] {
	const properties = additionalInputsSchema?.properties
	if (!modules || !properties) return []

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
			// composer drives. An expression that does not parse is kept, as unreadable.
			if (transform?.type !== 'javascript') continue
			const reads = flowInputReads(transform.expr)
			if (reads ? reads.size === 0 : !transform.expr.includes('flow_input')) continue
			// Reading no declared input, or two of them, names none: this agent reads something
			// the composer cannot drive, which is what disagreement means here.
			const usable = flowInputRef(transform, properties)
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

	return [...keyOf.entries()].map(([name, key]) => ({
		name,
		key,
		property: properties[name]
	}))
}
