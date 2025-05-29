<script lang="ts">
	import { langToExt } from '$lib/editorUtils'
	import { ResourceService, type ListResourceResponse, type ScriptLang } from '$lib/gen'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import {
		copilotSessionModel,
		dbSchemas,
		SQLSchemaLanguages,
		workspaceStore,
		type DBSchema
	} from '$lib/stores'
	import { untrack } from 'svelte'
	import type { ContextElement } from './context'
	import type { DisplayMessage } from './shared'
	import { diffLines } from 'diff'

	let {
		displayMessages = $bindable(),
		code,
		lang,
		path,
		args,
		lastSavedCode,
		lastDeployedCode,
		error
	}: {
		displayMessages: DisplayMessage[]
		code: string
		lang: ScriptLang | 'bunnative'
		path: string | undefined
		args: Record<string, any>
		lastSavedCode: string | undefined
		lastDeployedCode: string | undefined
		error: string | undefined
	} = $props()

	let contextCodePath = $derived(
		path
			? (path.split('/').pop() ?? 'script') + '.' + langToExt(scriptLangToEditorLang(lang))
			: undefined
	)

	let selectedContext: ContextElement[] = $derived([
		{
			type: 'code',
			title: contextCodePath ?? '',
			content: code,
			lang
		}
	])

	export function getSelectedContext() {
		return selectedContext
	}

	export function setSelectedContext(newSelectedContext: ContextElement[]) {
		selectedContext = newSelectedContext
	}

	let db: { schema: DBSchema; resource: string } | undefined = $state(undefined)

	function updateSchema() {
		try {
			const schemaRes = lang === 'graphql' ? args.api : args.database
			if (typeof schemaRes === 'string') {
				const schemaPath = schemaRes.replace('$res:', '')
				const schema = $dbSchemas[schemaPath]
				if (schema && schema.lang === lang) {
					db = { schema, resource: schemaPath }
				} else {
					db = undefined
				}
			} else {
				db = undefined
			}
		} catch (err) {
			console.error('Could not update schema', err)
		}
	}

	$effect(() => {
		updateSchema()
	})

	let dbResources: ListResourceResponse = $state([])

	async function updateDBResources() {
		const workspace = $workspaceStore
		if (workspace) {
			dbResources = await ResourceService.listResource({
				workspace: workspace,
				resourceType: SQLSchemaLanguages.join(',')
			})
		}
	}

	$effect(() => {
		updateDBResources()
	})

	function updateAvailableContext() {
		if (!contextCodePath) {
			return []
		}
		const providerModel = $copilotSessionModel
		try {
			let newAvailableContext: ContextElement[] = [
				{
					type: 'code',
					title: contextCodePath,
					content: code,
					lang
				}
			]
			if (!providerModel?.model.endsWith('/thinking')) {
				for (const d of dbResources) {
					const loadedSchema = dbSchemas[d.path]
					newAvailableContext.push({
						type: 'db',
						title: d.path,
						// If the db is already fetched, add the schema to the context
						...(loadedSchema ? { schema: loadedSchema } : {})
					})
				}
			}

			if (lastSavedCode && lastSavedCode !== code) {
				newAvailableContext.push({
					type: 'diff',
					title: 'diff_with_last_saved_draft',
					content: lastSavedCode ?? '',
					diff: diffLines(lastSavedCode ?? '', code),
					lang
				})
			}

			if (lastDeployedCode && lastDeployedCode !== code) {
				newAvailableContext.push({
					type: 'diff',
					title: 'diff_with_last_deployed_version',
					content: lastDeployedCode ?? '',
					diff: diffLines(lastDeployedCode ?? '', code),
					lang
				})
			}

			if (error) {
				newAvailableContext = [
					...newAvailableContext,
					{
						type: 'error',
						title: 'error',
						content: error
					}
				]
			}

			return newAvailableContext
		} catch (err) {
			console.error('Could not update available context', err)
			return []
		}
	}

	function updateDisplayMessages(currentDisplayMessages: DisplayMessage[]): DisplayMessage[] {
		const schemas = $dbSchemas
		return currentDisplayMessages.map((m) => ({
			...m,
			contextElements:
				m.role !== 'tool' && m.contextElements
					? m.contextElements.map((c) =>
							c.type === 'db'
								? {
										type: 'db',
										title: c.title,
										schema: schemas[c.title]
									}
								: c
						)
					: undefined
		}))
	}

	$effect(() => {
		displayMessages = updateDisplayMessages(untrack(() => displayMessages))
	})

	let availableContext: ContextElement[] = $derived.by(updateAvailableContext)

	export function getAvailableContext() {
		return availableContext
	}

	function updateSelectedContext(currentSelectedContext: ContextElement[]) {
		if (!contextCodePath) {
			return currentSelectedContext
		}
		let newSelectedContext: ContextElement[] = [...currentSelectedContext]

		// If the db is already fetched, add it to the selected context
		if (
			db &&
			!newSelectedContext.find((c) => c.type === 'db' && db && c.title === db.resource) &&
			!$copilotSessionModel?.model.endsWith('/thinking')
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
			.map((c) =>
				c.type === 'code_piece' && code.includes(c.content)
					? c
					: availableContext.find((ac) => ac.type === c.type && ac.title === c.title)
			)
			.filter((c) => c !== undefined) as ContextElement[]

		selectedContext = newSelectedContext
	}

	$effect(() => {
		updateSelectedContext(untrack(() => selectedContext))
	})

	export function addSelectedLinesToContext(lines: string, startLine: number, endLine: number) {
		if (
			selectedContext.find(
				(c) => c.type === 'code_piece' && c.title === `L${startLine}-L${endLine}`
			)
		) {
			return
		}
		selectedContext = [
			...selectedContext,
			{
				type: 'code_piece',
				title: `L${startLine}-L${endLine}`,
				startLine,
				endLine,
				content: lines,
				lang
			}
		]
	}

	export function setFixContext() {
		const codeContext = contextCodePath
			? availableContext.find((c) => c.type === 'code' && c.title === contextCodePath)
			: undefined
		const errorContext = availableContext.find((c) => c.type === 'error')

		if (codeContext && errorContext) {
			selectedContext = [codeContext, errorContext]
		}
	}

	export function setAskAiContext(options: { withCode?: boolean; withDiff?: boolean }) {
		const codeContext = availableContext.find(
			(c) => c.type === 'code' && c.title === contextCodePath
		)
		if (!codeContext) {
			return
		}
		selectedContext = [
			...(options.withCode === false ? [] : [codeContext]),
			...(options.withDiff
				? [
						{
							type: 'diff' as const,
							title: 'diff_with_last_deployed_version',
							content: lastDeployedCode ?? '',
							diff: diffLines(lastDeployedCode ?? '', code),
							lang
						}
					]
				: [])
		]
	}

	export function updateContextOnRequest(options: { removeDiff?: boolean; addBackCode?: boolean }) {
		selectedContext = selectedContext.filter((c) => c.type !== 'code_piece')
		if (options.removeDiff) {
			selectedContext = selectedContext.filter((c) => c.type !== 'diff')
		}
		if (options.addBackCode) {
			const codeContext = availableContext.find(
				(c) => c.type === 'code' && c.title === contextCodePath
			)
			if (codeContext) {
				selectedContext = [...selectedContext, codeContext]
			}
		}
	}
</script>
