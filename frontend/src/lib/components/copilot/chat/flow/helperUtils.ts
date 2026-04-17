import type { FlowModule, OpenFlow, RawScript } from '$lib/gen'
import { forEachFlowModule } from '$lib/components/flows/dfs'
import { dfs } from '$lib/components/flows/previousResults'
import { SPECIAL_MODULE_IDS } from '../shared'
import type { InlineScriptSession } from './inlineScriptsUtils'

type FlowLike = Pick<OpenFlow, 'value'> & {
	schema?: Record<string, any>
}

export interface FlowJsonUpdate {
	modules?: FlowModule[]
	schema?: Record<string, any> | null
	preprocessorModule?: FlowModule | null
	failureModule?: FlowModule | null
}

export interface FlowJsonUpdateResult {
	emptyInlineScriptModuleIds: string[]
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

export function getMutableRawScriptModuleById(
	flow: FlowLike | undefined,
	id: string
): (FlowModule & { value: RawScript }) | undefined {
	if (!flow) {
		return undefined
	}

	if (flow.value.preprocessor_module?.id === id && flow.value.preprocessor_module.value.type === 'rawscript') {
		return flow.value.preprocessor_module as FlowModule & { value: RawScript }
	}

	if (flow.value.failure_module?.id === id && flow.value.failure_module.value.type === 'rawscript') {
		return flow.value.failure_module as FlowModule & { value: RawScript }
	}

	let matchedModule: (FlowModule & { value: RawScript }) | undefined
	forEachFlowModule(flow.value.modules, (module) => {
		if (!matchedModule && module.id === id && module.value.type === 'rawscript') {
			matchedModule = module as FlowModule & { value: RawScript }
		}
	})

	return matchedModule
}

export function updateRawScriptModuleContent(
	flow: FlowLike,
	id: string,
	code: string
): (FlowModule & { value: RawScript }) | undefined {
	const rawScriptModule = getMutableRawScriptModuleById(flow, id)
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
): FlowJsonUpdateResult {
	const emptyInlineScriptModuleIds = new Set<string>()

	if (modules !== undefined) {
		flow.value.modules = restoreFlowModules(modules, inlineScriptSession, emptyInlineScriptModuleIds)
	}

	if (schema !== undefined) {
		flow.schema = schema ?? undefined
	}

	if (preprocessorModule !== undefined) {
		flow.value.preprocessor_module =
			preprocessorModule === null
				? undefined
				: restoreFlowModule(preprocessorModule, inlineScriptSession, emptyInlineScriptModuleIds)
	}

	if (failureModule !== undefined) {
		flow.value.failure_module =
			failureModule === null
				? undefined
				: restoreFlowModule(failureModule, inlineScriptSession, emptyInlineScriptModuleIds)
	}

	return {
		emptyInlineScriptModuleIds: Array.from(emptyInlineScriptModuleIds)
	}
}

function restoreFlowModules(
	modules: FlowModule[],
	inlineScriptSession: InlineScriptSession,
	emptyInlineScriptModuleIds: Set<string>
): FlowModule[] {
	const restoredModules = inlineScriptSession.restoreInlineScriptReferences(modules)
	replaceNewInlineScriptRefsWithEmptyCode(restoredModules, emptyInlineScriptModuleIds)
	assertResolvedInlineScripts(restoredModules, inlineScriptSession)
	return restoredModules
}

function restoreFlowModule(
	module: FlowModule,
	inlineScriptSession: InlineScriptSession,
	emptyInlineScriptModuleIds: Set<string>
): FlowModule {
	const [restoredModule] = inlineScriptSession.restoreInlineScriptReferences([module])
	replaceNewInlineScriptRefsWithEmptyCode([restoredModule], emptyInlineScriptModuleIds)
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

function replaceNewInlineScriptRefsWithEmptyCode(
	modules: FlowModule[],
	emptyInlineScriptModuleIds: Set<string>
): void {
	function replaceInlineScriptRefWithEmptyCode(ownerId: string, content: string): string {
		const match = content.match(/^inline_script\.(.+)$/)
		if (!match || match[1] !== ownerId) {
			return content
		}

		emptyInlineScriptModuleIds.add(ownerId)
		return ''
	}

	forEachFlowModule(modules, (module) => {
		if (module.value.type === 'rawscript' && module.value.content) {
			module.value.content = replaceInlineScriptRefWithEmptyCode(module.id, module.value.content)
		}
	})
}
