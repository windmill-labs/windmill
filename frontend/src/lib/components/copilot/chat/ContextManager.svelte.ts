import { langToExt } from '$lib/editorUtils'
import { ResourceService, type ListResourceResponse, type ScriptLang } from '$lib/gen'
import { scriptLangToEditorLang } from '$lib/scripts'
import { SQLSchemaLanguages, type DBSchemas } from '$lib/stores'
import { diffLines } from 'diff'
import type { ContextElement } from './context'

import type { DisplayMessage } from './shared'

export interface ScriptOptions {
	lang: ScriptLang | 'bunnative'
	code: string
	error: string | undefined
	args: Record<string, any>
	path: string | undefined
	lastSavedCode?: string
	lastDeployedCode?: string
	diffMode: boolean
}

export default class ContextManager {
	private selectedContext: ContextElement[] = $state([])
	private availableContext: ContextElement[] = $state([])

	private workspace: string | undefined = undefined
	private dbResources: ListResourceResponse = []
	private scriptOptions: ScriptOptions | undefined = undefined

	private async refreshDbResources(workspace: string) {
		this.dbResources = await ResourceService.listResource({
			workspace: workspace,
			resourceType: SQLSchemaLanguages.join(',')
		})
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

	async updateAvailableContext(
		scriptOptions: ScriptOptions,
		dbSchemas: DBSchemas,
		workspace: string,
		toolSupport: boolean,
		currentlySelectedContext: ContextElement[]
	) {
		try {
			let firstTime = !this.workspace
			if (this.workspace !== workspace) {
				await this.refreshDbResources(workspace)
				this.workspace = workspace
			}
			this.scriptOptions = scriptOptions
			let newAvailableContext: ContextElement[] = [
				{
					type: 'code',
					title: this.getContextCodePath(scriptOptions) ?? '',
					content: scriptOptions.code,
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

			if (scriptOptions.lastSavedCode && scriptOptions.lastSavedCode !== scriptOptions.code) {
				newAvailableContext.push({
					type: 'diff',
					title: 'diff_with_last_saved_draft',
					content: scriptOptions.lastSavedCode ?? '',
					diff: diffLines(scriptOptions.lastSavedCode ?? '', scriptOptions.code),
					lang: scriptOptions.lang
				})
			}

			if (scriptOptions.lastDeployedCode && scriptOptions.lastDeployedCode !== scriptOptions.code) {
				newAvailableContext.push({
					type: 'diff',
					title: 'diff_with_last_deployed_version',
					content: scriptOptions.lastDeployedCode ?? '',
					diff: diffLines(scriptOptions.lastDeployedCode ?? '', scriptOptions.code),
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

			let newSelectedContext: ContextElement[] = [...currentlySelectedContext]

			if (firstTime) {
				newSelectedContext = [
					{
						type: 'code',
						title: this.getContextCodePath(scriptOptions) ?? '',
						content: scriptOptions.code,
						lang: scriptOptions.lang
					}
				]
			}

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
						(c.type === 'code_piece' && scriptOptions.code.includes(c.content)) ||
						c.type === 'code' ||
						newAvailableContext.some((ac) => ac.type === c.type && ac.title === c.title)
				)
				.map((c) =>
					c.type === 'code'
						? {
								...c,
								content: scriptOptions.code,
								title: this.getContextCodePath(scriptOptions)
							}
						: c.type === 'db' && dbSchemas[c.title]
							? {
									...c,
									schema: dbSchemas[c.title]
								}
							: c
				)

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

	addSelectedLinesToContext(lines: string, startLine: number, endLine: number) {
		if (
			!this.scriptOptions ||
			this.selectedContext.find(
				(c) => c.type === 'code_piece' && c.title === `L${startLine}-L${endLine}`
			)
		) {
			return
		}
		this.selectedContext = [
			...this.selectedContext,
			{
				type: 'code_piece',
				title: `L${startLine}-L${endLine}`,
				startLine,
				endLine,
				content: lines,
				lang: this.scriptOptions.lang
			}
		]
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
							diff: diffLines(this.scriptOptions.lastDeployedCode ?? '', this.scriptOptions.code),
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
		return displayMessages.map((m) => ({
			...m,
			contextElements:
				m.role !== 'tool' && m.contextElements
					? m.contextElements.map((c) =>
							c.type === 'db'
								? {
										type: 'db',
										title: c.title,
										schema: dbSchemas[c.title]
									}
								: c
						)
					: undefined
		}))
	}
}
