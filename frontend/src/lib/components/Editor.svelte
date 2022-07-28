<script lang="ts">
	import { browser, dev } from '$app/env'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/utils'

	import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
	import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
	import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'

	import * as monaco from 'monaco-editor'

	import type { DocumentUri, MessageTransports } from 'monaco-languageclient'
	import { createEventDispatcher, onDestroy, onMount } from 'svelte'
	import { buildWorkerDefinition } from 'monaco-editor-workers'

	import { StandaloneServices } from 'vscode/services'
	import getMessageServiceOverride from 'vscode/service-override/messages'

	StandaloneServices.initialize({
		...getMessageServiceOverride(document.body)
	})

	let divEl: HTMLDivElement | null = null
	let editor: monaco.editor.IStandaloneCodeEditor

	export let deno = false
	export let lang = deno ? 'typescript' : 'python'
	export let code: string = ''
	export let hash: string = (Math.random() + 1).toString(36).substring(2)
	export let cmdEnterAction: (() => void) | undefined = undefined
	export let formatAction: (() => void) | undefined = undefined
	export let automaticLayout = true
	export let websocketAlive = { pyright: false, black: false, deno: false }
	export let extraLib: string = ''
	export let extraLibPath: string = ''

	let websockets: WebSocket[] = []
	let websocketInterval: NodeJS.Timer | undefined
	let lastWsAttempt: Date = new Date()
	let nbWsAttempt = 0
	let uri: string = ''
	let disposeMethod: () => void | undefined
	const dispatch = createEventDispatcher()

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
					if (label === 'json') {
						return new jsonWorker()
					} else if (label === 'typescript' || label === 'javascript') {
						return new tsWorker()
					} else {
						return new editorWorker()
					}
				}
			}
		}
	}

	export function getCode(): string {
		return editor?.getValue()
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

	export function setCode(ncode: string): void {
		if (editor) {
			return editor.setValue(ncode)
		} else {
			code = ncode
		}
	}

	function format() {
		if (editor) {
			editor.getAction('editor.action.formatDocument').run()
			if (formatAction) {
				formatAction()
			}
		}
	}

	export async function reloadWebsocket() {
		closeWebsockets()
		if (lang == 'python' || deno) {
			const { MonacoLanguageClient } = await import('monaco-languageclient')
			const { CloseAction, ErrorAction } = await import('vscode-languageclient')
			const { toSocket, WebSocketMessageReader, WebSocketMessageWriter } = await import(
				'vscode-ws-jsonrpc'
			)
			const vscode = await import('vscode')

			const { RequestType } = await import('vscode-jsonrpc')

			function createLanguageClient(
				transports: MessageTransports,
				name: string,
				initializationOptions?: any
			) {
				const client = new MonacoLanguageClient({
					name: name,
					clientOptions: {
						documentSelector: deno ? ['typescript'] : ['python'],
						errorHandler: {
							error: () => ({ action: ErrorAction.Continue }),
							closed: () => ({
								action: CloseAction.DoNotRestart
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

					webSocket.onopen = () => {
						websockets.push(webSocket)
						const socket = toSocket(webSocket)
						const reader = new WebSocketMessageReader(socket)
						const writer = new WebSocketMessageWriter(socket)
						const languageClient = createLanguageClient({ reader, writer }, name, options)
						languageClient.start()
						lastWsAttempt = new Date()
						nbWsAttempt = 0
						if (name == 'deno') {
							vscode.commands.getCommands().then((v) => {
								if (!v.includes('deno.cache')) {
									vscode.commands.registerCommand('deno.cache', (uris: DocumentUri[] = []) => {
										languageClient.sendRequest(new RequestType('deno/cache'), {
											referrer: { uri },
											uris: uris.map((uri) => ({ uri }))
										})
									})
								}
							})
						}
						reader.onClose(() => {
							try {
								languageClient.stop()
							} catch (err) {
								console.error(err)
							}
						})
						socket.onClose((_code, _reason) => {
							websocketAlive[name] = false
						})

						websocketAlive[name] = true
					}
				} catch (err) {
					console.error(`connection to ${name} language server failed`)
				}
			}

			if (deno) {
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
			} else {
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
			}

			websocketInterval && clearInterval(websocketInterval)
			websocketInterval = setInterval(() => {
				if (document.visibilityState == 'visible') {
					if (
						!lastWsAttempt ||
						(lastWsAttempt.getTime() - new Date().getTime() > 60000 && nbWsAttempt < 2)
					) {
						if (!websocketAlive.black && !websocketAlive.deno && !websocketAlive.pyright) {
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

	function closeWebsockets() {
		for (const x of websockets) {
			try {
				x.close()
			} catch (err) {
				console.log('error disposing websocket', err)
			}
		}
		websockets = []
		websocketInterval && clearInterval(websocketInterval)
	}

	async function loadMonaco() {
		if (lang == 'python') {
			monaco.languages.register({
				id: 'python',
				extensions: ['.py'],
				aliases: ['python'],
				mimetypes: ['application/text']
			})
		}

		let path: string = 'unknown'
		if (lang == 'python') {
			path = `${hash}.py`
		} else if (lang == 'json') {
			path = `${hash}.json`
		} else if (lang == 'javascript') {
			path = `${hash}.js`
		} else if (lang == 'typescript') {
			path = `${hash}.ts`
		}
		uri = `file:///${path}`
		const model = monaco.editor.createModel(code, lang, monaco.Uri.parse(uri))

		model.updateOptions({ tabSize: 2, insertSpaces: true })
		editor = monaco.editor.create(divEl as HTMLDivElement, {
			model: model,
			value: code,
			language: lang,
			automaticLayout,
			readOnly: false,
			autoDetectHighContrast: true,
			//lineNumbers: 'off',
			//lineDecorationsWidth: 0,
			lineNumbersMinChars: 4,
			lineNumbers: (ln) => '<span class="pr-4 text-gray-400">' + ln + '</span>',
			folding: false,
			scrollBeyondLastLine: false,
			minimap: {
				enabled: false
			},
			scrollbar: {
				alwaysConsumeMouseWheel: false
			},
			lightbulb: {
				enabled: true
			}
		})

		editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, function () {
			format()
		})

		editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, function () {
			if (cmdEnterAction) {
				cmdEnterAction()
			}
		})

		editor.onDidChangeModelContent((event) => {
			code = getCode()
			dispatch('change')
		})

		if (lang == 'json') {
			monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
				validate: true,
				allowComments: false,
				schemas: [],
				enableSchemaRequest: true
			})
		}

		if (lang == 'javascript' || lang == 'typescript') {
			monaco.languages.typescript.javascriptDefaults.setCompilerOptions({
				target: monaco.languages.typescript.ScriptTarget.Latest,
				allowNonTsExtensions: true,
				noLib: true
			})
		}

		if (lang == 'typescript') {
			monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
				target: monaco.languages.typescript.ScriptTarget.Latest,
				allowNonTsExtensions: true,
				noLib: true
			})
			if (deno) {
				monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
					noSemanticValidation: true,
					noSuggestionDiagnostics: true,
					noSyntaxValidation: true
				})
			}
		}

		if (lang == 'javascript') {
			if (extraLib != '' && extraLibPath != '') {
				monaco.languages.typescript.javascriptDefaults.addExtraLib(extraLib, extraLibPath)
			}
		}

		if (lang == 'python' || deno) {
			const { MonacoServices } = await import('monaco-languageclient')

			MonacoServices.install()
			// install Monaco language client services

			reloadWebsocket()
		}

		return () => {
			if (editor) {
				try {
					closeWebsockets()
					editor.dispose()
				} catch (err) {
					console.log('error disposing editor', err)
				}
			}
		}
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

<!-- <button class="default-button px-6 max-h-8" type="button" on:click={format}>
	Format (CtrlCmd + S)
</button> -->

<div bind:this={divEl} class={$$props.class} />

<style>
	.editor {
		@apply px-0;
		/* stylelint-disable-next-line unit-allowed-list */
		height: 80vh;
	}

	.small-editor {
		/* stylelint-disable-next-line unit-allowed-list */
		height: 26vh;
	}

	.few-lines-editor {
		/* stylelint-disable-next-line unit-allowed-list */
		height: 80px;
	}
</style>
