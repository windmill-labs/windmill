import { ResourceService, type Flow, type ListResourceResponse, type ScriptLang } from '$lib/gen'
import { scriptLangToEditorLang } from '$lib/scripts'
import { SQLSchemaLanguages, type DBSchemas } from '$lib/stores'
import { diffLines } from 'diff'
import { createAppDomSelectorElement, type ContextElement, type FlowModuleElement } from './context'
import type { FlowModule } from '$lib/gen'

import type { DisplayMessage } from './shared'
import { langToExt } from '$lib/editorLangUtils'
import type { ExtendedOpenFlow } from '$lib/components/flows/types'

export interface ScriptOptions {
	lang: ScriptLang | 'bunnative'
	getCode: () => string
	error: string | undefined
	args: Record<string, any>
	path: string | undefined
	lastSavedCode?: string
	lastDeployedCode?: string
	diffMode: boolean
	workflowAsCode?: boolean
}

export interface FlowOptions {
	currentFlow: ExtendedOpenFlow
	lastDeployedFlow?: Flow
	path: string | undefined
	modules: FlowModule[]
	lastSavedFlow?: Flow
}

export default class ContextManager {
	private selectedContext: ContextElement[] = $state([])
	private availableContext: ContextElement[] = $state([])

	private workspace: string | undefined = undefined
	private dbResourcesWorkspace: string | undefined = undefined
	private dbResources: ListResourceResponse = []
	private scriptOptions: ScriptOptions | undefined = undefined

	private async refreshDbResources(workspace: string) {
		this.dbResources = await ResourceService.listResource({
			workspace: workspace,
			resourceType: SQLSchemaLanguages.join(',')
		})
		this.dbResourcesWorkspace = workspace
	}

	private getSelectedDBSchema(scriptOptions: ScriptOptions, dbSchemas: DBSchemas) {
		const schemaRes =
			scriptOptions.lang === 'graphql' ? scriptOptions.args.api : scriptOptions.args.database
		if (typeof schemaRes === 'string') {
			const schemaPath = schemaRes.replace('$res:', '')
			const schema = dbSchemas[schemaPath]
			if (schema && schema.lang === scriptOptions.lang) {
				return { schema, resource: schemaPath }
			} else {
				return { schema: undefined, resource: schemaPath }
			}
		}
	}

	private getContextCodePath(scriptOptions: ScriptOptions) {
		return (
			(scriptOptions.path?.split('/').pop() ?? 'script') +
			'.' +
			langToExt(scriptLangToEditorLang(scriptOptions.lang))
		)
	}

	updateAvailableContextForGlobal(workspace: string, currentlySelectedContext: ContextElement[]) {
		this.availableContext = []
		if (!workspace || (this.workspace !== undefined && this.workspace !== workspace)) {
			this.workspace = workspace
			this.selectedContext = []
			return
		}
		this.workspace = workspace
		this.selectedContext = currentlySelectedContext.filter(
			(context) =>
				context.type === 'workspace_script' ||
				context.type === 'workspace_flow' ||
				context.type === 'workspace_app'
		)
	}

	async updateAvailableContextForFlow(
		flowOptions: FlowOptions,
		dbSchemas: DBSchemas,
		workspace: string,
		toolSupport: boolean,
		currentlySelectedContext: ContextElement[]
	) {
		try {
			if (this.dbResourcesWorkspace !== workspace) {
				await this.refreshDbResources(workspace)
			}
			this.workspace = workspace

			let newAvailableContext: ContextElement[] = []

			// Add diff context if we have a deployed flow version
			const deployedFlowString = JSON.stringify(flowOptions.lastDeployedFlow, null, 2)
			const savedFlowString = JSON.stringify(flowOptions.lastSavedFlow, null, 2)
			const currentFlowString = JSON.stringify(flowOptions.currentFlow, null, 2)

			if (currentFlowString && deployedFlowString && deployedFlowString !== currentFlowString) {
				newAvailableContext.push({
					type: 'diff',
					title: 'diff_with_last_deployed_version',
					content: deployedFlowString,
					diff: diffLines(deployedFlowString, currentFlowString),
					lang: 'graphql' // irrelevant, but needed for the diff component
				})
			}

			if (currentFlowString && savedFlowString && savedFlowString !== currentFlowString) {
				newAvailableContext.push({
					type: 'diff',
					title: 'diff_with_last_saved_draft',
					content: savedFlowString,
					diff: diffLines(savedFlowString, currentFlowString),
					lang: 'graphql' // irrelevant, but needed for the diff component
				})
			}

			for (const module of flowOptions.modules) {
				newAvailableContext.push({
					type: 'flow_module',
					id: module.id,
					title: `${module.id}`,
					value: {
						language: 'language' in module.value ? module.value.language : 'bunnative',
						path: 'path' in module.value ? module.value.path : '',
						content: 'content' in module.value ? module.value.content : '',
						type: module.value.type
					}
				})
			}

			if (toolSupport) {
				for (const d of this.dbResources) {
					const loadedSchema = dbSchemas[d.path]
					newAvailableContext.push({
						type: 'db',
						title: d.path,
						// If the db is already fetched, add the schema to the context
						...(loadedSchema ? { schema: loadedSchema } : {})
					})
				}
			}

			let newSelectedContext: ContextElement[] = [...currentlySelectedContext]

			// Filter selected context to only include available items. Workspace
			// references (workspace_script / workspace_flow) are user-picked via
			// the @-mention picker and intentionally aren't in availableContext —
			// preserve them unconditionally so the badge survives editor refreshes.
			newSelectedContext = newSelectedContext
				.filter(
					(c) =>
						c.type === 'workspace_script' ||
						c.type === 'workspace_flow' ||
						c.type === 'workspace_app' ||
						newAvailableContext.some((ac) => ac.type === c.type && ac.title === c.title)
				)
				.map((c) =>
					c.type === 'db' && dbSchemas[c.title]
						? {
								...c,
								schema: dbSchemas[c.title]
							}
						: c
				)

			this.availableContext = newAvailableContext
			this.selectedContext = newSelectedContext
		} catch (err) {
			console.error('Could not update available context for flow', err)
		}
	}

	async updateAvailableContext(
		scriptOptions: ScriptOptions,
		dbSchemas: DBSchemas,
		workspace: string,
		toolSupport: boolean,
		currentlySelectedContext: ContextElement[]
	) {
		try {
			if (this.dbResourcesWorkspace !== workspace) {
				await this.refreshDbResources(workspace)
			}
			this.workspace = workspace
			let newAvailableContext: ContextElement[] = [
				{
					type: 'code',
					title: this.getContextCodePath(scriptOptions) ?? '',
					content: scriptOptions.getCode(),
					lang: scriptOptions.lang
				}
			]

			if (toolSupport) {
				for (const d of this.dbResources) {
					const loadedSchema = dbSchemas[d.path]
					newAvailableContext.push({
						type: 'db',
						title: d.path,
						// If the db is already fetched, add the schema to the context
						...(loadedSchema ? { schema: loadedSchema } : {})
					})
				}
			}

			if (scriptOptions.lastSavedCode && scriptOptions.lastSavedCode !== scriptOptions.getCode()) {
				newAvailableContext.push({
					type: 'diff',
					title: 'diff_with_last_saved_draft', // can't use spaces in the title, because it will break the word match in the context text area hightlighting logic
					content: scriptOptions.lastSavedCode ?? '',
					diff: diffLines(scriptOptions.lastSavedCode ?? '', scriptOptions.getCode()),
					lang: scriptOptions.lang
				})
			}

			if (
				scriptOptions.lastDeployedCode &&
				scriptOptions.lastDeployedCode !== scriptOptions.getCode()
			) {
				newAvailableContext.push({
					type: 'diff',
					title: 'diff_with_last_deployed_version',
					content: scriptOptions.lastDeployedCode ?? '',
					diff: diffLines(scriptOptions.lastDeployedCode ?? '', scriptOptions.getCode()),
					lang: scriptOptions.lang
				})
			}

			if (scriptOptions.error) {
				newAvailableContext = [
					...newAvailableContext,
					{
						type: 'error',
						title: 'error',
						content: scriptOptions.error
					}
				]
			}

			// Seed with the (refreshed) code block + everything else previously
			// selected. The filter further down validates each entry against
			// newAvailableContext (and the per-type allowlist for code_piece /
			// workspace_*); types that are auto-derived (diff/error/db) survive
			// when they're still in availableContext, user-picked workspace refs
			// survive unconditionally, and `code` is excluded from the carryover
			// because we just rebuilt it.
			let newSelectedContext: ContextElement[] = [
				{
					type: 'code',
					title: this.getContextCodePath(scriptOptions) ?? '',
					content: scriptOptions.getCode(),
					lang: scriptOptions.lang,
					deletable: false
				},
				...currentlySelectedContext.filter((c) => c.type !== 'code')
			]

			const db = this.getSelectedDBSchema(scriptOptions, dbSchemas)
			if (
				db &&
				!newSelectedContext.find((c) => c.type === 'db' && db && c.title === db.resource) &&
				toolSupport
			) {
				newSelectedContext = [
					...newSelectedContext,
					{
						type: 'db',
						title: db.resource,
						schema: db.schema
					}
				]
			}

			newSelectedContext = newSelectedContext
				.filter(
					(c) =>
						(c.type === 'code_piece' && scriptOptions.getCode().includes(c.content)) ||
						c.type === 'code' ||
						// Workspace references are user-picked via @-mention and not in
						// availableContext; preserve so badges survive editor refreshes.
						c.type === 'workspace_script' ||
						c.type === 'workspace_flow' ||
						c.type === 'workspace_app' ||
						newAvailableContext.some((ac) => ac.type === c.type && ac.title === c.title)
				)
				.map((c) => {
					if (c.type === 'code') {
						return {
							...c,
							content: scriptOptions.getCode(),
							title: this.getContextCodePath(scriptOptions)
						}
					}
					if (c.type === 'db' && dbSchemas[c.title]) {
						return { ...c, schema: dbSchemas[c.title] }
					}
					// For other auto-derived types (diff, error), rehydrate from the
					// freshly-built newAvailableContext so the carryover doesn't keep
					// stale `content` / `diff` payloads — preserve the user-set
					// `deletable` flag on top of the fresh entry.
					const fresh = newAvailableContext.find((ac) => ac.type === c.type && ac.title === c.title)
					if (fresh && 'deletable' in c) {
						return { ...fresh, deletable: c.deletable } as ContextElement
					}
					return fresh ?? c
				})

			this.availableContext = newAvailableContext
			this.selectedContext = newSelectedContext
		} catch (err) {
			console.error('Could not update available context', err)
		}
	}

	getSelectedContext() {
		return this.selectedContext
	}

	setSelectedContext(newSelectedContext: ContextElement[]) {
		this.selectedContext = newSelectedContext
	}

	getAvailableContext() {
		return this.availableContext
	}

	setScriptOptions(scriptOptions: ScriptOptions) {
		this.scriptOptions = scriptOptions
	}

	addSelectedLinesToContext(lines: string, startLine: number, endLine: number, moduleId?: string) {
		const title = moduleId ? `${moduleId} L${startLine}-L${endLine}` : `L${startLine}-L${endLine}`
		if (
			!this.scriptOptions ||
			this.selectedContext.find(
				(c) =>
					(c.type === 'code_piece' && c.title === title) ||
					(c.type === 'flow_module_code_piece' && c.id === moduleId && c.title === title)
			)
		) {
			return
		}
		if (moduleId) {
			const module = [...this.availableContext, ...this.selectedContext].find(
				(c) => c.type === 'flow_module' && c.id === moduleId
			) as FlowModuleElement
			if (!module) {
				console.error('Module not found', moduleId)
				return
			}
			this.selectedContext = [
				...this.selectedContext,
				{
					type: 'flow_module_code_piece',
					id: moduleId,
					title: title,
					startLine,
					endLine,
					content: lines,
					lang: this.scriptOptions.lang,
					value: module.value
				}
			]
		} else {
			this.selectedContext = [
				...this.selectedContext,
				{
					type: 'code_piece',
					title: title,
					startLine,
					endLine,
					content: lines,
					lang: this.scriptOptions.lang
				}
			]
		}
	}

	/**
	 * Attach a raw-app preview element (picked via the inspector) as a selector chip.
	 * Deduped by selector; the model reads the element live via search_dom / read_dom.
	 */
	addSelectedDomElement(info: {
		selector: string
		appPath: string
		tagName: string
		id?: string
		className?: string
	}) {
		if (
			this.selectedContext.find(
				(c) =>
					c.type === 'app_dom_selector' &&
					c.selector === info.selector &&
					c.appPath === info.appPath
			)
		) {
			return
		}
		this.selectedContext = [...this.selectedContext, createAppDomSelectorElement(info)]
	}

	/** Replace all DOM selector chips with this single element (plain inspector pick). */
	setSelectedDomElement(info: {
		selector: string
		appPath: string
		tagName: string
		id?: string
		className?: string
	}) {
		this.selectedContext = [
			...this.selectedContext.filter((c) => c.type !== 'app_dom_selector'),
			createAppDomSelectorElement(info)
		]
	}

	/** Remove one DOM selector chip (× on the chip or on the preview overlay).
	 * Scope by appPath when known: two apps can generate the same selector, so a
	 * selector-only match would drop another app's chip. */
	removeSelectedDomElement(selector: string, appPath?: string) {
		this.selectedContext = this.selectedContext.filter(
			(c) =>
				!(
					c.type === 'app_dom_selector' &&
					c.selector === selector &&
					(appPath === undefined || c.appPath === appPath)
				)
		)
	}

	/** Drop DOM selector chips (one-shot per message — cleared after send). */
	clearSelectedDomElements() {
		this.selectedContext = this.selectedContext.filter((c) => c.type !== 'app_dom_selector')
	}

	setFixContext() {
		const codeContext = this.availableContext.find((c) => c.type === 'code')
		const errorContext = this.availableContext.find((c) => c.type === 'error')

		if (codeContext && errorContext) {
			this.selectedContext = [codeContext, errorContext]
		}
	}

	setAskAiContext(options: { withCode?: boolean; withDiff?: boolean }) {
		if (!this.scriptOptions) {
			return
		}
		const codeContext = this.availableContext.find((c) => c.type === 'code')
		if (!codeContext) {
			return
		}
		this.selectedContext = [
			...(options.withCode === false ? [] : [codeContext]),
			...(options.withDiff
				? [
						{
							type: 'diff' as const,
							title: 'diff_with_last_deployed_version',
							content: this.scriptOptions.lastDeployedCode ?? '',
							diff: diffLines(
								this.scriptOptions.lastDeployedCode ?? '',
								this.scriptOptions.getCode()
							),
							lang: this.scriptOptions.lang
						}
					]
				: [])
		]
	}

	updateContextOnRequest(options: { removeDiff?: boolean; addBackCode?: boolean }) {
		this.selectedContext = this.selectedContext.filter((c) => c.type !== 'code_piece')
		if (options.removeDiff) {
			this.selectedContext = this.selectedContext.filter((c) => c.type !== 'diff')
		}
		if (options.addBackCode) {
			const codeContext = this.availableContext.find((c) => c.type === 'code')
			if (codeContext) {
				this.selectedContext = [...this.selectedContext, codeContext]
			}
		}
	}

	static updateDisplayMessages(
		displayMessages: DisplayMessage[],
		dbSchemas: DBSchemas
	): DisplayMessage[] {
		return displayMessages.map((m) => {
			// Only user/assistant messages carry contextElements; tool and summary
			// messages pass through untouched.
			if ((m.role === 'user' || m.role === 'assistant') && m.contextElements) {
				return {
					...m,
					contextElements: m.contextElements.map((c) =>
						c.type === 'db'
							? {
									type: 'db' as const,
									title: c.title,
									schema: dbSchemas[c.title]
								}
							: c
					)
				}
			}
			return m
		})
	}

	setSelectedModuleContext(
		moduleId: string | undefined,
		availableContext: ContextElement[] | undefined
	) {
		if (availableContext && moduleId) {
			const module = availableContext.find((c) => c.type === 'flow_module' && c.id === moduleId)
			if (
				module &&
				!this.selectedContext.find((c) => c.type === 'flow_module' && c.id === moduleId)
			) {
				this.selectedContext = this.selectedContext.filter((c) => c.type !== 'flow_module')
				this.selectedContext = [module, ...this.selectedContext]
			}
		} else if (!moduleId) {
			this.selectedContext = this.selectedContext.filter((c) => c.type !== 'flow_module')
		}
	}

	clearContext() {
		this.selectedContext = []
	}
}
