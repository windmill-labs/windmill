<script module>
	import '@codingame/monaco-vscode-standalone-languages'
	import '@codingame/monaco-vscode-standalone-typescript-language-features'
	import { typescriptDefaults } from '@codingame/monaco-vscode-standalone-typescript-language-features'
</script>

<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { sendUserToast } from '$lib/toast'

	import { createEventDispatcher, onDestroy, onMount, untrack } from 'svelte'

	// import libStdContent from '$lib/es6.d.ts.txt?raw'
	// import domContent from '$lib/dom.d.ts.txt?raw'

	// import denoFetchContent from '$lib/deno_fetch.d.ts.txt?raw'

	import * as vscode from 'vscode'
	// import '@codingame/monaco-vscode-typescript-basics-default-extension'
	// import '@codingame/monaco-vscode-typescript-language-features-default-extension'
	// import 'vscode/localExtensionHost'

	import { MonacoLanguageClient } from 'monaco-languageclient'

	import { toSocket, WebSocketMessageReader, WebSocketMessageWriter } from 'vscode-ws-jsonrpc'
	import { CloseAction, ErrorAction, RequestType } from 'vscode-languageclient'
	import type { DocumentUri, MessageTransports } from 'vscode-languageclient'

	import { MonacoBinding } from 'y-monaco'
	import {
		dbSchemas,
		type DBSchema,
		codeCompletionSessionEnabled,
		lspTokenStore,
		formatOnSave,
		vimMode,
		relativeLineNumbers
	} from '$lib/stores'

	import { editorConfig, updateOptions } from '$lib/editorUtils'
	import { createHash as randomHash } from '$lib/editorLangUtils'
	import { workspaceStore } from '$lib/stores'
	import {
		type Preview,
		ResourceService,
		type ScriptLang,
		UserService,
		WorkspaceService
	} from '$lib/gen'
	import type { Text } from 'yjs'
	import {
		initializeVscode,
		keepModelAroundToAvoidDisposalOfWorkers,
		MONACO_Y_PADDING
	} from '$lib/components/vscode'

	// import { initializeMode } from 'monaco-graphql/esm/initializeMode.js'
	// import type { MonacoGraphQLAPI } from 'monaco-graphql/esm/api.js'

	import {
		editor as meditor,
		languages,
		KeyCode,
		KeyMod,
		Uri as mUri,
		type IRange,
		type IDisposable
	} from 'monaco-editor'

	import EditorTheme from './EditorTheme.svelte'
	import {
		BIGQUERY_TYPES,
		DUCKDB_TYPES,
		MSSQL_TYPES,
		MYSQL_TYPES,
		ORACLEDB_TYPES,
		POSTGRES_TYPES,
		SNOWFLAKE_TYPES
	} from '$lib/consts'
	import { setupTypeAcquisition, type DepsToGet } from '$lib/ata/index'
	import { initWasmTs, type InferAssetsSqlQueryDetails } from '$lib/infer'
	import { initVim } from './monaco_keybindings'
	import { updateSqlQueriesInWorker, waitForWorkerInitialization } from './sqlTypeService'
	import { parseTypescriptDeps } from '$lib/relative_imports'

	import { scriptLangToEditorLang } from '$lib/scripts'
	import * as htmllang from '$lib/svelteMonarch'
	import { conf, language } from '$lib/vueMonarch'

	import { Autocompletor } from './copilot/autocomplete/Autocompletor'
	import { AIChatEditorHandler, type ReviewChangesOpts } from './copilot/chat/monaco-adapter'
	import GlobalReviewButtons from './copilot/chat/GlobalReviewButtons.svelte'
	import AIChatInlineWidget from './copilot/chat/AIChatInlineWidget.svelte'
	import { writable } from 'svelte/store'
	import { formatResourceTypes } from './copilot/chat/script/core'
	import type { ScriptLintResult } from './copilot/chat/shared'
	import FakeMonacoPlaceHolder from './FakeMonacoPlaceHolder.svelte'
	import { editorPositionMap } from '$lib/utils'
	import { extToLang, langToExt } from '$lib/editorLangUtils'
	import { aiChatManager } from './copilot/chat/AIChatManager.svelte'
	import type { Selection } from 'monaco-editor'
	import { canHavePreprocessor, getPreprocessorModuleCode } from '$lib/script_helpers'
	import { setMonacoTypescriptOptions } from './monacoLanguagesOptions'
	import { copilotInfo } from '$lib/aiStore'
	import { getDbSchemas } from './apps/components/display/dbtable/metadata'
	import { rawAppLintStore, type MonacoLintError } from './raw_apps/lintStore'
	import { MarkerSeverity } from 'monaco-editor'
	import { resource, useDebounce, watch } from 'runed'
	// import EditorTheme from './EditorTheme.svelte'

	let divEl: HTMLDivElement | null = $state(null)
	let editor: meditor.IStandaloneCodeEditor | null = $state(null)

	interface Props {
		code?: string
		cmdEnterAction?: (() => void) | undefined
		formatAction?: (() => void) | undefined
		automaticLayout?: boolean
		websocketAlive?: any
		shouldBindKey?: boolean
		fixedOverflowWidgets?: boolean
		path?: string | undefined
		yContent?: Text | undefined
		awareness?: any | undefined
		folding?: boolean
		args?: Record<string, any> | undefined
		useWebsockets?: boolean
		small?: boolean
		scriptLang: Preview['language'] | 'bunnative' | 'tsx' | 'jsx' | 'json' | undefined
		disabled?: boolean
		lineNumbersMinChars?: number
		files?: Record<string, { code: string; readonly?: boolean }> | undefined
		extraLib?: string | undefined
		changeTimeout?: number
		loadAsync?: boolean
		key?: string | undefined
		class?: string | undefined
		moduleId?: string
		enablePreprocessorSnippet?: boolean
		/** When set, enables raw app lint collection mode and reports Monaco markers to the lint store under this key */
		rawAppRunnableKey?: string | undefined
		// Used to provide typed queries in TypeScript when detecting assets
		preparedAssetsSqlQueries?: InferAssetsSqlQueryDetails[] | undefined
		// To execute preview scripts with the right worker group
		customTag?: string
	}

	let {
		code = $bindable(),
		cmdEnterAction = undefined,
		formatAction = undefined,
		automaticLayout = true,
		websocketAlive = $bindable(),
		shouldBindKey = true,
		fixedOverflowWidgets = true,
		path = undefined,
		yContent = undefined,
		awareness = undefined,
		folding = false,
		args = undefined,
		useWebsockets = true,
		small = false,
		scriptLang,
		disabled = false,
		lineNumbersMinChars = 3,
		files = {},
		extraLib = undefined,
		changeTimeout = 500,
		loadAsync = false,
		key = undefined,
		class: clazz = undefined,
		moduleId = undefined,
		enablePreprocessorSnippet = false,
		rawAppRunnableKey = undefined,
		preparedAssetsSqlQueries,
		customTag
	}: Props = $props()

	$effect.pre(() => {
		if (websocketAlive == undefined) {
			websocketAlive = {
				pyright: false,
				ruff: false,
				deno: false,
				go: false,
				shellcheck: false
			}
		}
	})

	let lang = $state(scriptLangToEditorLang(scriptLang))

	let filePath = $state(computePath(path))

	let initialPath: string | undefined = $state(path)

	let websockets: WebSocket[] = []
	let languageClients: MonacoLanguageClient[] = []
	let websocketInterval: number | undefined
	let lastWsAttempt: Date = new Date()
	let nbWsAttempt = 0
	let disposeMethod: (() => void) | undefined
	const dispatch = createEventDispatcher()
	// let graphqlService: MonacoGraphQLAPI | undefined = undefined

	let dbSchema: DBSchema | undefined = $state(undefined)

	let destroyed = false
	const uri = computeUri(
		untrack(() => filePath),
		scriptLang
	)

	console.log('uri', uri)

	function computeUri(filePath: string, scriptLang: string | undefined) {
		let file
		if (filePath.includes('.')) {
			file = filePath
		} else {
			file = `${filePath}.${scriptLang == 'tsx' ? 'tsx' : langToExt(lang)}`
		}
		if (file.startsWith('/')) {
			file = file.slice(1)
		}
		return !['deno', 'go', 'python3'].includes(scriptLang ?? '')
			? `file:///${file}`
			: `file:///tmp/monaco/${file}`
	}

	function computePath(path: string | undefined): string {
		if (
			['deno', 'go', 'python3'].includes(scriptLang ?? '') ||
			path == '' ||
			path == undefined //||path.startsWith('/')
		) {
			return randomHash()
		} else {
			// console.log('path', path)
			return path as string
		}
	}

	export function switchToFile(path: string, value: string, lang: string) {
		if (editor) {
			const uri = mUri.parse(path)
			console.log('switching to file', path, lang)
			// vscode.workspace.fs.writeFile(uri, new TextEncoder().encode(value))
			let nmodel = meditor.getModel(uri)
			if (nmodel) {
				console.log('using existing model', path)
				editor.setModel(nmodel)
			} else {
				console.log('creating model', path)
				nmodel = meditor.createModel(value, lang, uri)
				editor.setModel(nmodel)
			}
			model = nmodel
			setTypescriptExtraLibs()
		}
	}

	let valueAfterDispose: string | undefined = undefined
	export function getCode(): string {
		if (valueAfterDispose != undefined) {
			return valueAfterDispose
		}
		return editor?.getValue() ?? ''
	}

	export function getModel(): meditor.IEditorModel | undefined {
		return editor?.getModel() ?? undefined
	}

	export function insertAtCursor(code: string): void {
		if (editor) {
			editor.trigger('keyboard', 'type', { text: code })
		}
	}

	export function insertAtCurrentLine(code: string): void {
		if (editor) {
			insertAtLine(code, editor.getPosition()?.lineNumber ?? 0)
		}
	}

	export function arrowDown(): void {
		if (editor) {
			let pos = editor.getPosition()
			if (pos) {
				editor.setPosition({ lineNumber: pos.lineNumber + 1, column: pos.column })
			}
		}
	}

	export function backspace(): void {
		if (editor) {
			editor.trigger('keyboard', 'deleteLeft', {})
		}
	}

	export function insertAtBeginning(code: string): void {
		if (editor) {
			const range = { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 1 }
			const op = { range: range, text: code, forceMoveMarkers: true }
			editor.executeEdits('external', [op])
		}
	}

	export function insertAtLine(code: string, line: number): void {
		if (editor) {
			const range = { startLineNumber: line, startColumn: 1, endLineNumber: line, endColumn: 1 }
			const op = { range: range, text: code, forceMoveMarkers: true }
			editor.executeEdits('external', [op])
		}
	}

	export function getSelectedLines(): string | undefined {
		if (editor) {
			const selection = editor.getSelection()
			if (selection) {
				const range: IRange = {
					startLineNumber: selection.startLineNumber,
					startColumn: 1,
					endLineNumber: selection.endLineNumber + 1,
					endColumn: 1
				}
				return editor.getModel()?.getValueInRange(range)
			}
		}
	}

	export function onDidChangeCursorSelection(f: (e: meditor.ICursorSelectionChangedEvent) => void) {
		if (editor) {
			return editor.onDidChangeCursorSelection(f)
		}
	}

	export function show(): void {
		divEl?.classList.remove('hidden')
	}

	export function hide(): void {
		divEl?.classList.add('hidden')
	}

	export function setCode(ncode: string, noHistory: boolean = false): void {
		if (code != ncode) {
			code = ncode
		}

		if (noHistory) {
			editor?.setValue(ncode)
		} else {
			if (editor?.getModel()) {
				// editor.setValue(ncode)
				editor.pushUndoStop()

				editor.executeEdits('set', [
					{
						range: editor.getModel()!.getFullModelRange(), // full range
						text: ncode
					}
				])

				editor.pushUndoStop()
			}
		}
		// Update lint diagnostics after code change
		updateRawAppLintDiagnostics()
	}

	/** Collect Monaco markers and update the raw app lint store */
	function updateRawAppLintDiagnostics(): void {
		if (!rawAppRunnableKey || !model) return
		const markers = meditor.getModelMarkers({ resource: model.uri })
		const lintErrors: MonacoLintError[] = markers
			.filter((m) => m.severity === MarkerSeverity.Error || m.severity === MarkerSeverity.Warning)
			.map((m) => ({
				message: m.message,
				severity: m.severity === MarkerSeverity.Error ? 'error' : 'warning',
				startLineNumber: m.startLineNumber,
				startColumn: m.startColumn,
				endLineNumber: m.endLineNumber,
				endColumn: m.endColumn
			}))
		rawAppLintStore.setDiagnostics(rawAppRunnableKey, lintErrors)
	}

	function updateCode() {
		const ncode = getCode()
		if (code == ncode) {
			return
		}
		code = ncode
		dispatch('change', ncode)
	}

	export function append(code: string): void {
		if (editor) {
			const lineCount = editor.getModel()?.getLineCount() || 0
			const lastLineLength = editor.getModel()?.getLineLength(lineCount) || 0
			const range: IRange = {
				startLineNumber: lineCount,
				startColumn: lastLineLength + 1,
				endLineNumber: lineCount,
				endColumn: lastLineLength + 1
			}
			editor.executeEdits('append', [
				{
					range,
					text: code,
					forceMoveMarkers: true
				}
			])
			editor.revealLine(lineCount)
		}
	}

	export async function format() {
		if (editor) {
			updateCode()
			if (lang != 'shell' && lang != 'nu') {
				if ($formatOnSave != false) {
					if (scriptLang == 'deno' && languageClients.length > 0) {
						languageClients.forEach(async (x) => {
							let edits = await x.sendRequest(new RequestType('textDocument/formatting'), {
								textDocument: { uri },
								options: {
									tabSize: 2,
									insertSpaces: true
								}
							})
							console.debug(edits)
							if (Array.isArray(edits)) {
								edits = edits.map((edit) =>
									edit.range.start != undefined &&
									edit.range.end != undefined &&
									edit.newText != undefined
										? {
												range: {
													startLineNumber: edit.range.start.line + 1,
													startColumn: edit.range.start.character + 1,
													endLineNumber: edit.range.end.line + 1,
													endColumn: edit.range.end.character + 1
												},
												text: edit.newText
											}
										: {}
								)
								//@ts-ignore
								editor?.executeEdits('fmt', edits)
							}
						})
					} else {
						await editor?.getAction('editor.action.formatDocument')?.run()
					}
				}
				updateCode()
			}
			if (formatAction) {
				formatAction()
			}
		}
	}

	export function getScriptLang(): string | undefined {
		return scriptLang
	}

	export function getEditor(): meditor.IStandaloneCodeEditor | null {
		return editor
	}

	/** Get lint errors and warnings from the Monaco editor */
	export function getLintErrors(): ScriptLintResult {
		if (!model) {
			return { errorCount: 0, warningCount: 0, errors: [], warnings: [] }
		}

		const markers = meditor.getModelMarkers({ resource: model.uri })
		const errors = markers.filter((m) => m.severity === MarkerSeverity.Error)
		const warnings = markers.filter((m) => m.severity === MarkerSeverity.Warning)

		return {
			errorCount: errors.length,
			warningCount: warnings.length,
			errors,
			warnings
		}
	}

	let command: IDisposable | undefined = undefined

	let sqlTypeCompletor: IDisposable | undefined = $state(undefined)
	let resultCollectionCompletor: IDisposable | undefined = $state(undefined)

	function addSqlTypeCompletions() {
		sqlTypeCompletor?.dispose()
		resultCollectionCompletor?.dispose()

		resultCollectionCompletor = languages.registerCompletionItemProvider('sql', {
			triggerCharacters: ['='],
			provideCompletionItems: function (model, position) {
				const lineContent = model.getLineContent(position.lineNumber)
				const match = lineContent.match(/^--\s*result_collection=/)
				if (!match) {
					return { suggestions: [] }
				}
				const word = model.getWordUntilPosition(position)
				const range = {
					startLineNumber: position.lineNumber,
					endLineNumber: position.lineNumber,
					startColumn: word.startColumn,
					endColumn: word.endColumn
				}
				const suggestions = [
					'last_statement_all_rows',
					'last_statement_first_row',
					'last_statement_all_rows_scalar',
					'last_statement_first_row_scalar',
					'all_statements_all_rows',
					'all_statements_first_row',
					'all_statements_all_rows_scalar',
					'all_statements_first_row_scalar'
				].map((label) => ({
					label: label,
					kind: languages.CompletionItemKind.Function,
					insertText: label,
					range,
					sortText: 'a'
				}))
				return { suggestions }
			}
		})
		sqlTypeCompletor = languages.registerCompletionItemProvider('sql', {
			triggerCharacters: scriptLang === 'postgresql' ? [':'] : ['('],
			provideCompletionItems: function (model, position) {
				const lineUntilPosition = model.getValueInRange({
					startLineNumber: position.lineNumber,
					startColumn: 1,
					endLineNumber: position.lineNumber,
					endColumn: position.column
				})
				let suggestions: languages.CompletionItem[] = []
				if (
					scriptLang === 'postgresql'
						? lineUntilPosition.endsWith('::')
						: lineUntilPosition.match(/^-- .* \(/)
				) {
					const word = model.getWordUntilPosition(position)
					const range = {
						startLineNumber: position.lineNumber,
						endLineNumber: position.lineNumber,
						startColumn: word.startColumn,
						endColumn: word.endColumn
					}
					suggestions = (
						scriptLang === 'postgresql'
							? POSTGRES_TYPES
							: scriptLang === 'mysql'
								? MYSQL_TYPES
								: scriptLang === 'snowflake'
									? SNOWFLAKE_TYPES
									: scriptLang === 'bigquery'
										? BIGQUERY_TYPES
										: scriptLang === 'mssql'
											? MSSQL_TYPES
											: scriptLang === 'oracledb'
												? ORACLEDB_TYPES
												: scriptLang === 'duckdb'
													? DUCKDB_TYPES
													: []
					).map((t) => ({
						label: t,
						kind: languages.CompletionItemKind.Function,
						insertText: t,
						range: range,
						sortText: 'a'
					}))
				}
				return {
					suggestions
				}
			}
		})
	}

	let sqlSchemaCompletor: IDisposable | undefined = undefined

	async function updateSchema(newSchemaRes: string | undefined) {
		if (typeof newSchemaRes === 'string') {
			const resourcePath = newSchemaRes.replace('$res:', '')
			dbSchema = $dbSchemas[resourcePath]
			if (dbSchema === undefined) {
				$dbSchemas[resourcePath] = await getDbSchemas(
					lang === 'graphql' ? 'graphql' : (scriptLang ?? ''),
					resourcePath,
					$workspaceStore,
					(e) => console.error(`error getting ${lang} (${scriptLang}) db schema`, e),
					{ customTag }
				)
			}
			dbSchema = $dbSchemas[resourcePath]
		} else {
			dbSchema = undefined
		}
	}

	function disposeSqlSchemaCompletor() {
		sqlSchemaCompletor?.dispose()
	}
	function disposeGaphqlService() {
		// graphqlService = undefined
	}

	function addDBSchemaCompletions() {
		const { lang: schemaLang, schema } = dbSchema || {}
		if (!schemaLang || !schema) {
			return
		}
		console.log('adding db schema completions', schemaLang)
		if (schemaLang === 'graphql') {
			//graphql depreciated until https://github.com/graphql/graphiql/issues/4104 is fixed with monaco > 0.52.2
			// languages.register({ id: 'graphql' })
			// graphqlService ||= initializeMode()
			// console.log('setting schema config', schema)
			// graphqlService?.setSchemaConfig([
			// 	{
			// 		uri: 'my-schema.graphql',
			// 		introspectionJSON: schema
			// 	}
			// ])
		} else {
			if (sqlSchemaCompletor) {
				sqlSchemaCompletor.dispose()
			}
			sqlSchemaCompletor = languages.registerCompletionItemProvider('sql', {
				triggerCharacters: ['.', ' ', '('],
				provideCompletionItems: function (model, position) {
					const textUntilPosition = model.getValueInRange({
						startLineNumber: 1,
						startColumn: 1,
						endLineNumber: position.lineNumber,
						endColumn: position.column
					})
					const word = model.getWordUntilPosition(position)
					const range = {
						startLineNumber: position.lineNumber,
						endLineNumber: position.lineNumber,
						startColumn: word.startColumn,
						endColumn: word.endColumn
					}
					let suggestions: languages.CompletionItem[] = []
					const noneMatch = textUntilPosition.match(/(?:add|create table)\s/i)
					if (noneMatch) {
						return {
							suggestions
						}
					}
					for (const schemaKey in schema) {
						suggestions.push({
							label: schemaKey,
							detail: 'schema',
							kind: languages.CompletionItemKind.Function,
							insertText: schemaKey,
							range: range,
							sortText: 'z'
						})
						for (const tableKey in schema[schemaKey]) {
							suggestions.push({
								label: tableKey,
								detail: `table (${schemaKey})`,
								kind: languages.CompletionItemKind.Function,
								insertText: tableKey,
								range: range,
								sortText: 'y'
							})
							const noColsMatch = textUntilPosition.match(
								/(?:from|insert into|update|table)\s(?![\s\S]*(\b(where|order by|group by|values|set|column)\b|\())/i
							)
							if (!noColsMatch) {
								for (const columnKey in schema[schemaKey][tableKey]) {
									suggestions.push({
										label: columnKey,
										detail: `${schema[schemaKey][tableKey][columnKey]['type']} (${schemaKey}.${tableKey})`,
										kind: languages.CompletionItemKind.Function,
										insertText: columnKey,
										range: range,
										sortText: 'x'
									})
								}
							}
							if (textUntilPosition.match(new RegExp(`${tableKey}.$`, 'i'))) {
								suggestions = suggestions.filter((x) =>
									x.detail?.includes(`(${schemaKey}.${tableKey})`)
								)
								return {
									suggestions
								}
							}
						}
						if (textUntilPosition.match(new RegExp(`${schemaKey}.$`, 'i'))) {
							suggestions = suggestions.filter((x) => x.detail === `table (${schemaKey})`)
							return {
								suggestions
							}
						}
					}
					return {
						suggestions
					}
				}
			})
		}
	}

	let preprocessorCompletor: IDisposable | undefined = undefined
	function addPreprocessorCompletions(lang: string) {
		if (preprocessorCompletor) {
			preprocessorCompletor.dispose()
		}

		const windmillLang = lang === 'typescript' ? 'deno' : lang === 'python' ? 'python3' : lang
		const preprocessorCode = getPreprocessorModuleCode(windmillLang as ScriptLang)

		if (!preprocessorCode) {
			return
		}

		preprocessorCompletor = languages.registerCompletionItemProvider(lang, {
			provideCompletionItems: function (model, position) {
				const word = model.getWordUntilPosition(position)

				if (word.word.length >= 3 && 'preprocessor'.startsWith(word.word)) {
					const range = {
						startLineNumber: position.lineNumber,
						endLineNumber: position.lineNumber,
						startColumn: word.startColumn,
						endColumn: word.endColumn
					}
					return {
						suggestions: [
							{
								label: 'preprocessor (windmill)',
								kind: languages.CompletionItemKind.Function,
								insertTextRules: languages.CompletionItemInsertTextRule.InsertAsSnippet,
								insertText: preprocessorCode,
								range,
								additionalTextEdits: [
									{
										range: {
											startLineNumber: position.lineNumber,
											endLineNumber: position.lineNumber,
											startColumn: 0,
											endColumn: word.startColumn
										},
										text: ''
									}
								]
							}
						]
					}
				}

				return {
					suggestions: []
				}
			}
		})
	}

	let reviewingChanges = $state(writable(false))
	let aiChatEditorHandler: AIChatEditorHandler | undefined = $state(undefined)

	// Inline ai chat widget
	let showInlineAIChat = $state(false)
	let inlineAIChatSelection: Selection | null = $state(null)
	let selectedCode = $state('')

	export async function reviewAndApplyCode(code: string, opts?: ReviewChangesOpts) {
		await aiChatEditorHandler?.reviewChanges(code, opts)
	}

	export async function reviewAppliedCode(
		originalCode: string,
		opts?: { onFinishedReview?: () => void }
	) {
		await aiChatEditorHandler?.reviewChanges(originalCode, {
			mode: 'revert',
			onFinishedReview: opts?.onFinishedReview
		})
	}

	export function getAiChatEditorHandler() {
		return aiChatEditorHandler
	}

	function addChatHandler(editor: meditor.IStandaloneCodeEditor) {
		try {
			aiChatEditorHandler = new AIChatEditorHandler(editor)
			reviewingChanges = aiChatEditorHandler.reviewingChanges
		} catch (err) {
			console.error('Could not add chat handler', err)
		}
	}

	let autocompletor: Autocompletor | undefined = $state(undefined)

	function addAutoCompletor(
		editor: meditor.IStandaloneCodeEditor,
		scriptLang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json'
	) {
		if (autocompletor) {
			autocompletor.dispose()
		}
		autocompletor = new Autocompletor(editor, scriptLang)
	}

	const outputChannel = {
		name: 'Language Server Client',
		appendLine: (msg: string) => {
			console.log(msg)
		},
		append: (msg: string) => {
			console.log(msg)
		},
		clear: () => {},
		replace: () => {},
		show: () => {},
		hide: () => {},
		dispose: () => {}
	}

	export async function reloadWebsocket() {
		await closeWebsockets()

		if (
			!useWebsockets ||
			!(
				(lang == 'typescript' && scriptLang === 'deno') ||
				lang == 'python' ||
				lang == 'go' ||
				lang == 'shell'
			)
		) {
			return
		}
		console.log('reloadWebsocket')

		function createLanguageClient(
			transports: MessageTransports,
			name: string,
			initializationOptions: any,
			middlewareOptions: ((params, token, next) => any) | undefined
		) {
			const client = new MonacoLanguageClient({
				name: name,
				messageTransports: transports,
				clientOptions: {
					outputChannel,
					documentSelector: [lang],
					errorHandler: {
						error: () => ({ action: ErrorAction.Continue }),

						closed: () => ({
							action: CloseAction.Restart
						})
					},
					markdown: {
						isTrusted: true
					},
					workspaceFolder:
						name != 'deno'
							? {
									uri: vscode.Uri.parse(uri),
									name: 'windmill',
									index: 0
								}
							: undefined,
					initializationOptions,
					middleware: {
						workspace: {
							configuration:
								middlewareOptions ??
								((params, token, next) => {
									return [{ enabled: true }]
								})
						}
					}
				}
			})
			return client
		}

		async function connectToLanguageServer(
			url: string,
			name: string,
			initOptions: any,
			middlewareOptions: any
		) {
			try {
				const webSocket = new WebSocket(url)
				websockets.push(webSocket)
				webSocket.onopen = async () => {
					const socket = toSocket(webSocket)
					const reader = new WebSocketMessageReader(socket)
					const writer = new WebSocketMessageWriter(socket)
					const languageClient = createLanguageClient(
						{ reader, writer },
						name,
						initOptions,
						middlewareOptions
					)
					// if (middlewareOptions != undefined) {
					// 	languageClient.registerNotUsedFeatures()
					// }

					const om = webSocket.onmessage
					webSocket.onmessage = (e) => {
						om && om.apply(webSocket, [e])
						if (destroyed) {
							webSocket.close()
							console.log('Stopping client early because of mismatch')
						}
					}

					languageClients.push(languageClient)

					// HACK ALERT: for some reasons, the client need to be restarted to take into account the 'go get <dep>' command
					// the only way I could figure out to listen for this event is this. I'm sure there is a better way to do this
					if (name == 'go') {
						const om = webSocket.onmessage
						webSocket.onmessage = (e) => {
							om && om.apply(webSocket, [e])
							const js = JSON.parse(e.data)
							if (js.method == 'window/showMessage' && js.params.message == 'completed') {
								console.log('reloading websocket after go get')
								reloadWebsocket()
							}
						}
					}
					reader.onClose(async () => {
						try {
							console.log('CLOSE')
							websocketAlive[name] = false
							await languageClient.stop()
						} catch (err) {
							console.error(err)
						}
					})
					socket.onClose((_code, _reason) => {
						websocketAlive[name] = false
					})

					try {
						console.log('starting client')
						await languageClient.start()
						// for python we want to use the pyright client for signature help, not ruff
						if (lang !== 'python' || (lang === 'python' && name == 'pyright')) {
							autocompletor?.setLanguageClient(languageClient)
						}
						console.log('started client')
					} catch (err) {
						console.log('err at client')
						console.error(err)
						return
					}

					lastWsAttempt = new Date()
					nbWsAttempt = 0
					if (name == 'deno') {
						command && command.dispose()
						command = undefined
						try {
							command = vscode.commands.registerCommand(
								'deno.cache',
								(uris: DocumentUri[] = []) => {
									languageClient.sendRequest(new RequestType('deno/cache'), {
										referrer: { uri },
										uris: uris.map((uri) => ({ uri }))
									})
								}
							)
						} catch (err) {
							console.warn(err)
						}
					}

					websocketAlive[name] = true
				}
			} catch (err) {
				console.error(`connection to ${name} language server failed`)
			}
		}

		const wsProtocol = BROWSER && window.location.protocol == 'https:' ? 'wss' : 'ws'
		const hostname = getHostname()

		let encodedImportMap = ''

		if (useWebsockets) {
			if (lang == 'typescript' && scriptLang === 'deno') {
				ata = undefined
				let root = await genRoot(hostname)
				const importMap = {
					imports: {
						'file:///': root + '/'
					}
				}
				if (filePath && filePath.split('/').length > 2) {
					let path_splitted = filePath.split('/')
					for (let c = 0; c < path_splitted.length; c++) {
						let key = 'file://./'
						for (let i = 0; i < c; i++) {
							key += '../'
						}
						let url = path_splitted.slice(0, -c - 1).join('/')
						let ending = c == path_splitted.length - 1 ? '' : '/'
						importMap['imports'][key] = `${root}/${url}${ending}`
					}
				}
				encodedImportMap = 'data:text/plain;base64,' + btoa(JSON.stringify(importMap))
				await connectToLanguageServer(
					`${wsProtocol}://${window.location.host}/ws/deno`,
					'deno',
					{
						certificateStores: null,
						enablePaths: [],
						config: null,
						importMap: encodedImportMap,
						internalDebug: false,
						lint: false,
						path: null,
						tlsCertificate: null,
						unsafelyIgnoreCertificateErrors: null,
						unstable: true,
						enable: true,
						codeLens: {
							implementations: true,
							references: true,
							referencesAllFunction: false
						},
						suggest: {
							autoImports: true,
							completeFunctionCalls: false,
							names: true,
							paths: true,
							imports: {
								autoDiscover: true,
								hosts: {
									'https://deno.land': true
								}
							}
						}
					},
					() => {
						return [
							{
								enable: true
							}
						]
					}
				)
			} else if (lang === 'python') {
				await connectToLanguageServer(
					`${wsProtocol}://${window.location.host}/ws/pyright`,
					'pyright',
					{},
					(params, token, next) => {
						if (params.items.find((x) => x.section === 'python')) {
							return [
								{
									analysis: {
										useLibraryCodeForTypes: true,
										autoImportCompletions: true,
										diagnosticSeverityOverrides: { reportMissingImports: 'none' },
										typeCheckingMode: 'basic'
									}
								}
							]
						}
						if (params.items.find((x) => x.section === 'python.analysis')) {
							return [
								{
									useLibraryCodeForTypes: true,
									autoImportCompletions: true,
									diagnosticSeverityOverrides: { reportMissingImports: 'none' },
									typeCheckingMode: 'basic'
								}
							]
						}
						return next(params, token)
					}
				)

				connectToLanguageServer(
					`${wsProtocol}://${window.location.host}/ws/ruff`,
					'ruff',
					{},
					undefined
				)
			} else if (lang === 'go') {
				connectToLanguageServer(
					`${wsProtocol}://${window.location.host}/ws/go`,
					'go',
					{
						'build.allowImplicitNetworkAccess': true
					},
					undefined
				)
			} else if (lang === 'shell') {
				connectToLanguageServer(
					`${wsProtocol}://${window.location.host}/ws/diagnostic`,
					'shellcheck',
					{
						linters: {
							shellcheck: {
								command: 'shellcheck',
								debounce: 100,
								args: ['--format=gcc', '-'],
								offsetLine: 0,
								offsetColumn: 0,
								sourceName: 'shellcheck',
								formatLines: 1,
								formatPattern: [
									'^[^:]+:(\\d+):(\\d+):\\s+([^:]+):\\s+(.*)$',
									{
										line: 1,
										column: 2,
										message: 4,
										security: 3
									}
								],
								securities: {
									error: 'error',
									warning: 'warning',
									note: 'info'
								}
							}
						},
						filetypes: {
							shell: 'shellcheck'
						}
					},
					undefined
				)
			} else {
				closeWebsockets()
			}

			websocketInterval && clearInterval(websocketInterval)
			websocketInterval = setInterval(() => {
				if (document.visibilityState == 'visible') {
					if (
						!lastWsAttempt ||
						(new Date().getTime() - lastWsAttempt.getTime() > 60000 && nbWsAttempt < 2)
					) {
						if (
							!websocketAlive.deno &&
							!websocketAlive.pyright &&
							!websocketAlive.go &&
							!websocketAlive.shellcheck &&
							!websocketAlive.ruff &&
							scriptLang != 'bun' &&
							scriptLang != 'tsx'
						) {
							console.log('reconnecting to language servers')
							lastWsAttempt = new Date()
							nbWsAttempt++
							reloadWebsocket()
						} else {
							if (nbWsAttempt >= 2) {
								sendUserToast('Giving up on establishing smart assistant connection', true)
								clearInterval(websocketInterval)
							}
						}
					}
				}
			}, 5000)
		}
	}

	let pathTimeout: number | undefined = undefined

	let yPadding = MONACO_Y_PADDING

	function getHostname() {
		return BROWSER ? window.location.protocol + '//' + window.location.host : 'SSR'
	}

	function handlePathChange() {
		console.log('path changed, reloading language server', initialPath, path)
		initialPath = path
		pathTimeout && clearTimeout(pathTimeout)
		ata = undefined
		pathTimeout = setTimeout(reloadWebsocket, 1000)
	}

	async function closeWebsockets() {
		command && command.dispose()
		command = undefined

		console.debug(`disposing ${websockets.length} language clients and closing websockets`)
		for (const x of languageClients) {
			try {
				await x.dispose()
			} catch (err) {
				console.debug('error disposing language client', err)
			}
		}
		languageClients = []

		for (const x of websockets) {
			try {
				await x.close()
			} catch (err) {
				console.debug('error closing websocket', err)
			}
		}

		console.debug('done closing websockets')
		websockets = []
		websocketInterval && clearInterval(websocketInterval)
	}

	// let widgets: HTMLElement | undefined = document.getElementById('monaco-widgets-root') ?? undefined
	let model: meditor.ITextModel | undefined = $state(undefined)

	let monacoBinding: MonacoBinding | undefined = $state(undefined)

	let initialized = $state(false)
	let ata: ((s: string | DepsToGet) => void) | undefined = undefined

	let statusDiv: Element | null = $state(null)

	function saveDraft() {
		dispatch('saveDraft', code)
	}

	let vimDisposable: IDisposable | undefined = $state(undefined)

	function onVimDisable() {
		vimDisposable?.dispose()
	}

	function onVimMode() {
		if (editor && statusDiv) {
			vimDisposable = initVim(editor, statusDiv, saveDraft)
		}
	}

	let svelteRegistered = false
	let vueRegistered = false
	function onFileChanges() {
		if (files && Object.keys(files).find((x) => x.endsWith('.svelte')) != undefined) {
			if (!svelteRegistered) {
				svelteRegistered = true
				languages.register({
					id: 'svelte',
					extensions: ['.svelte'],
					aliases: ['Svelte', 'svelte'],
					mimetypes: ['application/svelte']
				})
				languages.setLanguageConfiguration('svelte', htmllang.conf as any)

				languages.setMonarchTokensProvider('svelte', htmllang.language as any)
			}
		}

		if (files && Object.keys(files).find((x) => x.endsWith('.vue')) != undefined) {
			if (!vueRegistered) {
				vueRegistered = true
				languages.register({
					id: 'vue',
					extensions: ['.vue'],
					aliases: ['Vue', 'Vue'],
					mimetypes: ['application/svelte']
				})
				languages.setLanguageConfiguration('vue', conf as any)

				languages.setMonarchTokensProvider('vue', language as any)
			}
		}

		if (files && model) {
			for (const [path, { code, readonly }] of Object.entries(files)) {
				const luri = mUri.file(path)
				if (luri.toString() != model.uri.toString()) {
					let nmodel = meditor.getModel(luri)

					if (nmodel == undefined) {
						const lmodel = meditor.createModel(code, extToLang(path?.split('.')?.pop()!), luri)
						if (readonly) {
							lmodel.onDidChangeContent((evt) => {
								// This will effectively undo any new edits
								if (lmodel.getValue() != code && code) {
									lmodel.setValue(code)
								}
							})
						}
					} else {
						const lmodel = meditor.getModel(luri)
						if (lmodel && code) {
							lmodel.setValue(code)
						}
					}
				}
			}
		}
	}

	let timeoutModel: number | undefined = undefined
	async function loadMonaco() {
		setMonacoTypescriptOptions()
		console.log('path', uri)

		try {
			console.log("Loading Monaco's language client")
			await initializeVscode('editor', divEl!)
			console.log('done loading Monaco and vscode')
		} catch (e) {
			console.log('error initializing services', e)
		}

		// vscode.languages.registerDefinitionProvider('*', {
		// 	provideDefinition(document, position, token) {
		// 		// Get the word under the cursor (this will be the import or function being clicked)
		// 		const wordRange = document.getWordRangeAtPosition(position)
		// 		const word = document.getText(wordRange)

		// 		// Do something with the word (for example, log it or handle it)
		// 		console.log('Clicked on import or symbol:', word)

		// 		// Optionally, you can also return a definition location
		// 		return null // If you don't want to override the default behavior
		// 	}
		// })
		// console.log('bef ready')
		// console.log('af ready')

		initialized = true

		try {
			model = meditor.createModel(code ?? '', lang == 'nu' ? 'python' : lang, mUri.parse(uri))
		} catch (err) {
			console.log('model already existed', err)
			const nmodel = meditor.getModel(mUri.parse(uri))
			if (!nmodel) {
				throw err
			}
			model = nmodel
		}
		model.updateOptions(lang == 'python' ? { tabSize: 4, insertSpaces: true } : updateOptions)

		onFileChanges()

		try {
			editor = meditor.create(divEl as HTMLDivElement, {
				...editorConfig(
					code ?? '',
					lang,
					automaticLayout,
					fixedOverflowWidgets,
					$relativeLineNumbers
				),
				model,
				fontSize: !small ? 13.5 : 12,
				lineNumbersMinChars,
				// overflowWidgetsDomNode: widgets,
				tabSize: lang == 'python' ? 4 : 2,
				folding,
				padding: { bottom: yPadding, top: yPadding }
			})
			if (key && editorPositionMap?.[key]) {
				editor.setPosition(editorPositionMap[key])
				editor.revealPositionInCenterIfOutsideViewport(editorPositionMap[key])
			}
		} catch (e) {
			console.error('Error loading monaco:', e)
			return
		}

		keepModelAroundToAvoidDisposalOfWorkers()

		// In VSCode webview (iframe), clipboard operations need special handling
		// because the webview has restricted clipboard API access
		if (window.parent !== window) {
			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyC, function () {
				document.execCommand('copy')
			})
			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyX, function () {
				document.execCommand('cut')
			})
			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyV, async function () {
				try {
					// Use Clipboard API to read text, then insert via Monaco's API
					const text = await navigator.clipboard.readText()
					if (text && editor) {
						const selection = editor.getSelection()
						if (selection) {
							editor.executeEdits('paste', [
								{
									range: selection,
									text: text,
									forceMoveMarkers: true
								}
							])
						}
					}
				} catch (e) {
					// Clipboard API failed, try execCommand as fallback
					document.execCommand('paste')
				}
			})
		}

		// updateEditorKeybindingsMode(editor, 'vim', undefined)

		// Raw app lint collection: listen for marker changes and report to store
		let markerChangeDisposable: IDisposable | undefined = undefined
		if (rawAppRunnableKey && model) {
			markerChangeDisposable = meditor.onDidChangeMarkers((uris) => {
				if (!model || !rawAppRunnableKey) return
				const modelUri = model.uri.toString()
				if (uris.some((u) => u.toString() === modelUri)) {
					updateRawAppLintDiagnostics()
				}
			})
			// Initial lint diagnostics collection
			updateRawAppLintDiagnostics()
		}

		let ataModel: number | undefined = undefined

		editor?.onDidChangeModelContent((event) => {
			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				updateCode()
			}, changeTimeout)

			ataModel && clearTimeout(ataModel)
			ataModel = setTimeout(() => {
				if (scriptLang == 'bun' || scriptLang == 'bunnative') {
					ata?.(getCode())
				}
			}, 1000)
		})

		editor?.onDidBlurEditorText(() => {
			dispatch('blur')
		})

		editor?.onDidChangeCursorPosition((event) => {
			if (key) editorPositionMap[key] = event.position
		})

		editor?.onDidFocusEditorText(() => {
			dispatch('focus')

			// for escape we use onkeydown instead of addCommand because addCommand on escape specifically prevents default behavior (like autocomplete cancellation)
			editor?.onKeyDown((e) => {
				if (e.keyCode === KeyCode.Escape) {
					if (showInlineAIChat) {
						closeAIInlineWidget()
					}
					aiChatEditorHandler?.rejectAll()
				}
			})

			editor?.addCommand(KeyMod.CtrlCmd | KeyCode.DownArrow, function () {
				if (aiChatManager.pendingNewCode) {
					aiChatManager.scriptEditorApplyCode?.(aiChatManager.pendingNewCode)
					if (showInlineAIChat) {
						closeAIInlineWidget()
					}
				}
			})

			editor?.addCommand(KeyMod.CtrlCmd | KeyCode.KeyS, function () {
				updateCode()
				shouldBindKey && format && format()
			})

			editor?.addCommand(KeyMod.CtrlCmd | KeyCode.Enter, function () {
				updateCode()
				shouldBindKey && cmdEnterAction && cmdEnterAction()
			})

			editor?.addCommand(KeyMod.CtrlCmd | KeyMod.Shift | KeyCode.Digit7, function () {
				// CMD + slash (toggle comment) on some EU keyboards
				editor?.trigger('keyboard', 'editor.action.commentLine', {})
			})

			editor?.addCommand(KeyMod.CtrlCmd | KeyCode.KeyL, function () {
				const selectedLines = getSelectedLines()
				const selection = editor?.getSelection()
				const hasSelection =
					selection &&
					(selection.startLineNumber !== selection.endLineNumber ||
						selection.startColumn !== selection.endColumn)
				if (hasSelection && selectedLines) {
					aiChatManager.addSelectedLinesToContext(
						selectedLines,
						selection.startLineNumber,
						selection.endLineNumber,
						moduleId
					)
				} else {
					aiChatManager.toggleOpen()
					aiChatManager.focusInput()
				}
			})

			editor?.addCommand(KeyMod.CtrlCmd | KeyCode.KeyK, function () {
				if ($copilotInfo.enabled) {
					aiChatEditorHandler?.rejectAll()
					if (showInlineAIChat) {
						closeAIInlineWidget()
					} else {
						showAIInlineWidget()
					}
				}
			})

			editor?.addCommand(KeyMod.CtrlCmd | KeyCode.KeyU, function () {
				dispatch('toggleTestPanel')
			})

			if (
				!websocketAlive.deno &&
				!websocketAlive.pyright &&
				!websocketAlive.ruff &&
				!websocketAlive.shellcheck &&
				!websocketAlive.go &&
				!websocketInterval
			) {
				reloadWebsocket()
			}
		})

		reloadWebsocket()

		setTypescriptExtraLibs()
		setTypescriptRTNamespace()
		return () => {
			console.log('disposing editor')
			ata = undefined
			try {
				closeWebsockets()
				vimDisposable?.dispose()
				closeAIInlineWidget()
				markerChangeDisposable?.dispose()
				// Note: We don't clear lint diagnostics on dispose - they persist across runnable switches
				// Diagnostics are only updated when Monaco reports new markers for this runnable
				console.log('disposing editor')
				model?.dispose()
				editor && editor.dispose()
				console.log('disposed editor')
			} catch (err) {
				console.log('error disposing editor', err)
			}
		}
	}

	export async function fetchPackageDeps(deps: DepsToGet) {
		ata?.(deps)
	}

	let customTsTypesData = resource([() => lang], async () => {
		if (lang !== 'typescript') return undefined
		let datatables = await WorkspaceService.listDataTables({ workspace: $workspaceStore ?? '' })
		let ducklakes = await WorkspaceService.listDucklakes({ workspace: $workspaceStore ?? '' })
		return { datatables, ducklakes }
	})
	function setTypescriptCustomTypes() {
		if (!customTsTypesData.current) return
		if (lang !== 'typescript') return

		const ducklakeNames = customTsTypesData.current.ducklakes
		const datatableNames = customTsTypesData.current.datatables

		const ducklakeNameType = ducklakeNames.length
			? ducklakeNames.map((name) => JSON.stringify(name)).join(' | ')
			: 'string'
		const datatableNameType = datatableNames.length
			? datatableNames.map((name) => JSON.stringify(name)).join(' | ')
			: 'string'
		const isDucklakeOptional = ducklakeNames.includes('main')
		const isDataTableOptional = datatableNames.includes('main')

		let disposeTs = typescriptDefaults.addExtraLib(
			`export {};
			declare module 'windmill-client' {
				import { type DatatableSqlTemplateFunction, type SqlTemplateFunction } from 'windmill-client';
				export function ducklake(name${isDucklakeOptional ? '?' : ''}: ${ducklakeNameType}): SqlTemplateFunction;
				export function datatable(name${isDataTableOptional ? '?' : ''}: ${datatableNameType}): DatatableSqlTemplateFunction;
			}`,
			'file:///custom_wmill_types.d.ts'
		)
		return () => {
			disposeTs.dispose()
		}
	}

	async function setTypescriptRTNamespace() {
		if (
			scriptLang &&
			(scriptLang === 'bun' ||
				scriptLang === 'deno' ||
				scriptLang === 'bunnative' ||
				scriptLang === 'nativets')
		) {
			const resourceTypes = await ResourceService.listResourceType({
				workspace: $workspaceStore ?? ''
			})

			const namespace = formatResourceTypes(
				resourceTypes,
				scriptLang === 'bunnative' ? 'bun' : scriptLang
			)

			typescriptDefaults.addExtraLib(namespace, 'rt.d.ts')
		}
	}

	async function setTypescriptExtraLibs() {
		if (extraLib) {
			const uri = mUri.parse('file:///extraLib.d.ts')
			typescriptDefaults.addExtraLib(extraLib, uri.toString())
		}

		if (
			lang === 'typescript' &&
			(scriptLang == 'bun' || scriptLang == 'tsx' || scriptLang == 'bunnative') &&
			ata == undefined
		) {
			const hostname = getHostname()

			const addLibraryToRuntime = async (code: string, _path: string) => {
				const path = 'file://' + _path
				let uri = mUri.parse(path)
				console.log('adding library to runtime', path)
				typescriptDefaults.addExtraLib(code, path)
				try {
					await vscode.workspace.fs.writeFile(uri, new TextEncoder().encode(code))
				} catch (e) {
					console.log('error writing file', e)
				}
			}

			const addLocalFile = async (code: string, _path: string) => {
				let p = new URL(_path, uri).href
				// if (_path?.startsWith('/')) {
				// 	p = 'file://' + p
				// }
				let nuri = mUri.parse(p)
				console.log('adding local file', _path, nuri.toString())
				if (editor) {
					let localModel = meditor.getModel(nuri)
					if (localModel) {
						localModel.setValue(code)
					} else {
						meditor.createModel(code, 'typescript', nuri)
					}
					try {
						if (model) {
							model?.setValue(model.getValue())
						}
					} catch (e) {
						console.log('error resetting model', e)
					}
				}
			}
			await initWasmTs()
			const root = await genRoot(hostname)
			console.log('SETUP TYPE ACQUISITION', { root, path })
			ata = setupTypeAcquisition({
				projectName: 'Windmill',
				depsParser: (c) => {
					return parseTypescriptDeps(c)
				},
				root,
				scriptPath: path,
				logger: console,
				delegate: {
					receivedFile: addLibraryToRuntime,
					localFile: addLocalFile,
					progress: (downloaded: number, total: number) => {
						// console.log({ dl, ttl })
					},
					started: () => {
						console.log('ATA start')
					},
					finished: (f) => {
						console.log('ATA done')
					}
				}
			})
			if (scriptLang == 'bun') {
				ata?.('import "bun-types"')
			}
			if (scriptLang == 'bunnative' || scriptLang == 'bun') {
				ata?.(code ?? '')
			}
			dispatch('ataReady')
		}
	}

	export function addAction(
		id: string,
		label: string,
		callback: (editor: meditor.IStandaloneCodeEditor) => void,
		keybindings: number[] = []
	) {
		editor?.addAction({
			id,
			label,
			keybindings,
			contextMenuGroupId: 'navigation',
			run: function (editor: meditor.IStandaloneCodeEditor) {
				callback(editor)
			}
		})
	}

	function showAIInlineWidget() {
		if (!editor) return

		inlineAIChatSelection = editor.getSelection()
		if (!inlineAIChatSelection) {
			return
		}
		const model = editor.getModel()
		selectedCode = ''
		if (model) {
			selectedCode = model.getValueInRange(inlineAIChatSelection)
		}
		showInlineAIChat = true
		aiChatInlineWidget?.focusInput()
	}

	function closeAIInlineWidget() {
		showInlineAIChat = false
		inlineAIChatSelection = null
		selectedCode = ''
	}

	let aiChatInlineWidget: AIChatInlineWidget | null = $state(null)

	let loadTimeout: number | undefined = undefined
	onMount(async () => {
		if (BROWSER) {
			if (loadAsync) {
				loadTimeout = setTimeout(() => loadMonaco().then((x) => (disposeMethod = x)), 0)
			} else {
				let m = await loadMonaco()
				disposeMethod = m
			}
		}
	})

	onDestroy(() => {
		console.log('destroying editor')
		valueAfterDispose = getCode()
		destroyed = true
		disposeMethod && disposeMethod()
		websocketInterval && clearInterval(websocketInterval)
		sqlSchemaCompletor && sqlSchemaCompletor.dispose()
		autocompletor && autocompletor.dispose()
		sqlTypeCompletor && sqlTypeCompletor.dispose()
		resultCollectionCompletor && resultCollectionCompletor.dispose()
		preprocessorCompletor && preprocessorCompletor.dispose()
		timeoutModel && clearTimeout(timeoutModel)
		loadTimeout && clearTimeout(loadTimeout)
		aiChatEditorHandler?.clear()
	})

	async function genRoot(hostname: string) {
		let token = $lspTokenStore
		if (!token) {
			let expiration = new Date()
			expiration.setHours(expiration.getHours() + 72)
			const newToken = await UserService.createToken({
				requestBody: { label: 'Ephemeral lsp token', expiration: expiration.toISOString() }
			})
			$lspTokenStore = newToken
			token = newToken
		}
		let root = hostname + '/api/scripts_u/tokened_raw/' + $workspaceStore + '/' + token
		return root
	}

	function acceptCodeChanges() {
		const mode = aiChatEditorHandler?.getReviewMode?.()
		if (mode === 'revert') {
			aiChatEditorHandler?.keepAll()
		} else {
			aiChatEditorHandler?.acceptAll()
		}
	}

	function rejectCodeChanges() {
		const mode = aiChatEditorHandler?.getReviewMode?.()
		if (mode === 'revert') {
			aiChatEditorHandler?.revertAll()
		} else {
			aiChatEditorHandler?.rejectAll()
		}
	}

	function onKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			if (showInlineAIChat) {
				closeAIInlineWidget()
			}
			rejectCodeChanges()
		} else if ((e.ctrlKey || e.metaKey) && e.key === 'ArrowDown' && aiChatManager.pendingNewCode) {
			acceptCodeChanges()
			if (showInlineAIChat) {
				closeAIInlineWidget()
			}
		}
	}
	$effect(() => {
		lang = scriptLangToEditorLang(scriptLang)
	})
	$effect(() => {
		filePath = computePath(path)
	})
	$effect(() => {
		path != initialPath &&
			(scriptLang == 'deno' || scriptLang == 'bun' || scriptLang == 'bunnative') &&
			untrack(() => {
				handlePathChange()
			})
	})
	$effect(() => {
		initialized && lang === 'sql' && scriptLang
			? untrack(() => addSqlTypeCompletions())
			: (sqlTypeCompletor?.dispose(), resultCollectionCompletor?.dispose())
	})

	$effect(() => {
		initialized && canHavePreprocessor(lang) && enablePreprocessorSnippet
			? untrack(() => addPreprocessorCompletions(lang))
			: preprocessorCompletor?.dispose()
	})

	let lastArg = undefined
	$effect(() => {
		let newArg = lang === 'graphql' ? args?.api : args?.database
		if (newArg !== lastArg) {
			lastArg = newArg
			$dbSchemas && untrack(() => updateSchema(newArg))
		}
	})
	$effect(() => {
		console.log('updating db schema completions', dbSchema, lang)
		initialized &&
			dbSchema &&
			['sql', 'graphql'].includes(lang) &&
			untrack(() => addDBSchemaCompletions())
	})
	$effect(() => {
		;(!dbSchema || lang !== 'sql') && untrack(() => disposeSqlSchemaCompletor())
	})
	$effect(() => {
		;(!dbSchema || lang !== 'graphql') && untrack(() => disposeGaphqlService())
	})
	$effect(() => {
		$copilotInfo.enabled &&
			$codeCompletionSessionEnabled &&
			Autocompletor.isProviderModelSupported($copilotInfo.codeCompletionModel) &&
			initialized &&
			editor &&
			scriptLang &&
			untrack(() => editor && addAutoCompletor(editor, scriptLang))
	})
	$effect(() => {
		$copilotInfo.enabled && initialized && editor && untrack(() => editor && addChatHandler(editor))
	})
	$effect(() => {
		!$codeCompletionSessionEnabled && autocompletor?.dispose()
	})
	$effect(() => {
		if (yContent && awareness && model && editor) {
			untrack(() => {
				monacoBinding && monacoBinding.destroy()
				monacoBinding = new MonacoBinding(
					yContent,
					model!,
					new Set([editor as meditor.IStandaloneCodeEditor]),
					awareness
				)
			})
		}
	})
	$effect(() => {
		editor && $vimMode && statusDiv && untrack(() => onVimMode())
	})
	$effect(() => {
		!$vimMode && vimDisposable && untrack(() => onVimDisable())
	})
	$effect(() => {
		files && model && untrack(() => onFileChanges())
	})
	$effect(() => {
		editor?.updateOptions({
			lineNumbers: $relativeLineNumbers ? 'relative' : 'on'
		})
	})

	let isTsWorkerInitialized = resource(
		[() => lang, () => initialized, () => filePath],
		async () => {
			if (lang !== 'typescript' || !initialized) return false
			console.log('[Editor.isTsWorkerInitialized] Waiting for TS Worker...')
			await waitForWorkerInitialization(filePath)
			console.log('[Editor.isTsWorkerInitialized] TS Worker initialized successfully')
			return true
		}
	)

	// Update SQL query type information in the TypeScript worker
	// This enables TypeScript to show proper types for SQL template literals
	let handleSqlTypingInTs = useDebounce(function handleSqlTypingInTs() {
		if (lang !== 'typescript' || !isTsWorkerInitialized.current) return
		if (!preparedAssetsSqlQueries || preparedAssetsSqlQueries.length === 0) {
			// Clear SQL queries if none exist
			updateSqlQueriesInWorker(filePath, [])
			return
		}

		// Send SQL query information to the custom TypeScript worker
		// The worker will inject type parameters into the code that TypeScript analyzes

		// Worker async function call freezes if we pass a Proxy, $state.snapshot() is very important here
		updateSqlQueriesInWorker(filePath, $state.snapshot(preparedAssetsSqlQueries))
	}, 250)

	watch(
		[
			() => preparedAssetsSqlQueries,
			() => lang,
			() => filePath,
			() => isTsWorkerInitialized.current
		],
		() => {
			handleSqlTypingInTs()
		}
	)

	watch([() => customTsTypesData.current], setTypescriptCustomTypes)
</script>

<svelte:window onkeydown={onKeyDown} />
<EditorTheme />
{#if !editor}
	<div class="inset-0 absolute overflow-clip">
		<FakeMonacoPlaceHolder {code} lineNumbersWidth={51} />
	</div>
{/if}
<div bind:this={divEl} class="{clazz} editor {disabled ? 'disabled' : ''}"></div>
{#if $vimMode}
	<div class="fixed bottom-0 z-30" bind:this={statusDiv}></div>
{/if}

{#if $reviewingChanges}
	<GlobalReviewButtons onAcceptAll={acceptCodeChanges} onRejectAll={rejectCodeChanges} />
{/if}

{#if editor && $copilotInfo.enabled && aiChatEditorHandler}
	<AIChatInlineWidget
		bind:this={aiChatInlineWidget}
		bind:show={showInlineAIChat}
		{editor}
		editorHandler={aiChatEditorHandler}
		selection={inlineAIChatSelection}
		{selectedCode}
	/>
{/if}

<style global lang="postcss">
	.editor {
		@apply p-0;
	}
	.yRemoteSelection {
		background-color: rgb(250, 129, 0, 0.5);
	}
	.yRemoteSelectionHead {
		position: absolute;
		border-left: orange solid 2px;
		border-top: orange solid 2px;
		border-bottom: orange solid 2px;
		height: 100%;
		box-sizing: border-box;
	}
	.yRemoteSelectionHead::after {
		position: absolute;
		content: ' ';
		border: 3px solid orange;
		border-radius: 4px;
		left: -4px;
		top: -5px;
	}
</style>
