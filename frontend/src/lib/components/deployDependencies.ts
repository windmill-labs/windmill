import type { Kind } from '$lib/utils_deployable'

export type Dependency = { kind: Kind; path: string }

export function stripResourcePrefix(ref: string): string {
	return ref.replace(/^\$res:/, '').replace(/^res:\/\//, '')
}

/** Workspace objects referenced by an input_transforms map through a static `$res:`/`$var:` value. */
export function collectTransformRefs(transforms: unknown): Dependency[] {
	const result: Dependency[] = []
	for (const t of Object.values((transforms ?? {}) as Record<string, any>)) {
		if (t?.type == 'static' && typeof t.value == 'string') {
			if (t.value.startsWith('$res:')) {
				result.push({ kind: 'resource', path: t.value.substring(5) })
			} else if (t.value.startsWith('$var:')) {
				result.push({ kind: 'variable', path: t.value.substring(5) })
			}
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

/** A linked step's own dependencies: the saved agent, plus the flow-local `tool_inputs` overrides.
 * An override replaces the resource tool's default at runtime, so the value this flow actually uses
 * must follow the deploy — the saved default alone is not enough. */
export function aiAgentModuleDependencies(value: unknown): Dependency[] {
	const v = value as any
	const result: Dependency[] = []
	if (typeof v?.agent == 'string' && v.agent) {
		result.push({ kind: 'resource', path: stripResourcePrefix(v.agent) })
	}
	for (const overrides of Object.values((v?.tool_inputs ?? {}) as Record<string, unknown>)) {
		result.push(...collectTransformRefs(overrides))
	}
	return result
}
