<script lang="ts">
	import { BROWSER } from 'esm-env'

	import { sendUserToast } from '$lib/toast'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'

	import * as vscode from 'vscode'

	import 'monaco-editor/esm/vs/editor/edcore.main'
	import {
		editor as meditor,
		KeyCode,
		KeyMod,
		Uri as mUri,
		languages
	} from 'monaco-editor/esm/vs/editor/editor.api'
	import 'monaco-editor/esm/vs/basic-languages/python/python.contribution'
	import 'monaco-editor/esm/vs/basic-languages/go/go.contribution'
	import 'monaco-editor/esm/vs/basic-languages/shell/shell.contribution'
	import 'monaco-editor/esm/vs/basic-languages/typescript/typescript.contribution'
	import 'monaco-editor/esm/vs/language/typescript/monaco.contribution'
	import { MonacoLanguageClient, initServices } from 'monaco-languageclient'
	import { toSocket, WebSocketMessageReader, WebSocketMessageWriter } from 'vscode-ws-jsonrpc'
	import { CloseAction, ErrorAction, RequestType } from 'vscode-languageclient'
	import { MonacoBinding } from 'y-monaco'

	languages.typescript.typescriptDefaults.setModeConfiguration({
		completionItems: false,
		definitions: false,
		hovers: false
	})

	meditor.defineTheme('myTheme', {
		base: 'vs',
		inherit: true,
		rules: [],
		colors: {
			'editorLineNumber.foreground': '#999',
			'editorGutter.background': '#F9FAFB'
		}
	})
	meditor.setTheme('myTheme')

	import {
		createHash as randomHash,
		editorConfig,
		langToExt,
		updateOptions
	} from '$lib/editorUtils'
	import {
		BASH_INIT_CODE,
		DENO_INIT_CODE_CLEAR,
		GO_INIT_CODE,
		PYTHON_INIT_CODE_CLEAR
	} from '$lib/script_helpers'
	import type { Disposable } from 'vscode'
	import type { DocumentUri, MessageTransports } from 'vscode-languageclient'
	import { dirtyStore } from './common/confirmationModal/dirtyStore'
	import { buildWorkerDefinition } from './build_workers'
	import { workspaceStore } from '$lib/stores'
	import { UserService } from '$lib/gen'
	import type { Text } from 'yjs'

	let divEl: HTMLDivElement | null = null
	let editor: meditor.IStandaloneCodeEditor

	export let lang: 'typescript' | 'python' | 'go' | 'shell'
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
	export let path: string = randomHash()
	export let yContent: Text | undefined = undefined
	export let awareness: any | undefined = undefined

	if (path == '' || path == undefined || path.startsWith('/')) {
		path = randomHash()
	}

	let initialPath: string = path

	$: path != initialPath && lang == 'typescript' && handlePathChange()

	let websockets: [MonacoLanguageClient, WebSocket][] = []
	let websocketInterval: NodeJS.Timer | undefined
	let lastWsAttempt: Date = new Date()
	let nbWsAttempt = 0
	let disposeMethod: () => void | undefined
	const dispatch = createEventDispatcher()

	const uri =
		lang == 'typescript'
			? `file:///${path}.${langToExt(lang)}`
			: `file:///tmp/monaco/${randomHash()}.${langToExt(lang)}`

	buildWorkerDefinition('../../../workers', import.meta.url, false)

	export function getCode(): string {
		return editor?.getValue() ?? ''
	}

	export function insertAtCursor(code: string): void {
		if (editor) {
			editor.trigger('keyboard', 'type', { text: code })
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

	export function format() {
		if (editor) {
			code = getCode()
			if (lang != 'shell') {
				editor?.getAction('editor.action.formatDocument')?.run()
			}
			if (formatAction) {
				formatAction()
			}
		}
	}

	export async function clearContent() {
		if (editor) {
			if (lang == 'typescript') {
				setCode(DENO_INIT_CODE_CLEAR)
			} else if (lang == 'python') {
				setCode(PYTHON_INIT_CODE_CLEAR)
			} else if (lang == 'go') {
				setCode(GO_INIT_CODE)
			} else if (lang == 'shell') {
				setCode(BASH_INIT_CODE)
			}
		}
	}

	let command: Disposable | undefined = undefined

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
		try {
			await initServices({
				enableThemeService: false,
				enableModelEditorService: true,
				enableNotificationService: false,
				modelEditorServiceConfig: {
					useDefaultFunction: true
				},
				debugLogging: false
			})
		} catch (e) {
			console.log('initServices failed', e.message)
			if (e.message != 'Lifecycle cannot go backwards') {
				return
			}
		}

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
					websockets.push([languageClient, webSocket])

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
							console.error(err)
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
		if (lang == 'typescript') {
			if (path && path.split('/').length > 2) {
				let expiration = new Date()
				expiration.setHours(expiration.getHours() + 2)
				const token = await UserService.createToken({
					requestBody: { label: 'Ephemeral lsp token', expiration: expiration.toISOString() }
				})
				let root = hostname + '/api/scripts_u/tokened_raw/' + $workspaceStore + '/' + token
				const importMap = {
					imports: {
						'file:///': root + '/'
					}
				}
				let path_splitted = path.split('/')
				for (let c = 0; c < path_splitted.length; c++) {
					let key = 'file://./'
					for (let i = 0; i < c; i++) {
						key += '../'
					}
					let url = path_splitted.slice(0, -c - 1).join('/')
					let ending = c == path_splitted.length - 1 ? '' : '/'
					importMap['imports'][key] = `${root}/${url}${ending}`
				}
				encodedImportMap = 'data:text/plain;base64,' + btoa(JSON.stringify(importMap))
			}
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
						!websocketAlive.go
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

	let pathTimeout: NodeJS.Timeout | undefined = undefined
	function handlePathChange() {
		initialPath = path
		pathTimeout && clearTimeout(pathTimeout)
		pathTimeout = setTimeout(reloadWebsocket, 3000)
	}

	async function closeWebsockets() {
		command && command.dispose()
		command = undefined
		for (const x of websockets) {
			try {
				await x[0].dispose()
				x[1].close()
			} catch (err) {
				console.log('error disposing language client, closing websocket', err)
				try {
					x[1].close()
				} catch (err) {
					console.log('error disposing websocket, closin', err)
				}
			}
		}
		console.log('disposed language client and closed websocket')
		websockets = []
		websocketInterval && clearInterval(websocketInterval)
	}

	let widgets: HTMLElement | undefined = document.getElementById('monaco-widgets-root') ?? undefined
	let model: meditor.ITextModel

	let monacoBinding: MonacoBinding | undefined = undefined
	// @ts-ignore
	$: if (yContent && awareness && model && editor) {
		monacoBinding && monacoBinding.destroy()
		monacoBinding = new MonacoBinding(yContent, model, new Set([editor]), awareness)
	}

	async function loadMonaco() {
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
			...editorConfig(model, code, lang, automaticLayout, fixedOverflowWidgets),
			overflowWidgetsDomNode: widgets,
			tabSize: lang == 'python' ? 4 : 2
		})

		let timeoutModel: NodeJS.Timeout | undefined = undefined
		editor.onDidChangeModelContent((event) => {
			$dirtyStore = true

			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				let ncode = getCode()
				if (ncode != '') {
					code = ncode
					dispatch('change', code)
				}
			}, 500)
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
				!websocketInterval
			) {
				reloadWebsocket()
			}
		})

		reloadWebsocket()

		return () => {
			console.log('disposing editor')
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
		disposeMethod && disposeMethod()
		websocketInterval && clearInterval(websocketInterval)
	})
</script>

<div bind:this={divEl} class="{$$props.class} editor" />

<style global lang="postcss">
	.editor {
		@apply p-0 border rounded-md border-gray-50;
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
