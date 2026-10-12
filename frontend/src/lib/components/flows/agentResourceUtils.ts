import { deepEqual } from 'fast-equals'
import type { InputTransform } from '$lib/gen'
import {
	AGENT_FIELDS,
	AGENT_OVERRIDE_KEYS,
	describeMemoryPolicy,
	type AgentOverrideKey
} from './agentFormFields'

// The brain fields stored flat in an `ai_agent` resource value. The flow-local inputs below are
// intentionally excluded — they are supplied per-flow.
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

/**
 * The inputs a step supplies for itself, whether or not it is linked to a saved agent.
 *
 * `enabled_tools` is one of them because it narrows one use of an agent rather than the agent:
 * saving it into the resource would impose one flow's roster on every flow linking it. The history
 * inputs are too, since which conversation a step reads belongs to the flow using the agent.
 */
export const AGENT_FLOW_LOCAL_KEYS = [
	'user_message',
	'user_attachments',
	'enabled_tools',
	'memory_id',
	'previous_messages'
] as const

/** Every input a linked step holds itself: the flow-local ones, and its overrides of what the
 *  agent saved inside `provider`. */
export const AGENT_LINKED_STEP_KEYS: readonly string[] = [
	...AGENT_FLOW_LOCAL_KEYS,
	...AGENT_OVERRIDE_KEYS
]

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
 * Keep only the inputs a linked step holds on a step's input_transforms. Used when linking: the brain comes
 * from the resource, so the step must not carry stale `provider`/`system_prompt`/… transforms — at
 * runtime they'd still be resolved (an unnecessary, possibly failing provider `$res:` fetch) yet
 * never used, since the linked branch takes the brain from the resource.
 */
export function flowLocalInputs(
	inputTransforms: Record<string, InputTransform> | undefined
): Record<string, InputTransform> {
	const out: Record<string, InputTransform> = {}
	for (const key of AGENT_LINKED_STEP_KEYS) {
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

/**
 * The schema of a step linked to a saved agent, in a flow: its flow-local inputs, plus the saved
 * settings it may override.
 */
export function linkedAgentStepSchema(schema: any): any {
	const local = flowLocalAgentSchema(schema)
	if (!local?.properties) {
		return local
	}
	return {
		...local,
		properties: {
			...local.properties,
			model: {
				type: 'string',
				description: "Replaces the model saved on the agent for this step's runs."
			},
			reasoning_effort: {
				type: 'string',
				description: "Replaces the reasoning effort saved on the agent for this step's runs."
			}
		},
		order: [...local.order, ...AGENT_OVERRIDE_KEYS]
	}
}

/**
 * The `provider` of a step unlinked from its agent: the saved one with the step's overrides folded
 * in, since an unlinked step sets model and effort nowhere else. Mirrors the worker: an empty
 * override keeps the saved value, and another model drops the saved effort.
 */
export function providerWithOverrides(
	provider: any,
	overrides: Partial<Record<AgentOverrideKey, InputTransform>> | undefined
): InputTransform | undefined {
	if (!provider || typeof provider !== 'object') return undefined
	const set = (key: AgentOverrideKey): any => {
		const t = overrides?.[key] as any
		if (t?.type === 'javascript' && t.expr) return t
		if (t?.type === 'static' && typeof t.value === 'string' && t.value.trim()) return t
		return undefined
	}
	const model = set('model')
	const effort = set('reasoning_effort')
	const folded = { ...provider }
	if (model?.type === 'static' && model.value.trim() !== provider.model && !effort) {
		delete folded.reasoning_effort
	}
	if (model?.type !== 'javascript' && effort?.type !== 'javascript') {
		if (model) folded.model = model.value.trim()
		if (effort) folded.reasoning_effort = effort.value.trim()
		return { type: 'static', value: folded }
	}
	const field = (key: AgentOverrideKey, t: any) => {
		if (!t && folded[key] === undefined) return []
		// The saved effort holds only while the model expression resolves to the saved model.
		if (!t && key === 'reasoning_effort' && model?.type === 'javascript') {
			const savedModel = JSON.stringify(folded.model)
			return [
				`${key}: ((${model.expr}) || ${savedModel}) === ${savedModel} ? ${JSON.stringify(folded[key])} : undefined`
			]
		}
		if (!t) return [`${key}: ${JSON.stringify(folded[key])}`]
		if (t.type === 'static') return [`${key}: ${JSON.stringify(t.value.trim())}`]
		const saved = folded[key] === undefined ? 'undefined' : JSON.stringify(folded[key])
		return [`${key}: (${t.expr}) || ${saved}`]
	}
	const { model: _m, reasoning_effort: _e, ...rest } = folded
	const fields = [
		...Object.entries(rest).map(
			([key, value]) => `${JSON.stringify(key)}: ${JSON.stringify(value)}`
		),
		...field('model', model),
		...field('reasoning_effort', effort)
	]
	return { type: 'javascript', expr: `({ ${fields.join(', ')} })` }
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
			value = [v.kind, v.model, v.reasoning_effort].filter(Boolean).join(' · ') || 'configured'
		} else if (key === 'memory') {
			value = typeof v === 'object' ? describeMemoryPolicy(v) : String(v)
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
