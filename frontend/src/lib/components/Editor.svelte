<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { sendUserToast } from '$lib/toast'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'

	import * as vscode from 'vscode'

	import {
		editor as meditor,
		languages,
		KeyCode,
		KeyMod,
		Uri as mUri,
		type IRange
	} from 'monaco-editor'
	import 'monaco-editor/esm/vs/basic-languages/python/python.contribution'
	import 'monaco-editor/esm/vs/basic-languages/go/go.contribution'
	import 'monaco-editor/esm/vs/basic-languages/shell/shell.contribution'
	import 'monaco-editor/esm/vs/basic-languages/typescript/typescript.contribution'
	import 'monaco-editor/esm/vs/basic-languages/sql/sql.contribution'
	import 'monaco-editor/esm/vs/basic-languages/graphql/graphql.contribution'
	import 'monaco-editor/esm/vs/basic-languages/powershell/powershell.contribution'
	import 'monaco-editor/esm/vs/language/typescript/monaco.contribution'
	import 'monaco-editor/esm/vs/basic-languages/css/css.contribution'

	import libStdContent from '$lib/es6.d.ts.txt?raw'
	import denoFetchContent from '$lib/deno_fetch.d.ts.txt?raw'

	import { MonacoLanguageClient } from 'monaco-languageclient'

	import { toSocket, WebSocketMessageReader, WebSocketMessageWriter } from 'vscode-ws-jsonrpc'
	import { CloseAction, ErrorAction, RequestType } from 'vscode-languageclient'
	import { MonacoBinding } from 'y-monaco'
	import {
		dbSchemas,
		type DBSchema,
		copilotInfo,
		codeCompletionSessionEnabled,
		lspTokenStore,
		formatOnSave
	} from '$lib/stores'

	import {
		createHash as randomHash,
		editorConfig,
		langToExt,
		updateOptions
	} from '$lib/editorUtils'
	import type { Disposable } from 'vscode'
	import type { DocumentUri, MessageTransports } from 'vscode-languageclient'
	import { buildWorkerDefinition } from './build_workers'
	import { workspaceStore } from '$lib/stores'
	import { Preview, UserService } from '$lib/gen'
	import type { Text } from 'yjs'
	import { initializeMode } from 'monaco-graphql/esm/initializeMode.js'
	import type { MonacoGraphQLAPI } from 'monaco-graphql/esm/api.js'
	import { sleep } from '$lib/utils'
	import { editorCodeCompletion } from './copilot/completion'
	import { initializeVscode } from './vscode'
	import EditorTheme from './EditorTheme.svelte'
	import {
		BIGQUERY_TYPES,
		MSSQL_TYPES,
		MYSQL_TYPES,
		POSTGRES_TYPES,
		SNOWFLAKE_TYPES
	} from '$lib/consts'
	import { setupTypeAcquisition } from '$lib/ata/index'
	import { initWasm, parseDeps } from '$lib/infer'
	// import EditorTheme from './EditorTheme.svelte'

	let divEl: HTMLDivElement | null = null
	let editor: meditor.IStandaloneCodeEditor

	export let lang:
		| 'typescript'
		| 'python'
		| 'go'
		| 'shell'
		| 'sql'
		| 'graphql'
		| 'powershell'
		| 'css'
		| 'javascript'
	export let code: string = ''
	export let cmdEnterAction: (() => void) | undefined = undefined
	export let formatAction: (() => void) | undefined = undefined
	export let automaticLayout = true
	export let websocketAlive = {
		pyright: false,
		black: false,
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
	export let listenEmptyChanges = false
	export let small = false
	export let scriptLang: Preview.language
	export let disabled: boolean = false

	const rHash = randomHash()
	$: filePath = computePath(path)

	function computePath(path: string | undefined): string {
		if (path == '' || path == undefined || path.startsWith('/')) {
			return rHash
		} else {
			return path as string
		}
	}

	let initialPath: string | undefined = path

	$: path != initialPath && (scriptLang == 'deno' || scriptLang == 'bun') && handlePathChange()

	let websockets: WebSocket[] = []
	let languageClients: MonacoLanguageClient[] = []
	let websocketInterval: NodeJS.Timeout | undefined
	let lastWsAttempt: Date = new Date()
	let nbWsAttempt = 0
	let disposeMethod: () => void | undefined
	const dispatch = createEventDispatcher()
	let graphqlService: MonacoGraphQLAPI | undefined = undefined

	let dbSchema: DBSchema | undefined = undefined

	let destroyed = false
	const uri =
		lang != 'go' && lang != 'typescript' && lang != 'python'
			? `file:///${filePath ?? rHash}.${langToExt(lang)}`
			: `file:///tmp/monaco/${randomHash()}.${langToExt(lang)}`

	console.log('uri', uri)

	buildWorkerDefinition('../../../workers', import.meta.url, false)

	export function getCode(): string {
		return editor?.getValue() ?? ''
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
		code = ncode
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

	export function append(code): void {
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
			code = getCode()
			if (lang != 'shell') {
				if ($formatOnSave != false) {
					await editor?.getAction('editor.action.formatDocument')?.run()
				}
				code = getCode()
			}
			if (formatAction) {
				formatAction()
			}
		}
	}

	let command: Disposable | undefined = undefined

	let sqlTypeCompletor: Disposable | undefined = undefined

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

	let sqlSchemaCompletor: Disposable | undefined = undefined

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

	let copilotCompletor: Disposable | undefined = undefined
	let copilotTs = Date.now()
	let abortController: AbortController | undefined = undefined
	function addCopilotSuggestions() {
		if (copilotCompletor) {
			copilotCompletor.dispose()
		}
		copilotCompletor = vscode.languages.registerInlineCompletionItemProvider(
			{ pattern: '**' },
			{
				async provideInlineCompletionItems(model, position, context, token) {
					abortController?.abort()
					const textUntilPosition = model.getText(
						new vscode.Range(0, 0, position.line, position.character)
					)
					let items: vscode.InlineCompletionItem[] = []

					const lastChar = textUntilPosition[textUntilPosition.length - 1]
					if (textUntilPosition.trim().length > 5 && lastChar.match(/[\(\{\s:=]/)) {
						const textAfterPosition = model.getText(
							new vscode.Range(position.line, position.character, model.lineCount + 1, 1)
						)

						const thisTs = Date.now()
						copilotTs = thisTs
						await sleep(200)
						if (copilotTs === thisTs) {
							abortController?.abort()
							abortController = new AbortController()
							token.onCancellationRequested(() => {
								abortController?.abort()
							})
							const insertText = await editorCodeCompletion(
								textUntilPosition,
								textAfterPosition,
								lang,
								abortController
							)
							if (insertText) {
								items = [
									{
										insertText,
										range: new vscode.Range(
											position.line,
											position.character,
											position.line,
											position.character
										)
									}
								]
							}
						}
					}

					return {
						items,
						commands: []
					}
				}
			}
		)
	}

	$: $copilotInfo.exists_openai_resource_path &&
		$copilotInfo.code_completion_enabled &&
		$codeCompletionSessionEnabled &&
		initialized &&
		addCopilotSuggestions()

	$: !$codeCompletionSessionEnabled && copilotCompletor && copilotCompletor.dispose()

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
		console.log('reloadWebsocket')
		await closeWebsockets()

		function createLanguageClient(
			transports: MessageTransports,
			name: string,
			initializationOptions: any,
			middlewareOptions: ((params, token, next) => any) | undefined
		) {
			const client = new MonacoLanguageClient({
				name: name,
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
				},
				connectionProvider: {
					get: () => {
						return Promise.resolve(transports)
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
		const hostname = BROWSER ? window.location.protocol + '//' + window.location.host : 'SSR'

		let encodedImportMap = ''
		// if (lang == 'typescript') {

		// 	let worker = await languages.typescript.getTypeScriptWorker()
		// 	console.log(worker)
		// }
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
			} else if (lang === 'javascript') {
				const stdLib = { content: libStdContent, filePath: 'es6.d.ts' }
				if (scriptLang == 'bun') {
					languages.typescript.javascriptDefaults.setExtraLibs([stdLib])
				} else {
					const denoFetch = { content: denoFetchContent, filePath: 'deno_fetch.d.ts' }
					languages.typescript.javascriptDefaults.setExtraLibs([stdLib, denoFetch])
				}
				if (scriptLang == 'bun' && ata == undefined) {
					const addLibraryToRuntime = async (code: string, _path: string) => {
						const path = 'file://' + _path
						let uri = mUri.parse(path)
						console.log('adding library to runtime', path)
						languages.typescript.javascriptDefaults.addExtraLib(code, path)
						try {
							await vscode.workspace.fs.writeFile(uri, new TextEncoder().encode(code))
						} catch (e) {
							console.log('error writing file', e)
						}
					}

					const addLocalFile = async (code: string, _path: string) => {
						let p = new URL(_path, uri).href
						let nuri = mUri.parse(p)
						if (editor) {
							let localModel = meditor.getModel(nuri)
							if (localModel) {
								localModel.setValue(code)
							} else {
								meditor.createModel(code, 'javascript', nuri)
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
					await initWasm()
					const root = await genRoot(hostname)
					console.log('SETUP TYPE ACQUISITION', { root, path })
					ata = setupTypeAcquisition({
						projectName: 'Windmill',
						depsParser: (c) => {
							return parseDeps(c)
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
					ata?.('import "bun-types"')
					ata?.(code)
				}
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
				connectToLanguageServer(
					`${wsProtocol}://${window.location.host}/ws/diagnostic`,
					'black',
					{
						formatters: {
							black: {
								command: 'black',
								args: ['--quiet', '-']
							}
						},
						formatFiletypes: {
							python: 'black'
						}
					},
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
							!websocketAlive.black &&
							!websocketAlive.deno &&
							!websocketAlive.pyright &&
							!websocketAlive.go &&
							!websocketAlive.shellcheck &&
							!websocketAlive.ruff &&
							scriptLang != 'bun'
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
	let ata: ((s: string) => void) | undefined = undefined

	async function loadMonaco() {
		try {
			console.log("Loading Monaco's language client")
			await initializeVscode()
		} catch (e) {
			console.log('error initializing services', e)
		}

		// console.log('bef ready')
		// console.log('af ready')

		initialized = true

		languages.typescript.typescriptDefaults.setModeConfiguration({
			completionItems: false,
			definitions: false,
			hovers: false
		})

		languages.typescript.typescriptDefaults.setCompilerOptions({
			target: languages.typescript.ScriptTarget.Latest,
			allowNonTsExtensions: true,
			noSemanticValidation: false,
			noSyntaxValidation: false,

			checkJs: true,
			allowJs: true,
			noUnusedLocals: true,
			strict: true,
			noLib: false,
			moduleResolution: languages.typescript.ModuleResolutionKind.NodeJs
		})

		languages.typescript.javascriptDefaults.setModeConfiguration({
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

		languages.typescript.javascriptDefaults.setEagerModelSync(true)
		languages.typescript.javascriptDefaults.setDiagnosticsOptions({
			noSemanticValidation: false,
			noSyntaxValidation: false,
			noSuggestionDiagnostics: false,
			diagnosticCodesToIgnore: [1108]
		})

		languages.typescript.javascriptDefaults.setCompilerOptions({
			target: languages.typescript.ScriptTarget.Latest,
			allowNonTsExtensions: true,
			noSemanticValidation: false,
			noSyntaxValidation: false,
			allowImportingTsExtensions: true,
			checkJs: true,
			allowJs: true,
			noUnusedParameters: true,
			noUnusedLocals: true,
			strict: true,
			noLib: false,

			moduleResolution: languages.typescript.ModuleResolutionKind.NodeJs
		})

		try {
			model = meditor.createModel(code, lang, mUri.parse(uri))
		} catch (err) {
			console.log('model already existed', err)
			const nmodel = meditor.getModel(mUri.parse(uri))
			if (!nmodel) {
				throw err
			}
			model = nmodel
		}
		model.updateOptions(lang == 'python' ? { tabSize: 4, insertSpaces: true } : updateOptions)

		editor = meditor.create(divEl as HTMLDivElement, {
			...editorConfig(code, lang, automaticLayout, fixedOverflowWidgets),
			model,
			fontSize: !small ? 14 : 12,
			// overflowWidgetsDomNode: widgets,
			tabSize: lang == 'python' ? 4 : 2,
			folding
		})

		let timeoutModel: NodeJS.Timeout | undefined = undefined
		let ataModel: NodeJS.Timeout | undefined = undefined

		editor.onDidChangeModelContent((event) => {
			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				let ncode = getCode()
				if (ncode != '' || listenEmptyChanges) {
					code = ncode
					dispatch('change', code)
				}
			}, 500)

			ataModel && clearTimeout(ataModel)
			ataModel = setTimeout(() => {
				ata?.(getCode())
			}, 1000)
		})

		editor.onDidBlurEditorText(() => {
			dispatch('blur')
		})

		editor.onDidFocusEditorText(() => {
			dispatch('focus')

			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyS, function () {
				code = getCode()
				shouldBindKey && format && format()
			})

			editor.addCommand(KeyMod.CtrlCmd | KeyCode.Enter, function () {
				code = getCode()
				shouldBindKey && cmdEnterAction && cmdEnterAction()
			})

			if (
				!websocketAlive.black &&
				!websocketAlive.deno &&
				!websocketAlive.pyright &&
				!websocketAlive.ruff &&
				!websocketAlive.shellcheck &&
				!websocketAlive.go &&
				!websocketInterval &&
				scriptLang != 'bun'
			) {
				console.log('reconnecting to language servers on focus')
				reloadWebsocket()
			}
		})

		reloadWebsocket()

		return () => {
			console.log('disposing editor')
			ata = undefined
			try {
				closeWebsockets()
				model?.dispose()
				editor && editor.dispose()
				console.log('disposed editor')
			} catch (err) {
				console.log('error disposing editor', err)
			}
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

	onMount(() => {
		if (BROWSER) {
			loadMonaco().then((x) => (disposeMethod = x))
		}
	})

	onDestroy(() => {
		console.log('destroying editor')
		destroyed = true
		disposeMethod && disposeMethod()
		websocketInterval && clearInterval(websocketInterval)
		sqlSchemaCompletor && sqlSchemaCompletor.dispose()
		copilotCompletor && copilotCompletor.dispose()
		sqlTypeCompletor && sqlTypeCompletor.dispose()
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
<div bind:this={divEl} class="{$$props.class} editor {disabled ? 'disabled' : ''}" />

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
