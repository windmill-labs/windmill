import { deepEqual } from 'fast-equals'
import type { InputTransform, MemoryConfig } from '$lib/gen'

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

/** What an agent step produces, as `output_type` names it. */
export type AgentOutputType = 'text' | 'image' | 'decision'

/** The output types that run the agent loop, which a field serves unless it says otherwise. */
const LOOP_OUTPUT_TYPES: readonly AgentOutputType[] = ['text', 'image']
const EVERY_OUTPUT_TYPE: readonly AgentOutputType[] = ['text', 'image', 'decision']

/** The output type an `output_type` value selects. An expression is only known once a run
 *  evaluates it, so it reads as text, which is also what an absent key runs as. */
export function agentOutputType(value: unknown): AgentOutputType {
	return value === 'image' || value === 'decision' ? value : 'text'
}

/** Whether a run with this output type reads the field. */
export function agentFieldServes(spec: AgentFieldSpec, outputType: AgentOutputType): boolean {
	return (spec.outputTypes ?? LOOP_OUTPUT_TYPES).includes(outputType)
}

/** The tool roster, which reads `flowModule.value.tools` rather than an `input_transforms` key.
 *  It lives in the registry so the groups keep a single ordering. */
export const AGENT_TOOLS_ROW = 'tools'

/** A step's own history inputs. Never seeded with a placeholder: a run reads a present key as the
 *  step's choice, so only the author adds them. */
export const AGENT_HISTORY_KEYS = ['memory_id', 'previous_messages'] as const
export type AgentHistoryKey = (typeof AGENT_HISTORY_KEYS)[number]

/** What turning managed memory on writes. */
export const DEFAULT_AGENT_MEMORY: MemoryConfig = { kind: 'window', context_length: 10 }

/** The docs section on how an agent's memory is named and kept. */
export const AGENT_MEMORY_DOCS_URL =
	'https://www.windmill.dev/docs/core_concepts/ai_agents#memory-auto--manual'

/** Whether Windmill stores and replays the agent's conversation, mirroring the worker: `window`, or
 *  its older spelling `auto`, with a message count above 0. A legacy `manual` list is not managed. */
export function keepsManagedMemory(memory: any): boolean {
	return (memory?.kind === 'window' || memory?.kind === 'auto') && Boolean(memory.context_length)
}

export type AgentMemoryMode = 'legacy' | 'managed' | 'off'

/** Which shape a run reads this memory as: an older `auto`/`manual` setting, or the current one. */
export function agentMemoryMode(memory: any): AgentMemoryMode {
	if (memory?.kind === 'manual') return 'legacy'
	// The worker reads an `auto` that keeps no messages as off, history inputs included, so the form
	// offers what that run would read.
	if (memory?.kind === 'auto') return memory.context_length ? 'legacy' : 'off'
	return keepsManagedMemory(memory) ? 'managed' : 'off'
}

/** Whether a run reads this step input, mirroring the worker: managed memory reads only a memory
 *  id, memory that is off only previous messages, and an older setting neither. A setting the form
 *  cannot read yet leaves both open. */
export function historyInputApplies(
	key: AgentHistoryKey,
	mode: AgentMemoryMode | undefined
): boolean {
	if (mode === undefined) return true
	if (mode === 'legacy') return false
	return (key === 'memory_id') === (mode === 'managed')
}

/** A memory setting in words, for a linked agent's summary. */
export function describeMemoryPolicy(memory: any): string {
	if (keepsManagedMemory(memory)) return `Last ${memory.context_length} messages`
	if (memory?.kind === 'manual') return 'Off, sends previous messages saved with the agent'
	return 'Off'
}

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
	/** What the add menu opens the field on, where that is not `implicit`. Only a field whose empty
	 *  value is a choice of its own needs one: an empty `enabled_tools` advertises no tools, so its
	 *  row opens on an empty list to keep what is shown and what a run does the same thing, which
	 *  leaves an absent field as the only way to say every tool. */
	seed?: unknown
	/** What leaving the field unset does, written for a reader, shown under the field's name in the
	 *  add menu. */
	defaultHint?: string
	/** The output types whose runs read the field; it hides under any other. Unset means text and
	 *  image, the two that run the agent loop: a decision reads only its provider, state and
	 *  questions. */
	outputTypes?: readonly AgentOutputType[]
}

export const AGENT_FIELDS: AgentFieldSpec[] = [
	{
		key: 'provider',
		group: 'model',
		label: 'Provider',
		core: true,
		outputTypes: EVERY_OUTPUT_TYPE
	},
	// Not text-only, unlike the fields around it: image output runs through an ordinary chat model
	// (OpenAI's `image_generation` tool, OpenRouter's `modalities`) that samples at this
	// temperature, so hiding it would hide a setting the run still uses.
	{
		key: 'temperature',
		group: 'model',
		label: 'Temperature',
		tooltip: 'How random the generation is, from 0 for deterministic up to 2.',
		defaultHint: 'Default: the provider decides'
	},
	{
		key: 'max_completion_tokens',
		group: 'model',
		label: 'Max output tokens',
		tooltip: 'The most tokens the model may produce in its answer.',
		defaultHint: 'Default: the provider decides'
	},
	{
		key: 'state',
		group: 'messages',
		label: 'State',
		tooltip:
			'What the questions are asked about: a text, an object or a list of texts. Name each part of an object, and send only what the questions need: detail they do not need makes the answers less accurate.',
		core: true,
		outputTypes: ['decision']
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
		key: 'system_prompt',
		group: 'messages',
		label: 'System message',
		tooltip: 'Sets how the agent should behave. Sent ahead of everything else.',
		core: true
	},
	{
		key: 'memory',
		group: 'messages',
		label: 'Managed memory',
		tooltip: 'Windmill stores the conversation and sends its last messages with each request.',
		implicit: { kind: 'off' },
		defaultHint: 'Default: off',
		outputTypes: ['text']
	},
	{
		key: 'memory_id',
		group: 'messages',
		label: 'Memory id',
		tooltip:
			'Conversation history id: runs with the same id share their history. Inherited uses the memory_id the run was started with: the conversation id in chat mode, or the memory_id query parameter otherwise. Without either, each run starts fresh. Custom sets the id on the step: a fixed id shares one history across all runs, an expression keeps one history per value.',
		implicit: '',
		outputTypes: ['text']
	},
	{
		key: 'previous_messages',
		group: 'messages',
		label: 'Previous messages',
		tooltip: 'History the flow supplies, sent between the system message and the user message.',
		implicit: [],
		defaultHint: 'Default: none',
		outputTypes: ['text']
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
		key: 'enabled_tools',
		group: 'tools',
		label: 'Enabled tools',
		tooltip:
			'Which of the agent tools a run carries, so it costs no more than it needs. Selecting none leaves the agent with no tools, and unsetting the field gives it all of them. Set it to an expression to decide per run, naming each one the way this list does: a tool by its own name, an MCP server by its resource path, and web search by "__wm_web_search". An MCP server carries every tool it exposes, which its own include and exclude lists decide.',
		seed: [],
		defaultHint: 'Default: all of them'
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
			'Image output needs S3 storage on the workspace, ignores tools, and works with OpenAI, Google AI and the OpenRouter gemini-image-preview model. Decision output answers typed questions about a state with probabilities, and runs on a TypeSafe resource.',
		implicit: 'text',
		defaultHint: 'Default: text',
		outputTypes: EVERY_OUTPUT_TYPE
	},
	{
		key: 'questions',
		group: 'output',
		label: 'Questions',
		tooltip:
			'Each question by name, as { type, instructions, criteria }. choice picks one option: criteria maps each option to what it means. score places the state on a scale: criteria lists 2 to 10 levels in order. noul is yes or no: criteria can describe true and false. The result holds each answer with its probabilities under output.',
		core: true,
		outputTypes: ['decision']
	},
	{
		key: 'output_schema',
		group: 'output',
		label: 'Output schema',
		tooltip: 'A JSON schema the answer has to follow.',
		defaultHint: 'Default: none',
		outputTypes: ['text']
	},
	{
		key: 'streaming',
		group: 'output',
		label: 'Stream the response',
		tooltip: 'Send the answer back as it is generated, rather than once it is complete.',
		implicit: true,
		defaultHint: 'Default: on',
		outputTypes: ['text']
	}
]

export const AGENT_FIELD_BY_KEY: Record<string, AgentFieldSpec> = Object.fromEntries(
	AGENT_FIELDS.map((f) => [f.key, f])
)

/**
 * Fields the agent editor's test form has to offer whatever the agent holds, rather than only the
 * ones a step wrote: a saved agent stores no flow-local input, so its own form cannot open a row
 * for one and the test form is the only place left to supply it.
 *
 * `enabled_tools` stays out because narrowing a roster belongs to the step that reuses the agent,
 * not to a run of the agent itself.
 */
export const AGENT_EDITOR_RUN_INPUTS: readonly string[] = ['user_attachments']

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
	schemaProperties: Record<string, any> | undefined,
	/** A linked step's comes from its agent, since the step holds no `output_type` of its own. */
	outputType: AgentOutputType = agentOutputType(
		args?.output_type?.type === 'static' ? args.output_type.value : undefined
	)
): Set<string> {
	const visible = new Set<string>()
	for (const spec of AGENT_FIELDS) {
		if (!agentFieldAppliesTo(spec, schemaProperties) || !agentFieldServes(spec, outputType)) {
			continue
		}
		if (agentFieldIsSet(spec, args?.[spec.key])) visible.add(spec.key)
	}
	return visible
}
