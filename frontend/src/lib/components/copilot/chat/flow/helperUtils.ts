import type { FlowModule, FlowValue, OpenFlow, RawScript } from '$lib/gen'
import { forEachFlowModule } from '$lib/components/flows/dfs'
import { findModuleInFlow } from '$lib/components/flows/flowTree'
import type { InlineScriptSession } from './inlineScriptsUtils'

type FlowLike = Pick<OpenFlow, 'value'> & {
	schema?: Record<string, any>
}

export type FlowGroup = NonNullable<FlowValue['groups']>[number]

export interface FlowJsonUpdate {
	modules?: FlowModule[]
	schema?: Record<string, any> | null
	preprocessorModule?: FlowModule | null
	failureModule?: FlowModule | null
	groups?: FlowGroup[] | null
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

export function validateFlowGroups(
	rawGroups: unknown,
	moduleIds?: Set<string>
): FlowGroup[] | null {
	if (rawGroups == null) {
		return null
	}

	if (!Array.isArray(rawGroups)) {
		throw new Error('Flow groups must be an array')
	}

	return rawGroups.map((group, index) => {
		if (!group || typeof group !== 'object' || Array.isArray(group)) {
			throw new Error(`Invalid group at index ${index}: must be an object`)
		}
		const g = group as Record<string, unknown>
		if (typeof g.start_id !== 'string' || !g.start_id) {
			throw new Error(`Invalid group at index ${index}: start_id must be a non-empty string`)
		}
		if (typeof g.end_id !== 'string' || !g.end_id) {
			throw new Error(`Invalid group at index ${index}: end_id must be a non-empty string`)
		}
		if (moduleIds) {
			if (!moduleIds.has(g.start_id)) {
				throw new Error(
					`Invalid group at index ${index}: start_id "${g.start_id}" does not match any flow module`
				)
			}
			if (!moduleIds.has(g.end_id)) {
				throw new Error(
					`Invalid group at index ${index}: end_id "${g.end_id}" does not match any flow module`
				)
			}
		}
		return g as unknown as FlowGroup
	})
}

export function applyFlowJsonUpdate(
	flow: FlowLike,
	inlineScriptSession: InlineScriptSession,
	{ modules, schema, preprocessorModule, failureModule, groups }: FlowJsonUpdate
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

	if (groups !== undefined) {
		flow.value.groups = groups == null || groups.length === 0 ? undefined : groups
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
