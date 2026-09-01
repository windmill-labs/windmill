import type { FlowModule, InputTransform } from '$lib/gen'
import { getAllModules } from '../flowExplorer'
import { Bot, Hash, Paperclip, ScrollText, Thermometer } from 'lucide-svelte'
import type { ComponentType } from 'svelte'

/**
 * AI agent inputs the chat composer can drive, in footer display order.
 *
 * The composer never edits the flow: an agent field is reachable only when the author
 * wired a flow input to it, so what the chip writes is a run input like any other. That
 * also keeps it working on the deployed chat, where the reader has no write access, and
 * on a step linked to an `ai_agent` resource, where every field but `user_message` /
 * `user_attachments` comes from the resource and is not overridable at all.
 *
 * `max_iterations` is deliberately absent: it caps the agent's tool-use loop rather than
 * a single generation, so it belongs with the flow's settings, not the model's.
 */
export const AGENT_CHAT_INPUT_KEYS = [
	'user_attachments',
	'provider',
	'system_prompt',
	'temperature',
	'max_completion_tokens'
] as const

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

export const AGENT_CHAT_INPUT_META: Record<
	AgentChatInputKey,
	{ icon: ComponentType; label: string; summarize: (value: any) => string | undefined }
> = {
	user_attachments: {
		icon: Paperclip,
		label: 'Attach',
		summarize: (value) => {
			const count = Array.isArray(value) ? value.length : value ? 1 : 0
			return count > 0 ? String(count) : undefined
		}
	},
	provider: {
		icon: Bot,
		label: 'Model',
		summarize: (value) => (typeof value?.model === 'string' ? value.model : undefined)
	},
	system_prompt: {
		icon: ScrollText,
		label: 'System prompt',
		summarize: (value) => (typeof value === 'string' && value.trim() !== '' ? 'set' : undefined)
	},
	temperature: {
		icon: Thermometer,
		label: 'Temperature',
		summarize: (value) => (typeof value === 'number' ? String(value) : undefined)
	},
	max_completion_tokens: {
		icon: Hash,
		label: 'Max tokens',
		summarize: (value) => (typeof value === 'number' ? String(value) : undefined)
	}
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

/**
 * The model the flow already fixes, when it fixes exactly one: a single AI agent step
 * whose `provider` is a static value. Named on the settings trigger the way the session
 * chat names its own model, but not editable — no flow input feeds it, so there is
 * nothing the composer could write.
 */
export function resolveStaticAgentModel(
	modules: FlowModule[] | undefined
): AgentModel | undefined {
	const agents = getAllModules(modules ?? []).filter((m) => m.value.type === 'aiagent')
	if (agents.length !== 1) return undefined
	const provider = (agents[0].value as any).input_transforms?.['provider']
	if (provider?.type !== 'static') return undefined
	const value = provider.value
	return typeof value?.model === 'string'
		? { kind: value.kind, model: value.model, reasoning_effort: value.reasoning_effort }
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
