<script lang="ts" context="module">
	import getDialogServiceOverride from 'vscode/service-override/dialogs'
	import getNotificationServiceOverride from 'vscode/service-override/notifications'
	import { StandaloneServices } from 'vscode/services'

	try {
		StandaloneServices?.initialize({
			...getNotificationServiceOverride(document.body),
			...getDialogServiceOverride()
		})
	} catch (e) {
		console.error(e)
	}
</script>

<script lang="ts">
	import { browser } from '$app/environment'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/utils'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'

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
	import { MonacoLanguageClient, MonacoServices } from 'monaco-languageclient'
	import { toSocket, WebSocketMessageReader, WebSocketMessageWriter } from 'vscode-ws-jsonrpc'
	import { CloseAction, ErrorAction, RequestType } from 'vscode-languageclient'
	import * as vscode from 'vscode'

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

	let divEl: HTMLDivElement | null = null
	let editor: meditor.IStandaloneCodeEditor

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

	// if (lang != 'typescript') {
	buildWorkerDefinition('../../../workers', import.meta.url, false)
	// }

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
			editor?.getAction('editor.action.formatDocument')?.run()
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

		monacoServices = MonacoServices.install()

		function createLanguageClient(
			transports: MessageTransports,
			name: string,
			initializationOptions: any,
			middlewareOptions: ((params, token, next) => any) | undefined
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

		const wsProtocol = $page.url.protocol == 'https:' ? 'wss' : 'ws'
		if (lang == 'typescript') {
			await connectToLanguageServer(
				`${wsProtocol}://${$page.url.host}/ws/deno`,
				'deno',
				{
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
				undefined
			)
		} else if (lang === 'python') {
			await connectToLanguageServer(
				`${wsProtocol}://${$page.url.host}/ws/pyright`,
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
				`${wsProtocol}://${$page.url.host}/ws/black`,
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
				`${wsProtocol}://${$page.url.host}/ws/go`,
				'go',
				{
					'build.allowImplicitNetworkAccess': true
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

	let widgets: HTMLElement | undefined = document.getElementById('monaco-widgets-root') ?? undefined
	async function loadMonaco() {
		const model = meditor.createModel(code, lang, mUri.parse(uri))

		model.updateOptions(updateOptions)
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
				code = getCode()
				dispatch('change', code)
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
				!websocketAlive.go &&
				!websocketInterval
			) {
				reloadWebsocket()
			}
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
		callback: (editor: meditor.IStandaloneCodeEditor) => void,
		keybindings: number[] = []
	) {
		editor.addAction({
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

<style lang="postcss">
	.editor {
		@apply p-0 border rounded-md border-gray-50;
	}
</style>
