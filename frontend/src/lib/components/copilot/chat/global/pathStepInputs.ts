import type { FlowModule } from '$lib/gen'

/** A flow step that calls a workspace script or flow by path. */
export type PathStep = { id: string; kind: 'script' | 'flow'; path: string; inputs: string[] }

export type MissingStepInputs = { id: string; path: string; missing: string[] }

type FlowModules = {
	modules?: FlowModule[]
	preprocessor_module?: FlowModule | null
	failure_module?: FlowModule | null
}

/**
 * Every path step of a flow, nested ones included. Hub scripts are not workspace items, and a
 * step pinned to a version is not checked against the latest schema. The preprocessor and any
 * module with `pass_flow_input_directly` are left out: the worker hands them the flow's own args
 * and ignores their input_transforms. The walk also reads stored flows, which nothing
 * validated, so it tolerates missing fields.
 */
export function collectPathSteps(flow: FlowModules | undefined): PathStep[] {
	const steps: PathStep[] = []
	const visit = (modules: FlowModule[] | undefined | null) => {
		for (const m of modules ?? []) {
			const v = m?.value
			if (!v) continue
			if (v.type === 'script' || v.type === 'flow') {
				if (typeof v.path !== 'string' || v.path.startsWith('hub/')) continue
				if (v.type === 'script' && v.hash) continue
				if (passesFlowInput(m)) continue
				steps.push({
					id: m.id,
					kind: v.type,
					path: v.path,
					inputs: Object.keys(v.input_transforms ?? {})
				})
			} else if (v.type === 'forloopflow' || v.type === 'whileloopflow') {
				visit(v.modules)
			} else if (v.type === 'branchone') {
				for (const b of v.branches ?? []) visit(b?.modules)
				visit(v.default)
			} else if (v.type === 'branchall') {
				for (const b of v.branches ?? []) visit(b?.modules)
			}
		}
	}
	visit(flow?.modules)
	if (flow?.failure_module) visit([flow.failure_module])
	return steps
}

// The generated client does not carry this flag. Only the module's copy counts: the worker
// never reads the one a script or flow value can also hold.
const passesFlowInput = (m: FlowModule) =>
	(m as { pass_flow_input_directly?: boolean }).pass_flow_input_directly === true

/** Required inputs without a default, which a step calling the item has to pass. */
export function requiredInputs(schema: unknown): string[] {
	const s = schema as { required?: string[]; properties?: Record<string, { default?: unknown }> }
	return (s?.required ?? []).filter((name) => s.properties?.[name]?.default === undefined)
}

/**
 * The steps of `next` that leave out a required input of the item they call. A gap the
 * previous version of the flow already had is not reported, so an edit is never refused
 * over a step it did not touch. An item whose schema is unknown is not checked.
 */
export function missingStepInputs(
	next: PathStep[],
	previous: PathStep[],
	schemaOf: (step: PathStep) => unknown
): MissingStepInputs[] {
	const gaps = (step: PathStep) => {
		const schema = schemaOf(step)
		return schema === undefined
			? []
			: requiredInputs(schema).filter((n) => !step.inputs.includes(n))
	}
	const known = new Set(
		previous.flatMap((step) => gaps(step).map((name) => `${step.id}\0${step.path}\0${name}`))
	)
	return next
		.map((step) => ({
			id: step.id,
			path: step.path,
			missing: gaps(step).filter((name) => !known.has(`${step.id}\0${step.path}\0${name}`))
		}))
		.filter((step) => step.missing.length > 0)
}
