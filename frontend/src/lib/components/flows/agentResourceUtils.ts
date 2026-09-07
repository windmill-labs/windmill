import { deepEqual } from 'fast-equals'
import type { InputTransform } from '$lib/gen'
import { AGENT_FIELDS } from './agentFormFields'

// The brain fields stored flat in an `ai_agent` resource value. The flow-local inputs
// (user_message/user_attachments) are intentionally excluded — they are supplied per-flow.
export const AGENT_BRAIN_KEYS = [
	'provider',
	'output_type',
	'system_prompt',
	'streaming',
	'memory',
	'output_schema',
	'max_completion_tokens',
	'temperature',
	'max_iterations'
] as const

export const AGENT_FLOW_LOCAL_KEYS = ['user_message', 'user_attachments'] as const

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
 * agent: the brain fields come from the resource, so only user_message/user_attachments stay editable.
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

/** Flatten a saved agent's brain config into human-readable label/value rows for a read-only
 * display on a linked step. Only set fields are returned, in the canonical brain-key order. */
export function summarizeAgentBrain(
	config: AIAgentConfig | undefined
): { label: string; value: string }[] {
	const rows: { label: string; value: string }[] = []
	for (const key of AGENT_BRAIN_KEYS) {
		const v = (config as any)?.[key]
		if (v === undefined || v === null || v === '') continue
		let value: string
		if (key === 'provider') {
			value = [v.kind, v.model].filter(Boolean).join(' · ') || 'configured'
		} else if (key === 'memory') {
			// Memory configs are serialized with a `kind` tag (serde tag = "kind").
			value = typeof v === 'object' ? (v.kind ?? v.type ?? 'configured') : String(v)
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
