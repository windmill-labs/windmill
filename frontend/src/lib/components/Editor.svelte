<script lang="ts" context="module">
	import * as monaco from 'monaco-editor'

	monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
		target: monaco.languages.typescript.ScriptTarget.Latest,
		allowNonTsExtensions: true,
		noLib: true
	})
	monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
		noSemanticValidation: true,
		noSuggestionDiagnostics: true,
		noSyntaxValidation: true
	})
	monaco.editor.defineTheme('myTheme', {
		base: 'vs',
		inherit: true,
		rules: [],
		colors: {
			'editorLineNumber.foreground': '#999',
			'editorGutter.background': '#F9FAFB'
		}
	})
	monaco.editor.setTheme('myTheme')
</script>

<script lang="ts">
	import { browser, dev } from '$app/env'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/utils'

	import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
	import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'

	import { buildWorkerDefinition } from 'monaco-editor-workers'
	import type {
		Disposable,
		DocumentUri,
		MessageTransports,
		MonacoLanguageClient
	} from 'monaco-languageclient'
	import { createEventDispatcher, onDestroy, onMount } from 'svelte'

	import {
		BASH_INIT_CODE,
		DENO_INIT_CODE_CLEAR,
		GO_INIT_CODE,
		PYTHON_INIT_CODE_CLEAR
	} from '$lib/script_helpers'
	import {
		createHash as randomHash,
		editorConfig,
		langToExt,
		updateOptions
	} from '$lib/editorUtils'
	import { dirtyStore } from './common/confirmationModal/dirtyStore'

	let divEl: HTMLDivElement | null = null
	let editor: monaco.editor.IStandaloneCodeEditor

	export let lang: 'typescript' | 'python' | 'go' | 'shell'
	export let code: string = ''
	export let hash: string = randomHash()
	export let cmdEnterAction: (() => void) | undefined = undefined
	export let formatAction: (() => void) | undefined = undefined
	export let automaticLayout = true
	export let websocketAlive = { pyright: false, black: false, deno: false, go: false }
	export let shouldBindKey: boolean = true
	export let fixedOverflowWidgets = true

	let websockets: [MonacoLanguageClient, WebSocket][] = []
	let websocketInterval: NodeJS.Timer | undefined
	let lastWsAttempt: Date = new Date()
	let nbWsAttempt = 0
	let disposeMethod: () => void | undefined
	const dispatch = createEventDispatcher()

	const uri = `file:///tmp/monaco/${hash}.${langToExt(lang)}`

	if (browser) {
		if (dev) {
			buildWorkerDefinition(
				'../../../node_modules/monaco-editor-workers/dist/workers',
				import.meta.url,
				false
			)
		} else {
			// @ts-ignore
			self.MonacoEnvironment = {
				getWorker: function (_moduleId: any, label: string) {
					if (label === 'typescript' || label === 'javascript') {
						return new tsWorker()
					} else {
						return new editorWorker()
					}
				}
			}
		}
	}

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

	export function setCode(ncode: string): void {
		code = ncode
		if (editor) {
			editor.setValue(ncode)
		}
	}

	function format() {
		if (editor) {
			code = getCode()
			editor.getAction('editor.action.formatDocument').run()
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
	let monacoServices: Disposable | undefined = undefined

	export async function reloadWebsocket() {
		await closeWebsockets()
		const { MonacoLanguageClient } = await import('monaco-languageclient')
		const { CloseAction, ErrorAction } = await import('vscode-languageclient')
		const { toSocket, WebSocketMessageReader, WebSocketMessageWriter } = await import(
			'vscode-ws-jsonrpc'
		)
		const vscode = await import('vscode')
		const { RequestType } = await import('vscode-jsonrpc')
		// install Monaco language client services
		const { MonacoServices } = await import('monaco-languageclient')

		monacoServices = MonacoServices.install()

		function createLanguageClient(
			transports: MessageTransports,
			name: string,
			initializationOptions?: any
		) {
			const client = new MonacoLanguageClient({
				name: name,
				clientOptions: {
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

					// workspaceFolder: { uri: Uri.parse(`/tmp/${name}`), name: 'tmp', index: 0 },
					initializationOptions,
					middleware: {
						workspace: {
							configuration: (params, token, configuration) => {
								return [
									{
										enable: true
									}
								]
							}
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

		async function connectToLanguageServer(url: string, name: string, options?: any) {
			try {
				const webSocket = new WebSocket(url)

				webSocket.onopen = async () => {
					const socket = toSocket(webSocket)
					const reader = new WebSocketMessageReader(socket)
					const writer = new WebSocketMessageWriter(socket)
					const languageClient = createLanguageClient({ reader, writer }, name, options)
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
						console.log('started client')
						await languageClient.start()
					} catch (err) {
						console.log('err at client')
						console.error(err)
						throw new Error(err)
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

		if (lang == 'typescript') {
			await connectToLanguageServer(`wss://${$page.url.host}/ws/deno`, 'deno', {
				certificateStores: null,
				enablePaths: [],
				config: null,
				importMap: null,
				internalDebug: false,
				lint: false,
				path: null,
				tlsCertificate: null,
				unsafelyIgnoreCertificateErrors: null,
				unstable: true,
				enable: true,
				cache: null,
				codeLens: {
					implementations: true,
					references: true
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
			})
		} else if (lang === 'python') {
			await connectToLanguageServer(`wss://${$page.url.host}/ws/pyright`, 'pyright', {
				executionEnvironments: [
					{
						root: '/tmp/pyright',
						pythonVersion: '3.7',
						pythonPlatform: 'platform',
						extraPaths: []
					}
				]
			})

			connectToLanguageServer(`wss://${$page.url.host}/ws/black`, 'black', {
				formatters: {
					black: {
						command: 'black',
						args: ['--quiet', '-']
					}
				},
				formatFiletypes: {
					python: 'black'
				}
			})
		} else if (lang === 'go') {
			connectToLanguageServer(`wss://${$page.url.host}/ws/go`, 'go', {
				'build.allowImplicitNetworkAccess': true
			})
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

	async function closeWebsockets() {
		command && command.dispose()
		command = undefined
		monacoServices && monacoServices.dispose()
		monacoServices = undefined
		for (const x of websockets) {
			try {
				await x[0].stop()
				x[1].close()
			} catch (err) {
				try {
					x[1].close()
				} catch (err) {}
				console.log('error disposing websocket', err)
			}
		}
		websockets = []
		websocketInterval && clearInterval(websocketInterval)
	}

	async function loadMonaco() {
		const model = monaco.editor.createModel(code, lang, monaco.Uri.parse(uri))

		model.updateOptions(updateOptions)
		editor = monaco.editor.create(
			divEl as HTMLDivElement,
			editorConfig(model, code, lang, automaticLayout, fixedOverflowWidgets)
		)

		let timeoutModel: NodeJS.Timeout | undefined = undefined
		editor.onDidChangeModelContent((event) => {
			$dirtyStore = true

			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				code = getCode()
				dispatch('change')
			}, 500)
		})

		editor.onDidFocusEditorText(() => {
			dispatch('focus')

			editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, function () {
				code = getCode()
				shouldBindKey && format && format()
			})

			editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, function () {
				code = getCode()
				shouldBindKey && cmdEnterAction && cmdEnterAction()
			})

			if (
				!websocketAlive.black &&
				!websocketAlive.deno &&
				!websocketAlive.pyright &&
				!websocketAlive.go &&
				!websocketInterval
			) {
				reloadWebsocket()
			}
		})

		editor.onDidBlurEditorText(() => {
			dispatch('blur')
		})

		reloadWebsocket()

		return () => {
			try {
				closeWebsockets()
				model.dispose()
				editor && editor.dispose()
			} catch (err) {
				console.log('error disposing editor', err)
			}
		}
	}

	export function addAction(
		id: string,
		label: string,
		callback: (editor: monaco.editor.IStandaloneCodeEditor) => void,
		keybindings: number[] = []
	) {
		editor.addAction({
			id,
			label,
			keybindings,
			contextMenuGroupId: 'navigation',

			run: function (editor: monaco.editor.IStandaloneCodeEditor) {
				callback(editor)
			}
		})
	}

	onMount(() => {
		if (browser) {
			loadMonaco().then((x) => (disposeMethod = x))
		}
	})

	onDestroy(() => {
		disposeMethod && disposeMethod()
		websocketInterval && clearInterval(websocketInterval)
	})
</script>

<div bind:this={divEl} class="{$$props.class} editor" />

<style>
	.editor {
		@apply p-0 border rounded-md border-gray-50;
	}
</style>
