import { deepEqual } from 'fast-equals'
import type { InputTransform } from '$lib/gen'

/**
 * How the AI agent form presents `AI_AGENT_SCHEMA`: which group a field belongs to, what it is
 * called, and what leaving it unset actually does at runtime.
 *
 * Kept apart from the schema because none of it is JSON Schema: the schema stays the contract with
 * the backend, this is the contract with the reader. It imports nothing from `flowInfers.ts`, so
 * the two cannot cycle.
 */

export type AgentFieldGroup = 'model' | 'messages' | 'tools' | 'output'

/** Groups in the order a run assembles the request: pick a model, build the messages, let it call
 *  tools, shape what comes back. */
export const AGENT_FIELD_GROUPS: { id: AgentFieldGroup; label: string }[] = [
	{ id: 'model', label: 'Model' },
	{ id: 'messages', label: 'Messages' },
	{ id: 'tools', label: 'Tools' },
	{ id: 'output', label: 'Output' }
]

/** The tool roster, which reads `flowModule.value.tools` rather than an `input_transforms` key.
 *  It lives in the registry so the groups keep a single ordering. */
export const AGENT_TOOLS_ROW = 'tools'

export interface AgentFieldSpec {
	key: string
	group: AgentFieldGroup
	/** Names the field wherever it appears: its row, the add menu, a linked agent's summary. */
	label: string
	tooltip?: string
	/** Always rendered, and not removable. */
	core?: boolean
	/** Rendered by the form itself rather than as an `input_transforms` row. */
	virtual?: boolean
	/** The value a run cannot tell apart from an absent key, which is what makes holding it mean
	 *  "unset". Read off the backend rather than `schema.default`, which the two have disagreed on
	 *  before. Also what the add menu seeds the field with, so a new row opens showing what it
	 *  overrides. */
	implicit?: unknown
	/** The same value written for a reader, shown under the field's name in the add menu. */
	defaultHint?: string
	/** Ignored for image output, so the field hides while `output_type` is `'image'`. */
	textOnly?: boolean
}

export const AGENT_FIELDS: AgentFieldSpec[] = [
	{
		key: 'provider',
		group: 'model',
		label: 'Provider',
		core: true
	},
	{
		key: 'temperature',
		group: 'model',
		label: 'Temperature',
		tooltip: 'How random the generation is, from 0 for deterministic up to 2.',
		defaultHint: 'Default: the provider decides',
		textOnly: true
	},
	{
		key: 'max_completion_tokens',
		group: 'model',
		label: 'Max output tokens',
		tooltip: 'The most tokens the model may produce in its answer.',
		defaultHint: 'Default: the provider decides'
	},
	{
		key: 'system_prompt',
		group: 'messages',
		label: 'System message',
		tooltip: 'Sets how the agent should behave. Sent ahead of everything else.',
		core: true
	},
	{
		key: 'memory',
		group: 'messages',
		label: 'Memory',
		tooltip:
			'History sent between the system message and the user message. Windmill can keep it for you, or you can supply the messages yourself.',
		implicit: { kind: 'off' },
		defaultHint: 'Default: off',
		textOnly: true
	},
	{
		key: 'user_message',
		group: 'messages',
		label: 'User message',
		tooltip:
			"The user turn, sent after the system message and any history. Turn on chat input on the flow's input interface to feed it from the chat.",
		core: true
	},
	{
		key: 'user_attachments',
		group: 'messages',
		label: 'Attachments',
		tooltip: 'Images or PDFs sent along with the user message. Needs S3 storage on the workspace.',
		implicit: [],
		defaultHint: 'Default: none'
	},
	{
		key: AGENT_TOOLS_ROW,
		group: 'tools',
		label: 'Tools',
		core: true,
		virtual: true
	},
	{
		key: 'max_iterations',
		group: 'tools',
		label: 'Max iterations',
		tooltip:
			'How many times the agent may go round the loop of calling the model and running the tools it asks for. One iteration can run several tools. If it is still calling tools at the last one, the step fails and returns the messages so far. Between 1 and 1000.',
		implicit: 10,
		defaultHint: 'Default: 10'
	},
	{
		key: 'output_type',
		group: 'output',
		label: 'Output type',
		tooltip:
			'Image output needs S3 storage on the workspace, ignores tools, and works with OpenAI, Google AI and the OpenRouter gemini-image-preview model.',
		implicit: 'text',
		defaultHint: 'Default: text'
	},
	{
		key: 'output_schema',
		group: 'output',
		label: 'Output schema',
		tooltip: 'A JSON schema the answer has to follow.',
		defaultHint: 'Default: none',
		textOnly: true
	},
	{
		key: 'streaming',
		group: 'output',
		label: 'Stream the response',
		tooltip: 'Send the answer back as it is generated, rather than once it is complete.',
		implicit: true,
		defaultHint: 'Default: on',
		textOnly: true
	}
]

export const AGENT_FIELD_BY_KEY: Record<string, AgentFieldSpec> = Object.fromEntries(
	AGENT_FIELDS.map((f) => [f.key, f])
)

/** Keys that mean nothing for image output, so the form drops them while `output_type` is
 *  `'image'`. `temperature` is genuinely forwarded to an image request, so this is a clear and not
 *  merely a hide. */
export const AGENT_TEXT_ONLY_KEYS = AGENT_FIELDS.filter((f) => f.textOnly).map((f) => f.key)

/**
 * Whether a transform holds something a run would do differently from an absent key. Core fields
 * are always set: they are what an agent is.
 */
export function agentFieldIsSet(
	spec: AgentFieldSpec,
	transform: InputTransform | any | undefined
): boolean {
	if (spec.core) return true
	if (!transform || typeof transform !== 'object') return false
	if (transform.type === 'javascript') return Boolean(transform.expr)
	// An AI-filled field is the agent tool case: the value arrives at runtime, so it is set.
	if (transform.type === 'ai') return true
	const value = transform.value
	// `null` as well as `undefined`: a placeholder transform serializes as `{"type":"static"}` and
	// comes back from the API with an explicit `"value": null`, so the two shapes are the same field.
	if (value === undefined || value === null) return false
	if (spec.implicit !== undefined && deepEqual(value, spec.implicit)) return false
	return true
}

/**
 * Whether a run of this step would stream its answer. Absence means on
 * (`args.streaming.unwrap_or(true)` in `ai_executor.rs`), so only an explicit `false` holds it
 * back; a transform resolved at runtime is assumed to leave the default alone.
 */
export function agentStreamingEnabled(transform: InputTransform | any | undefined): boolean {
	return !(transform?.type === 'static' && transform.value === false)
}

/**
 * Whether the current schema carries this field at all. A linked step's schema is reduced to the
 * flow-local inputs, which is what collapses its form to the Messages group on its own.
 */
export function agentFieldAppliesTo(
	spec: AgentFieldSpec,
	schemaProperties: Record<string, any> | undefined
): boolean {
	// A virtual row has no schema key to look for, so it keys off the brain being editable here.
	if (spec.virtual) return Boolean(schemaProperties && 'provider' in schemaProperties)
	return Boolean(schemaProperties && spec.key in schemaProperties)
}

/**
 * The fields to render when a step is first opened. Visibility is sticky from here on: the form
 * only ever adds to this set, so emptying a textbox never makes its row vanish under the cursor.
 */
export function initialVisibleAgentFields(
	args: Record<string, InputTransform | any> | undefined,
	schemaProperties: Record<string, any> | undefined
): Set<string> {
	const visible = new Set<string>()
	for (const spec of AGENT_FIELDS) {
		if (!agentFieldAppliesTo(spec, schemaProperties)) continue
		if (agentFieldIsSet(spec, args?.[spec.key])) visible.add(spec.key)
	}
	return visible
}
