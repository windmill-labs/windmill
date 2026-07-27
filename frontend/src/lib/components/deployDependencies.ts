import type { Kind } from '$lib/utils_deployable'

export type Dependency = { kind: Kind; path: string }

export function stripResourcePrefix(ref: string): string {
	return ref.replace(/^\$res:/, '').replace(/^res:\/\//, '')
}

/** Workspace objects referenced by an input_transforms map through a static `$res:`/`$var:` value.
 * The value is walked, not just string-matched: an AI agent's provider is an object holding its
 * credential under `resource`, so a top-level string check alone misses it. */
export function collectTransformRefs(transforms: unknown): Dependency[] {
	const result: Dependency[] = []
	const walk = (v: unknown) => {
		if (typeof v == 'string') {
			if (v.startsWith('$res:')) {
				result.push({ kind: 'resource', path: v.substring(5) })
			} else if (v.startsWith('$var:')) {
				result.push({ kind: 'variable', path: v.substring(5) })
			} else if (v.startsWith('$jsonvar:')) {
				result.push({ kind: 'variable', path: v.substring(9) })
			}
		} else if (typeof v == 'object' && v != null) {
			for (const inner of Object.values(v)) walk(inner)
		}
	}
	for (const t of Object.values((transforms ?? {}) as Record<string, any>)) {
		if (t?.type == 'static') {
			walk(t.value)
		}
	}
	return result
}

/** A saved agent bundles its tools, which reference workspace objects by bare path rather than
 * `$res:` — invisible to the generic value walk. A nested *linked* agent is queued as a resource and
 * recursed into; a nested *inline* one carries its tools here, so recurse into them too. */
export function agentResourceDependencies(value: unknown): Dependency[] {
	const result: Dependency[] = []
	for (const tool of ((value as any)?.tools ?? []) as any[]) {
		const v = tool?.value
		if (typeof v !== 'object' || v == null) continue
		result.push(...collectTransformRefs(v.input_transforms))
		if (typeof v.resource_path == 'string' && v.resource_path) {
			result.push({ kind: 'resource', path: stripResourcePrefix(v.resource_path) })
		} else if (v.type == 'script' && typeof v.path == 'string' && v.path) {
			if (!v.path.startsWith('hub/')) {
				result.push({ kind: 'script', path: v.path })
			}
		} else if (v.type == 'flow' && typeof v.path == 'string' && v.path) {
			result.push({ kind: 'flow', path: v.path })
		} else if (v.type == 'aiagent') {
			if (typeof v.agent == 'string' && v.agent) {
				result.push({ kind: 'resource', path: stripResourcePrefix(v.agent) })
			} else {
				result.push(...agentResourceDependencies(v))
			}
		}
	}
	return result
}

/** An AI agent step's own dependencies. Linked: the saved agent, plus the flow-local `tool_inputs`
 * overrides — an override replaces the resource tool's default at runtime, so the value this flow
 * actually uses must follow the deploy. Standalone: its inline tools, which the flow module walk
 * only partly reaches (MCP and websearch tools are not flow modules). */
export function aiAgentModuleDependencies(value: unknown): Dependency[] {
	const v = value as any
	const result: Dependency[] = collectTransformRefs(v?.input_transforms)
	if (typeof v?.agent == 'string' && v.agent) {
		result.push({ kind: 'resource', path: stripResourcePrefix(v.agent) })
	} else {
		result.push(...agentResourceDependencies(v))
	}
	for (const overrides of Object.values((v?.tool_inputs ?? {}) as Record<string, unknown>)) {
		result.push(...collectTransformRefs(overrides))
	}
	return result
}
