import type { FlowModule, OpenFlow, RawScript } from '$lib/gen'
import { forEachFlowModule } from '$lib/components/flows/dfs'
import { findModuleInFlow } from '$lib/components/flows/flowTree'
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

export function updateRawScriptModuleContent(
	flow: FlowLike,
	id: string,
	code: string
): (FlowModule & { value: RawScript }) | undefined {
	const module = findModuleInFlow(flow.value, id)
	if (!module || module.value.type !== 'rawscript') {
		return undefined
	}

	const rawScriptModule = module as FlowModule & { value: RawScript }
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
		flow.value.modules = restoreFlowModules(
			modules,
			inlineScriptSession,
			emptyInlineScriptModuleIds
		)
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
