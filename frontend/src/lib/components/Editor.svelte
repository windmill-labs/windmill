<script lang="ts">
	import { page } from '$app/stores'
	import type monaco from 'monaco-editor'
	import { browser } from '$app/env'

	import { onDestroy, onMount } from 'svelte'
	import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
	import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
	import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'
	import type { DocumentUri, MessageTransports } from 'monaco-languageclient'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()

	let divEl: HTMLDivElement | null = null
	let editor: monaco.editor.IStandaloneCodeEditor

	export let deno = false
	export let lang = deno ? 'typescript' : 'python'
	export let code: string
	export let hash: string = (Math.random() + 1).toString(36).substring(2)
	export let cmdEnterAction: (() => void) | undefined = undefined
	export let formatAction: (() => void) | undefined = undefined
	export let automaticLayout = true
	export let websocketAlive = { pyright: false, black: false, deno: false }
	let websockets: WebSocket[] = []
	let uri: string = ''
	let disposeMethod: () => void | undefined

	if (browser) {
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
			// install Monaco language client services
			const { MonacoLanguageClient } = await import('monaco-languageclient')
			const { CloseAction, ErrorAction } = await import('vscode-languageclient')
			const vscode = await import('vscode')

			const { RequestType, toSocket, WebSocketMessageReader, WebSocketMessageWriter } =
				await import('@codingame/monaco-jsonrpc')

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
							error: () => ({ action: ErrorAction.Shutdown }),
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

					webSocket.onopen = () => {
						websockets.push(webSocket)
						const socket = toSocket(webSocket)
						const reader = new WebSocketMessageReader(socket)
						const writer = new WebSocketMessageWriter(socket)
						const languageClient = createLanguageClient({ reader, writer }, name, options)
						languageClient.start()
						socket.onClose((_code, _reason) => {
							websocketAlive[name] = false
							try {
								languageClient.stop()
							} catch (err) {
								console.error(err)
							}
						})

						vscode.commands.registerCommand('deno.cache', (uris: DocumentUri[] = []) => {
							languageClient.sendRequest(new RequestType('deno/cache'), {
								referrer: { uri },
								uris: uris.map((uri) => ({ uri }))
							})
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
					unstable: false,
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
	}

	async function loadMonaco() {
		const monaco = await import('monaco-editor')
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
		model.updateOptions({ tabSize: 4, insertSpaces: true })
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
			} else {
				monaco.languages.typescript.typescriptDefaults.addExtraLib(
					`
/**
 * get variable (including secret) at path
 * @param {string} path - path of the variable (e.g: g/all/pretty_secret)
 */
export function variable(path: string): string;

/**
 * get resource at path
 * @param {string} path - path of the resource (e.g: g/all/my_resource)
 */
export function resource(path: string): any;

/**
 * get result of step n.
 * If n is negative, for instance -1, it is the step just before this one.
 * Step 0 is flow input.
 * @param {number} n - step number.
 */
export function step(n: number): any;

/**
 * flow input as an object
 */
export const flow_input: any;

/**
 * previous result as an object
 */
export const previous_result: any;

/**
 * static params of this same step
 */
export const params: any;
				`,
					'file:///node_modules/@types/windmill/index.d.ts'
				)
			}
		}
		if (lang == 'python' || deno) {
			const { MonacoServices } = await import('monaco-languageclient')

			MonacoServices.install(monaco)
		}

		reloadWebsocket()

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
		if (disposeMethod) {
			disposeMethod()
		}
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
