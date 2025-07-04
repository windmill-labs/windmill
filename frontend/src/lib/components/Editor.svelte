<!-- <script lang="ts"></script> -->

<script context="module">
	import '@codingame/monaco-vscode-standalone-languages'
	import '@codingame/monaco-vscode-standalone-typescript-language-features'
	import processStdContent from '$lib/process.d.ts.txt?raw'

	languages.typescript.typescriptDefaults.addExtraLib(processStdContent, 'process.d.ts')

	languages.typescript.typescriptDefaults.setModeConfiguration({
		completionItems: true,
		hovers: true,
		documentSymbols: true,
		definitions: true,
		references: true,
		documentHighlights: true,
		rename: true,
		diagnostics: true,
		documentRangeFormattingEdits: true,
		signatureHelp: true,
		onTypeFormattingEdits: true,
		codeActions: true,
		inlayHints: true
	})

	// languages.typescript.javascriptDefaults.setEagerModelSync(true)
	languages.typescript.typescriptDefaults.setEagerModelSync(true)

	// languages.typescript.javascriptDefaults.setDiagnosticsOptions({
	// 	noSemanticValidation: false,
	// 	noSyntaxValidation: false,
	// 	noSuggestionDiagnostics: false,
	// 	diagnosticCodesToIgnore: [1108]
	// })

	languages.typescript.typescriptDefaults.setDiagnosticsOptions({
		noSemanticValidation: false,
		noSyntaxValidation: false,

		noSuggestionDiagnostics: false,
		diagnosticCodesToIgnore: [1108, 7006, 7034, 7019, 7005]
	})

	languages.typescript.typescriptDefaults.setCompilerOptions({
		target: languages.typescript.ScriptTarget.Latest,
		allowNonTsExtensions: true,
		noSemanticValidation: false,
		noSyntaxValidation: false,
		completionItems: true,
		hovers: true,
		documentSymbols: true,
		definitions: true,
		references: true,
		documentHighlights: true,
		rename: true,
		diagnostics: true,
		documentRangeFormattingEdits: true,
		signatureHelp: true,
		onTypeFormattingEdits: true,
		codeActions: true,
		inlayHints: true,
		checkJs: true,
		allowJs: true,
		noUnusedLocals: true,
		strict: true,
		noLib: false,
		allowImportingTsExtensions: true,
		allowSyntheticDefaultImports: true,
		moduleResolution: languages.typescript.ModuleResolutionKind.NodeJs,
		jsx: languages.typescript.JsxEmit.React
	})
</script>

<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { sendUserToast } from '$lib/toast'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'

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
		copilotInfo,
		codeCompletionSessionEnabled,
		lspTokenStore,
		formatOnSave,
		vimMode
	} from '$lib/stores'

	import { editorConfig, updateOptions } from '$lib/editorUtils'
	import { createHash as randomHash } from '$lib/editorLangUtils'
	import { workspaceStore } from '$lib/stores'
	import { type Preview, ResourceService, type ScriptLang, UserService } from '$lib/gen'
	import type { Text } from 'yjs'
	import { initializeVscode, keepModelAroundToAvoidDisposalOfWorkers } from '$lib/components/vscode'

	import { initializeMode } from 'monaco-graphql/esm/initializeMode.js'
	import type { MonacoGraphQLAPI } from 'monaco-graphql/esm/api.js'

	import {
		editor as meditor,
		languages,
		KeyCode,
		KeyMod,
		Uri as mUri,
		type IRange,
		type IDisposable,
		type IPosition
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
	import { initWasmTs } from '$lib/infer'
	import { initVim } from './monaco_keybindings'
	import { parseTypescriptDeps } from '$lib/relative_imports'

	import { scriptLangToEditorLang } from '$lib/scripts'
	import * as htmllang from '$lib/svelteMonarch'
	import { conf, language } from '$lib/vueMonarch'

	import { Autocompletor } from './copilot/autocomplete/Autocompletor'
	import { AIChatEditorHandler } from './copilot/chat/monaco-adapter'
	import GlobalReviewButtons from './copilot/chat/GlobalReviewButtons.svelte'
	import AIChatInlineWidget from './copilot/chat/AIChatInlineWidget.svelte'
	import { writable } from 'svelte/store'
	import { formatResourceTypes } from './copilot/chat/script/core'
	import FakeMonacoPlaceHolder from './FakeMonacoPlaceHolder.svelte'
	import { editorPositionMap } from '$lib/utils'
	import { extToLang, langToExt } from '$lib/editorLangUtils'
	// import EditorTheme from './EditorTheme.svelte'

	let divEl: HTMLDivElement | null = null
	let editor: meditor.IStandaloneCodeEditor | null = null

	// for related places search: ADD_NEW_LANG
	export let code: string = ''
	export let cmdEnterAction: (() => void) | undefined = undefined
	export let formatAction: (() => void) | undefined = undefined
	export let automaticLayout = true
	export let websocketAlive = {
		pyright: false,
		ruff: false,
		deno: false,
		go: false,
		shellcheck: false
	}
	export let shouldBindKey: boolean = true
	export let fixedOverflowWidgets = true
	export let path: string | undefined = undefined
	export let yContent: Text | undefined = undefined
	export let awareness: any | undefined = undefined
	export let folding = false
	export let args: Record<string, any> | undefined = undefined
	export let useWebsockets: boolean = true
	export let small = false
	export let scriptLang: Preview['language'] | 'bunnative' | 'tsx' | 'jsx' | 'json' | undefined
	export let disabled: boolean = false
	export let lineNumbersMinChars = 3
	export let files: Record<string, { code: string; readonly?: boolean }> | undefined = {}
	export let extraLib: string | undefined = undefined
	export let changeTimeout: number = 500
	export let loadAsync = false
	export let key: string | undefined = undefined

	let lang = scriptLangToEditorLang(scriptLang)
	$: lang = scriptLangToEditorLang(scriptLang)

	let filePath = computePath(path)
	$: filePath = computePath(path)

	let initialPath: string | undefined = path

	$: path != initialPath &&
		(scriptLang == 'deno' || scriptLang == 'bun' || scriptLang == 'bunnative') &&
		handlePathChange()

	let websockets: WebSocket[] = []
	let languageClients: MonacoLanguageClient[] = []
	let websocketInterval: NodeJS.Timeout | undefined
	let lastWsAttempt: Date = new Date()
	let nbWsAttempt = 0
	let disposeMethod: (() => void) | undefined
	const dispatch = createEventDispatcher()
	let graphqlService: MonacoGraphQLAPI | undefined = undefined

	let dbSchema: DBSchema | undefined = undefined

	let destroyed = false
	const uri = computeUri(filePath, scriptLang)

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
			console.log('path', path)
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

	let command: IDisposable | undefined = undefined

	let sqlTypeCompletor: IDisposable | undefined = undefined

	$: initialized && lang === 'sql' && scriptLang
		? addSqlTypeCompletions()
		: sqlTypeCompletor?.dispose()

	function addSqlTypeCompletions() {
		if (sqlTypeCompletor) {
			sqlTypeCompletor.dispose()
		}
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

	function updateSchema() {
		const newSchemaRes = lang === 'graphql' ? args?.api : args?.database
		if (typeof newSchemaRes === 'string') {
			dbSchema = $dbSchemas[newSchemaRes.replace('$res:', '')]
		} else {
			dbSchema = undefined
		}
	}

	$: lang && args && $dbSchemas && updateSchema()
	$: initialized && dbSchema && ['sql', 'graphql'].includes(lang) && addDBSchemaCompletions()

	function disposeSqlSchemaCompletor() {
		sqlSchemaCompletor?.dispose()
	}
	$: (!dbSchema || lang !== 'sql') && disposeSqlSchemaCompletor()
	function disposeGaphqlService() {
		graphqlService = undefined
	}
	$: (!dbSchema || lang !== 'graphql') && disposeGaphqlService()

	function addDBSchemaCompletions() {
		const { lang: schemaLang, schema } = dbSchema || {}
		if (!schemaLang || !schema) {
			return
		}
		if (schemaLang === 'graphql') {
			graphqlService ||= initializeMode()
			graphqlService?.setSchemaConfig([
				{
					uri: 'my-schema.graphql',
					introspectionJSON: schema
				}
			])
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

	let reviewingChanges = writable(false)
	let aiChatEditorHandler: AIChatEditorHandler | undefined = undefined

	// Inline ai chat widget
	let showInlineAIChat = false
	let aiWidgetPosition: IPosition = { lineNumber: 1, column: 1 }
	let selectedCode = ''

	export function reviewAndApplyCode(code: string) {
		aiChatEditorHandler?.reviewAndApply(code)
	}

	function addChatHandler(editor: meditor.IStandaloneCodeEditor) {
		try {
			aiChatEditorHandler = new AIChatEditorHandler(editor)
			reviewingChanges = aiChatEditorHandler.reviewingChanges
		} catch (err) {
			console.error('Could not add chat handler', err)
		}
	}

	let autocompletor: Autocompletor | undefined = undefined

	function addAutoCompletor(
		editor: meditor.IStandaloneCodeEditor,
		scriptLang: ScriptLang | 'bunnative' | 'jsx' | 'tsx' | 'json'
	) {
		if (autocompletor) {
			autocompletor.dispose()
		}
		autocompletor = new Autocompletor(editor, scriptLang)
	}

	$: $copilotInfo.enabled &&
		$codeCompletionSessionEnabled &&
		Autocompletor.isProviderModelSupported($copilotInfo.codeCompletionModel) &&
		initialized &&
		editor &&
		scriptLang &&
		addAutoCompletor(editor, scriptLang)

	$: $copilotInfo.enabled && initialized && editor && addChatHandler(editor)

	$: !$codeCompletionSessionEnabled && autocompletor?.dispose()

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

	let pathTimeout: NodeJS.Timeout | undefined = undefined

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
	let model: meditor.ITextModel | undefined = undefined

	let monacoBinding: MonacoBinding | undefined = undefined

	$: if (yContent && awareness && model && editor) {
		monacoBinding && monacoBinding.destroy()
		monacoBinding = new MonacoBinding(
			yContent,
			model!,
			new Set([editor as meditor.IStandaloneCodeEditor]),
			awareness
		)
	}

	let initialized = false
	let ata: ((s: string | DepsToGet) => void) | undefined = undefined

	let statusDiv: Element | null = null

	function saveDraft() {
		dispatch('saveDraft', code)
	}

	let vimDisposable: IDisposable | undefined = undefined
	$: editor && $vimMode && statusDiv && onVimMode()
	$: !$vimMode && vimDisposable && onVimDisable()

	function onVimDisable() {
		vimDisposable?.dispose()
	}

	function onVimMode() {
		if (editor && statusDiv) {
			vimDisposable = initVim(editor, statusDiv, saveDraft)
		}
	}

	$: files && model && onFileChanges()

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

	let timeoutModel: NodeJS.Timeout | undefined = undefined
	async function loadMonaco() {
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
			model = meditor.createModel(code, lang == 'nu' ? 'python' : lang, mUri.parse(uri))
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
				...editorConfig(code, lang, automaticLayout, fixedOverflowWidgets),
				model,
				fontSize: !small ? 14 : 12,
				lineNumbersMinChars,
				// overflowWidgetsDomNode: widgets,
				tabSize: lang == 'python' ? 4 : 2,
				folding
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

		// updateEditorKeybindingsMode(editor, 'vim', undefined)

		let ataModel: NodeJS.Timeout | undefined = undefined

		editor?.onDidChangeModelContent((event) => {
			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				updateCode()
			}, changeTimeout)

			ataModel && clearTimeout(ataModel)
			ataModel = setTimeout(() => {
				if (scriptLang == 'bun') {
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
					dispatch('addSelectedLinesToAiChat', {
						lines: selectedLines,
						startLine: selection.startLineNumber,
						endLine: selection.endLineNumber
					})
				} else {
					dispatch('toggleAiPanel')
				}
			})

			editor?.addCommand(KeyMod.CtrlCmd | KeyCode.KeyK, function () {
				showAIInlineWidget()
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

			languages.typescript.typescriptDefaults.addExtraLib(namespace, 'rt.d.ts')
		}
	}

	async function setTypescriptExtraLibs() {
		if (extraLib) {
			const uri = mUri.parse('file:///extraLib.d.ts')
			languages.typescript.typescriptDefaults.addExtraLib(extraLib, uri.toString())
		}
		if (lang === 'typescript' && (scriptLang == 'bun' || scriptLang == 'tsx') && ata == undefined) {
			const hostname = getHostname()

			const addLibraryToRuntime = async (code: string, _path: string) => {
				const path = 'file://' + _path
				let uri = mUri.parse(path)
				console.log('adding library to runtime', path)
				languages.typescript.typescriptDefaults.addExtraLib(code, path)
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
				ata?.(code)
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

		const selection = editor.getSelection()
		if (!selection || selection.isEmpty()) {
			return
		}

		selectedCode = editor.getModel()?.getValueInRange(selection) || ''
		const startPos = selection.getStartPosition()
		aiWidgetPosition = {
			lineNumber: startPos.lineNumber,
			column: startPos.column
		}
		showInlineAIChat = true
	}

	function closeAIInlineWidget() {
		showInlineAIChat = false
		selectedCode = ''
	}

	let loadTimeout: NodeJS.Timeout | undefined = undefined
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
</script>

<EditorTheme />
{#if !editor}
	<div class="inset-0 absolute overflow-clip">
		<FakeMonacoPlaceHolder {code} />
	</div>
{/if}
<div bind:this={divEl} class="{$$restProps.class || ''} editor {disabled ? 'disabled' : ''}"></div>
{#if $vimMode}
	<div class="fixed bottom-0 z-30" bind:this={statusDiv}></div>
{/if}

{#if $reviewingChanges}
	<GlobalReviewButtons
		on:acceptAll={() => {
			aiChatEditorHandler?.acceptAll()
		}}
		on:rejectAll={() => {
			aiChatEditorHandler?.rejectAll()
		}}
	/>
{/if}

{#if editor}
	<AIChatInlineWidget bind:show={showInlineAIChat} {editor} position={aiWidgetPosition} />
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

	/* AI Code Widget Decorations */
	:global(.ai-selected-code) {
		background-color: rgba(59, 130, 246, 0.1) !important;
		border: 1px solid rgba(59, 130, 246, 0.3) !important;
	}

	:global(.ai-processing) {
		background-color: rgba(245, 158, 11, 0.1) !important;
		border: 1px solid rgba(245, 158, 11, 0.3) !important;
	}

	:global(.ai-processing-text) {
		color: var(--color-warning) !important;
		font-style: italic !important;
		font-size: 12px !important;
	}
</style>
