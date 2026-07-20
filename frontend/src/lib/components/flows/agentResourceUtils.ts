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

export type AgentAssertion =
	| { kind: 'contains'; value: string; case_sensitive?: boolean }
	| { kind: 'not_contains'; value: string; case_sensitive?: boolean }
	| { kind: 'regex'; pattern: string }
	| { kind: 'json_path_equals'; path: string; value: unknown }
	| { kind: 'output_schema_valid' }

export interface AgentEvalCase {
	id: string
	name?: string
	input: { user_message?: string; user_attachments?: unknown[] }
	judge_checklist?: string[]
	assertions?: AgentAssertion[]
}

export interface AgentEvalSuite {
	cases: AgentEvalCase[]
	judge?: unknown
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
	evals?: AgentEvalSuite
}

/** Extract the static brain values from a step's input_transforms into a flat agent config. */
export function inputTransformsToAgentConfig(
	inputTransforms: Record<string, InputTransform> | undefined,
	tools: AgentTool[] | undefined,
	evals?: AgentEvalSuite
): AIAgentConfig {
	const config: AIAgentConfig = { tools: tools ?? [], evals: evals ?? { cases: [] } }
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

export interface HostBoundTool {
	toolId: string
	label: string
	/** Inputs the agent author wired to context (javascript transforms); these dangle when the
	 * agent is linked into another flow and must be rebound to the host flow. */
	keys: { key: string; resourceExpr: string }[]
}

/**
 * From a linked agent's inherited tools, the per-tool inputs that need host-flow rebinding: those
 * whose saved transform is a `javascript` expr (it references the authoring flow's context, which
 * is meaningless in a different flow). Static/AI inputs are left as-is. Only FlowModule tools carry
 * input transforms; MCP/websearch tools have none.
 */
export function hostBoundToolInputs(tools: AgentTool[] | undefined): HostBoundTool[] {
	const out: HostBoundTool[] = []
	for (const tool of tools ?? []) {
		const its = tool?.value?.input_transforms as Record<string, any> | undefined
		if (!its) continue
		const keys = Object.entries(its)
			.filter(([, t]) => t?.type === 'javascript')
			.map(([key, t]) => ({ key, resourceExpr: (t as any)?.expr ?? '' }))
		if (keys.length > 0) {
			out.push({ toolId: tool.id, label: tool.summary || tool.id, keys })
		}
	}
	return out
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
