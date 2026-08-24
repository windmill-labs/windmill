import type { FlowModule } from '$lib/gen'

/** Visit every AI agent module of a module tree, including agents nested inside agent tools.
 * Takes `unknown` so it also runs on model-produced JSON that has not been schema-checked yet.
 *
 * Lives in its own leaf module (types only) so validation code can traverse a flow without
 * pulling in the editor-side agent tool helpers. */
export function forEachAiAgentModule(
	modules: unknown,
	cb: (mod: FlowModule, value: Record<string, any>) => void
): void {
	const visit = (mods: unknown) => {
		if (!Array.isArray(mods)) return
		for (const mod of mods) {
			const v = (mod as FlowModule | undefined)?.value as Record<string, any> | undefined
			if (!v) continue
			if (v.type === 'aiagent') {
				cb(mod as FlowModule, v)
				visit(v.tools)
			} else if (v.type === 'forloopflow' || v.type === 'whileloopflow') {
				visit(v.modules)
			} else if (v.type === 'branchone' || v.type === 'branchall') {
				if (v.type === 'branchone') visit(v.default)
				// Model-produced JSON reaches this before any schema check, so a `branches` that
				// is not an array must fall through to the schema error, not throw here.
				if (Array.isArray(v.branches)) {
					for (const b of v.branches) visit(b?.modules)
				}
			}
		}
	}
	visit(modules)
}
