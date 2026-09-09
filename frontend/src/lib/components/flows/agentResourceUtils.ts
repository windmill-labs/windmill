import { deepEqual } from 'fast-equals'
import type { InputTransform } from '$lib/gen'
import { AGENT_FIELDS } from './agentFormFields'

// The brain fields stored flat in an `ai_agent` resource value. The flow-local keys below are
// intentionally excluded — they are supplied per-flow.
export const AGENT_BRAIN_KEYS = [
	'provider',
	'output_type',
	'system_prompt',
	'streaming',
	'output_schema',
	'max_completion_tokens',
	'temperature',
	'max_iterations'
] as const

/**
 * The inputs a step supplies for itself, whether or not it is linked to a saved agent.
 *
 * `memory` is one of them because conversation history belongs to the flow having the
 * conversation, not to an agent reused across flows: it is identified by a `memory_id` minted per
 * step on flow save, and two flows linking one agent must not answer from each other's history.
 * `enabled_tools` likewise narrows one use of an agent, leaving the roster it narrows alone.
 */
export const AGENT_FLOW_LOCAL_KEYS = [
	'user_message',
	'user_attachments',
	'memory',
	'enabled_tools'
] as const

export type AgentTool = Record<string, any>

/**
 * Why this resource cannot be edited as an agent, if it cannot. A path with no deployed row answers
 * with its own draft, whose type the generic resource editor never writes, so an unknown type is
 * refused rather than assumed: deploying it would create that path as an agent.
 */
export function agentEditorRefusal(
	path: string,
	resourceType: string | undefined
): string | undefined {
	if (resourceType === 'ai_agent') return undefined
	return resourceType
		? `${path} is a ${resourceType} resource, not an agent.`
		: `${path} has no deployed agent to edit.`
}

/** Brain keys whose step transform is non-static and would be dropped by a save-as-agent snapshot. */
export function nonStaticBrainKeys(
	inputTransforms: Record<string, InputTransform> | undefined
): string[] {
	return AGENT_BRAIN_KEYS.filter((key) => {
		const t = inputTransforms?.[key] as any
		return t && t.type !== 'static'
	})
}

/**
 * Keep only the flow-local inputs on a step's input_transforms. Used when linking: the brain comes
 * from the resource, so the step must not carry stale `provider`/`system_prompt`/… transforms — at
 * runtime they'd still be resolved (an unnecessary, possibly failing provider `$res:` fetch) yet
 * never used, since the linked branch takes the brain from the resource.
 */
export function flowLocalInputs(
	inputTransforms: Record<string, InputTransform> | undefined
): Record<string, InputTransform> {
	const out: Record<string, InputTransform> = {}
	for (const key of AGENT_FLOW_LOCAL_KEYS) {
		if (inputTransforms?.[key]) {
			out[key] = inputTransforms[key]
		}
	}
	return out
}

/**
 * Whether a transform holds something a run would use. A field the form has not been filled in for
 * is seeded as `{"type":"static"}` — and comes back from the API with an explicit null — which a
 * run cannot tell from an absent key.
 */
function transformIsSet(transform: InputTransform | undefined): boolean {
	if (!transform) return false
	const t = transform as any
	// Same reading of "set" as `agentFieldIsSet`: an emptied expression is a field being written,
	// not one holding a value.
	if (t.type === 'javascript') return Boolean(t.expr)
	if (t.type !== 'static') return true
	return t.value !== undefined && t.value !== null
}

/**
 * `flowLocalInputs`, minus the fields the step is holding a placeholder for. Use it wherever the
 * step's inputs are laid over a value the agent supplied: an unfilled field must not shadow what it
 * inherits, which is the rule the worker follows too (`ai_executor.rs` writes the step's `memory`
 * over the resource's only when it is not null).
 */
export function overridingFlowLocalInputs(
	inputTransforms: Record<string, InputTransform> | undefined
): Record<string, InputTransform> {
	const out: Record<string, InputTransform> = {}
	for (const key of AGENT_FLOW_LOCAL_KEYS) {
		const transform = inputTransforms?.[key]
		if (transformIsSet(transform)) {
			out[key] = transform!
		}
	}
	return out
}

/**
 * The host-flow overrides to store on a linked step for one tool: the subset of the tool's edited
 * input_transforms that diverges from the resource tool's own transforms. Storing only the diff (not
 * the full merged map) keeps unchanged inputs inheriting from the resource, makes merely opening a
 * tool a no-op (its inputs still equal base ∪ overrides), and lets reverting an edit persist.
 */
export function toolInputOverrides(
	inputs: Record<string, InputTransform> | undefined,
	base: Record<string, InputTransform> | undefined
): Record<string, InputTransform> {
	const overrides: Record<string, InputTransform> = {}
	for (const [key, value] of Object.entries(inputs ?? {})) {
		if (!deepEqual(value, base?.[key])) {
			overrides[key] = value
		}
	}
	return overrides
}

export interface AIAgentConfig {
	provider?: unknown
	output_type?: string
	system_prompt?: string
	streaming?: boolean
	/** Only on an agent saved while memory was still a brain field. Nothing writes it any more, and
	 *  the worker honours it only while the step using it sets no memory of its own. */
	memory?: unknown
	output_schema?: unknown
	max_completion_tokens?: number
	temperature?: number
	max_iterations?: number
	tools?: AgentTool[]
}

/** Extract the static brain values from a step's input_transforms into a flat agent config. */
export function inputTransformsToAgentConfig(
	inputTransforms: Record<string, InputTransform> | undefined,
	tools: AgentTool[] | undefined
): AIAgentConfig {
	const config: AIAgentConfig = { tools: tools ?? [] }
	for (const key of AGENT_BRAIN_KEYS) {
		const t = inputTransforms?.[key] as any
		// `null` as well as `undefined`: a placeholder transform is `{"type":"static"}`, and it comes
		// back from the API — and from a schema backfill — with an explicit null. Writing it through
		// would put `memory: null` in the saved agent and show as a change against a config that
		// simply omits the key.
		if (t && t.type === 'static' && t.value !== undefined && t.value !== null) {
			;(config as any)[key] = t.value
		}
	}
	return config
}

/**
 * Reduce the AI agent schema to only the flow-local inputs. Used when a step is linked to a saved
 * agent: the brain fields come from the resource, so only `AGENT_FLOW_LOCAL_KEYS` stay editable.
 */
export function flowLocalAgentSchema(schema: any): any {
	if (!schema?.properties) {
		return schema
	}
	const properties: Record<string, unknown> = {}
	for (const key of AGENT_FLOW_LOCAL_KEYS) {
		if (schema.properties[key]) {
			properties[key] = schema.properties[key]
		}
	}
	return {
		...schema,
		properties,
		order: (schema.order ?? Object.keys(properties)).filter((k: string) => k in properties),
		required: (schema.required ?? []).filter((k: string) => k in properties)
	}
}

/** Read off the form's own registry, so a linked agent's summary cannot name a field differently
 *  from the form that edits it. */
export const AGENT_BRAIN_LABELS: Record<string, string> = Object.fromEntries(
	AGENT_FIELDS.map((f) => [f.key, f.label])
)

/**
 * Brain keys set to an input transform rather than to a value, which a saved agent's plain JSON
 * cannot hold. Matched on the tag so the whole `InputTransform` union is covered: a payload-key
 * test would miss `{"type":"ai"}`, which carries none.
 */
export function transformValuedBrainKeys(args: Record<string, any> | undefined): string[] {
	return AGENT_BRAIN_KEYS.filter((key) => {
		const v = args?.[key]
		return (
			v !== null &&
			typeof v === 'object' &&
			(v.type === 'javascript' || v.type === 'ai' || v.type === 'static')
		)
	})
}

/**
 * Flatten a saved agent's brain config into human-readable label/value rows for a read-only display
 * on a linked step. Only set fields are returned, in the canonical brain-key order.
 *
 * `memory` is listed after them although it is no longer a brain field, because an agent saved
 * while it was one still carries a config the worker honours. Nothing writes one any more, so the
 * row only ever appears on such an agent — and where it does, the step's own Memory field would
 * otherwise be the only thing on screen saying anything about memory, while reading "off".
 */
export function summarizeAgentBrain(
	config: AIAgentConfig | undefined
): { label: string; value: string }[] {
	const rows: { label: string; value: string }[] = []
	for (const key of [...AGENT_BRAIN_KEYS, 'memory']) {
		const v = (config as any)?.[key]
		if (v === undefined || v === null || v === '') continue
		let value: string
		if (key === 'provider') {
			value = [v.kind, v.model].filter(Boolean).join(' · ') || 'configured'
		} else if (key === 'memory') {
			// Memory configs are serialized with a `kind` tag (serde tag = "kind").
			value = typeof v === 'object' ? (v.kind ?? 'configured') : String(v)
		} else if (key === 'output_schema') {
			value = 'configured'
		} else if (typeof v === 'boolean') {
			value = v ? 'on' : 'off'
		} else if (typeof v === 'object') {
			value = JSON.stringify(v)
		} else {
			value = String(v)
		}
		rows.push({ label: AGENT_BRAIN_LABELS[key] ?? key, value })
	}
	return rows
}

/** Inverse: wrap brain config values as static input_transforms (used when unlinking a step). */
export function agentConfigToInputTransforms(
	config: AIAgentConfig
): Record<string, InputTransform> {
	const it: Record<string, InputTransform> = {}
	for (const key of AGENT_BRAIN_KEYS) {
		const v = (config as any)[key]
		if (v !== undefined) {
			it[key] = { type: 'static', value: v } as InputTransform
		}
	}
	return it
}
