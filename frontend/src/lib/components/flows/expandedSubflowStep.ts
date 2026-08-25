import type { FlowModule } from '$lib/gen'
import { findStepPath, parseExpandedSubflowId } from '$lib/components/restartFromStepPath'

export type ResolvedExpandedSubflowStep = {
	/** The flow the selected step lives in, i.e. the innermost subflow crossed. */
	containingFlowPath: string
	/** Path of each subflow crossed from the edited flow down to the containing one. */
	pathChain: string[]
	/** The selected step, or `undefined` when the id points at something that isn't a
	 * module of the containing flow (an AI agent tool node, a stale selection...). */
	module: FlowModule | undefined
}

/**
 * Graph node id of the expansion an inline-expanded subflow node sits in, i.e. the key
 * whose modules hold the step it stands for: `subflow:a:b` → `a`, `subflow:a:b:c` →
 * `subflow:a:b`. `undefined` for a top-level expansion, whose step belongs to the edited
 * flow itself. Note that the last segment of `subflowSteps` is the parent's own step, so
 * every segment stays in the parent key.
 */
export function expandedSubflowParentId(nodeId: string): string | undefined {
	const steps = parseExpandedSubflowId(nodeId)?.subflowSteps
	if (!steps) {
		return undefined
	}
	return steps.length > 1 ? 'subflow:' + steps.join(':') : steps[0]
}

/**
 * Resolves a `subflow:A:B:leaf` graph node id (an inline-expanded subflow step) to the
 * step it stands for, following each `Flow{path}` boundary through the flows it points
 * at. `loadFlowModules` fetches a flow's modules by path.
 *
 * Returns `undefined` when `id` is not an expanded-subflow node or a boundary can't be
 * followed; a resolution whose `module` is undefined still carries the containing flow,
 * which is enough to open it in the editor.
 */
export async function resolveExpandedSubflowStep(
	id: string,
	rootModules: FlowModule[],
	loadFlowModules: (path: string) => Promise<FlowModule[]>
): Promise<ResolvedExpandedSubflowStep | undefined> {
	const parsed = parseExpandedSubflowId(id)
	if (!parsed) {
		return undefined
	}
	let modules = rootModules
	const pathChain: string[] = []
	for (const stepId of parsed.subflowSteps) {
		const value = findStepPath(modules, stepId)?.target.value
		if (value?.type !== 'flow') {
			return undefined
		}
		pathChain.push(value.path)
		modules = await loadFlowModules(value.path)
	}
	return {
		containingFlowPath: pathChain[pathChain.length - 1],
		pathChain,
		module: findStepPath(modules, parsed.leaf)?.target
	}
}
