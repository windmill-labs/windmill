import type { FlowModule, OpenFlow, RawScript } from '$lib/gen'
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
): FlowJsonUpdateResult {
	const previousModules = flow.value.modules
	const previousSchema = flow.schema
	const previousPreprocessorModule = flow.value.preprocessor_module
	const previousFailureModule = flow.value.failure_module
	const previousInlineScripts = inlineScriptSession.getAll()
	const emptyInlineScriptModuleIds = new Set<string>()

	try {
		if (modules !== undefined) {
			seedMissingInlineScripts(modules, inlineScriptSession).forEach((id) =>
				emptyInlineScriptModuleIds.add(id)
			)
			flow.value.modules = restoreFlowModules(modules, inlineScriptSession)
		}

		if (schema !== undefined) {
			flow.schema = schema ?? undefined
		}

		if (preprocessorModule !== undefined) {
			flow.value.preprocessor_module =
				preprocessorModule === null
					? undefined
					: restoreFlowModuleWithSeededInlineScripts(
							preprocessorModule,
							inlineScriptSession,
							emptyInlineScriptModuleIds
						)
		}

		if (failureModule !== undefined) {
			flow.value.failure_module =
				failureModule === null
					? undefined
					: restoreFlowModuleWithSeededInlineScripts(
							failureModule,
							inlineScriptSession,
							emptyInlineScriptModuleIds
						)
		}

		return {
			emptyInlineScriptModuleIds: Array.from(emptyInlineScriptModuleIds)
		}
	} catch (error) {
		flow.value.modules = previousModules
		flow.schema = previousSchema
		flow.value.preprocessor_module = previousPreprocessorModule
		flow.value.failure_module = previousFailureModule
		inlineScriptSession.clear()
		for (const [moduleId, content] of Object.entries(previousInlineScripts)) {
			inlineScriptSession.set(moduleId, content)
		}
		throw error
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

function restoreFlowModuleWithSeededInlineScripts(
	module: FlowModule,
	inlineScriptSession: InlineScriptSession,
	emptyInlineScriptModuleIds: Set<string>
): FlowModule {
	seedMissingInlineScripts([module], inlineScriptSession).forEach((id) =>
		emptyInlineScriptModuleIds.add(id)
	)
	return restoreFlowModule(module, inlineScriptSession)
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

function seedMissingInlineScripts(
	modules: FlowModule[],
	inlineScriptSession: InlineScriptSession
): string[] {
	const emptyInlineScriptModuleIds: string[] = []

	function ensureEmptyInlineScript(moduleId: string, inlineScriptRefId: string) {
		if (moduleId !== inlineScriptRefId || inlineScriptSession.has(moduleId)) {
			return
		}

		inlineScriptSession.set(moduleId, '')
		emptyInlineScriptModuleIds.push(moduleId)
	}

	function visitModule(module: FlowModule) {
		if (module.value.type === 'rawscript' && module.value.content) {
			const match = module.value.content.match(/^inline_script\.(.+)$/)
			if (match) {
				ensureEmptyInlineScript(module.id, match[1])
			}
			return
		}

		if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			module.value.modules?.forEach(visitModule)
			return
		}

		if (module.value.type === 'branchone') {
			module.value.branches?.forEach((branch) => branch.modules?.forEach(visitModule))
			module.value.default?.forEach(visitModule)
			return
		}

		if (module.value.type === 'branchall') {
			module.value.branches?.forEach((branch) => branch.modules?.forEach(visitModule))
			return
		}

		if (module.value.type === 'aiagent') {
			for (const tool of module.value.tools ?? []) {
				if (
					tool.value &&
					'tool_type' in tool.value &&
					tool.value.tool_type === 'flowmodule' &&
					'type' in tool.value &&
					tool.value.type === 'rawscript' &&
					'content' in tool.value &&
					tool.value.content
				) {
					const match = (tool.value.content as string).match(/^inline_script\.(.+)$/)
					if (match) {
						ensureEmptyInlineScript(tool.id, match[1])
					}
				}
			}
		}
	}

	modules.forEach(visitModule)
	return emptyInlineScriptModuleIds
}
