import type { FlowModule, InputTransform } from '$lib/gen'

/**
 * The flow's own AI agent steps, including those inside loops and branches but never one
 * carried as another agent's tool.
 *
 * The graph walks an agent's tools as if they were child steps (flowTree.ts), which is
 * right for the graph and wrong here: a tool agent's inputs belong to the agent that
 * calls it, not to the chat. Counting it would let a nested agent's wiring speak for the
 * step the reader is actually talking to.
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
 * AI agent inputs the chat composer can drive.
 *
 * The composer never edits the flow: an agent field is reachable only when the author
 * wired a flow input to it, so what the composer writes is a run input like any other. That
 * also keeps it working on the deployed chat, where the reader has no write access, and
 * on a step linked to an `ai_agent` resource, where every field but `user_message` /
 * `user_attachments` comes from the resource and is not overridable at all.
 *
 * Only what the person chatting legitimately owns turn to turn belongs here, which today
 * is the files they attach and nothing else. `system_prompt`, `temperature` and
 * `max_completion_tokens` shape how the agent behaves for everyone who runs the flow —
 * surfacing them per conversation invites tuning the flow from the chat instead of
 * fixing it in the editor. They stay flow settings, reachable through Configure inputs
 * when the author deliberately exposes them. `max_iterations` is absent for the same
 * reason, and because it caps the tool-use loop rather than a single generation.
 */
export const AGENT_CHAT_INPUT_KEYS = ['user_attachments'] as const

export type AgentChatInputKey = (typeof AGENT_CHAT_INPUT_KEYS)[number]

/** The key that rides one message rather than the conversation, which the composer
 * clears on send. A later key of the other sort would not be this one. */
export const PER_TURN_AGENT_CHAT_INPUT_KEY: AgentChatInputKey = 'user_attachments'

export type AgentChatInput = {
	/** Flow input property feeding the agent field. */
	name: string
	key: AgentChatInputKey
	/** The flow input's own schema entry — the chip renders it with the same editor the modal would. */
	property: Record<string, any>
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

/** Whether a schema entry holds an s3 file, as the flow input editor recognises one. */
function holdsS3File(property: Record<string, any> | undefined): boolean {
	return (
		property?.format === 'resource-s3_object' ||
		property?.resourceType === 's3object' ||
		property?.resourceType === 's3_object'
	)
}

/** The flow input the composer's attachments feed, and whether it holds a list. */
export type AttachmentsTarget = { name: string; multiple: boolean }

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
): AttachmentsTarget | undefined {
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

	return [...keyOf.entries()].map(([name, key]) => ({
		name,
		key,
		property: properties[name]
	}))
}
