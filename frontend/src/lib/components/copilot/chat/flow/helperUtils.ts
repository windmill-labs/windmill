import type { FlowModule, OpenFlow, RawScript } from '$lib/gen'
import { dfs } from '$lib/components/flows/previousResults'
import { SPECIAL_MODULE_IDS } from '../shared'
import type { InlineScriptSession } from './inlineScriptsUtils'

type FlowLike = Pick<OpenFlow, 'value'> & {
	schema?: Record<string, any>
}

export interface FlowJsonUpdate {
	modules?: FlowModule[]
	schema?: Record<string, any>
	preprocessorModule?: FlowModule | null
	failureModule?: FlowModule | null
}

export function getFlowModuleById(flow: FlowLike | undefined, id: string): FlowModule | undefined {
	if (!flow) {
		return undefined
	}

	if (id === SPECIAL_MODULE_IDS.PREPROCESSOR) {
		return flow.value.preprocessor_module
	}

	if (id === SPECIAL_MODULE_IDS.FAILURE) {
		return flow.value.failure_module
	}

	return dfs(id, flow as OpenFlow, false)[0]
}

export function getRawScriptModuleById(
	flow: FlowLike | undefined,
	id: string
): (FlowModule & { value: RawScript }) | undefined {
	const module = getFlowModuleById(flow, id)
	if (!module || module.value.type !== 'rawscript') {
		return undefined
	}

	return module as FlowModule & { value: RawScript }
}

export function updateRawScriptModuleContent(
	flow: FlowLike,
	id: string,
	code: string
): (FlowModule & { value: RawScript }) | undefined {
	const rawScriptModule = getRawScriptModuleById(flow, id)
	if (!rawScriptModule) {
		return undefined
	}

	rawScriptModule.value.content = code
	return rawScriptModule
}

export function applyFlowJsonUpdate(
	flow: FlowLike,
	inlineScriptSession: InlineScriptSession,
	{ modules, schema, preprocessorModule, failureModule }: FlowJsonUpdate
): void {
	if (modules !== undefined) {
		flow.value.modules = restoreFlowModules(modules, inlineScriptSession)
	}

	if (schema !== undefined) {
		flow.schema = schema
	}

	if (preprocessorModule !== undefined) {
		flow.value.preprocessor_module =
			preprocessorModule === null
				? undefined
				: restoreFlowModule(preprocessorModule, inlineScriptSession)
	}

	if (failureModule !== undefined) {
		flow.value.failure_module =
			failureModule === null ? undefined : restoreFlowModule(failureModule, inlineScriptSession)
	}
}

function restoreFlowModules(
	modules: FlowModule[],
	inlineScriptSession: InlineScriptSession
): FlowModule[] {
	const restoredModules = inlineScriptSession.restoreInlineScriptReferences(modules)
	assertResolvedInlineScripts(restoredModules, inlineScriptSession)
	return restoredModules
}

function restoreFlowModule(
	module: FlowModule,
	inlineScriptSession: InlineScriptSession
): FlowModule {
	const [restoredModule] = inlineScriptSession.restoreInlineScriptReferences([module])
	assertResolvedInlineScripts([restoredModule], inlineScriptSession)
	return restoredModule
}

function assertResolvedInlineScripts(
	modules: FlowModule[],
	inlineScriptSession: InlineScriptSession
): void {
	const unresolvedRefs = inlineScriptSession.findUnresolvedInlineScriptRefs(modules)
	if (unresolvedRefs.length > 0) {
		throw new Error(`Unresolved inline script references: ${unresolvedRefs.join(', ')}`)
	}
}
