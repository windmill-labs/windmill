import type { InputTransform } from '$lib/gen'

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
		if (t && t.type === 'static' && t.value !== undefined) {
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

const AGENT_BRAIN_LABELS: Record<string, string> = {
	provider: 'Model',
	output_type: 'Output type',
	system_prompt: 'System prompt',
	streaming: 'Streaming',
	memory: 'Memory',
	output_schema: 'Output schema',
	max_completion_tokens: 'Max tokens',
	temperature: 'Temperature',
	max_iterations: 'Max iterations'
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
			value = typeof v === 'object' ? (v.type ?? 'configured') : String(v)
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
