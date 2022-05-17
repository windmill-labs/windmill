<script lang="ts">
	import { page } from '$app/stores'
	import type monaco from 'monaco-editor'
	import { browser, mode } from '$app/env'

	import { listen } from '@codingame/monaco-jsonrpc'
	import { onDestroy, onMount } from 'svelte'
	import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
	import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
	import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'

	let divEl: HTMLDivElement | null = null
	let editor: monaco.editor.IStandaloneCodeEditor
	let monaco

	export let deno = false
	export let lang = deno ? 'typescript' : 'python'
	export let code: string
	export let readOnly = false
	export let hash: string = (Math.random() + 1).toString(36).substring(2)
	export let cmdEnterAction: (() => void) | undefined = undefined
	export let formatAction: (() => void) | undefined = undefined
	export let automaticLayout = true
	export let websocketAlive = { pyright: false, black: false }
	let websockets: WebSocket[] = []

	let disposeMethod: () => void | undefined

	if (browser) {
		// @ts-ignore
		self.MonacoEnvironment = {
			getWorker: function (_moduleId: any, label: string) {
				if (label === 'json') {
					return new jsonWorker()
				}
				if (label === 'typescript' || label === 'javascript') {
					return new tsWorker()
				}
				return new editorWorker()
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
			const range = new monaco.Range(1, 1, 1, 1)
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
			const { MonacoLanguageClient, CloseAction, ErrorAction, createConnection } = await import(
				'@codingame/monaco-languageclient'
			)

			function createLanguageClient(connection: any, name: string, initializationOptions?: any) {
				return new MonacoLanguageClient({
					name: name,
					clientOptions: {
						documentSelector: deno ? ['typescript'] : ['python'],
						errorHandler: {
							error: () => ErrorAction.Shutdown,
							closed: () => CloseAction.Restart
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
						get: (errorHandler, closeHandler) => {
							return Promise.resolve(createConnection(connection, errorHandler, closeHandler))
						}
					}
				})
			}

			function connectToLanguageServer(url: string, name: string, options?: any) {
				try {
					const webSocket = new WebSocket(url)
					websockets.push(webSocket)
					// listen when the web socket is opened
					listen({
						webSocket,
						onConnection: (connection) => {
							// create and start the language client
							const languageClient = createLanguageClient(connection, name, options)
							const disposable = languageClient.start()
							websocketAlive[name] = true

							connection.onClose(() => {
								websocketAlive[name] = false
								try {
									disposable.dispose()
								} catch (err) {
									console.error('error disposing websocket', err)
								}
							})
						}
					})
				} catch (err) {
					console.error(`connection to ${name} language server failed`)
				}
			}

			if (deno) {
				connectToLanguageServer(`ws://${$page.url.host}/ws/deno`, 'deno', {
					deno: {
						enable: true,
						lint: true
					}
				})
			} else {
				connectToLanguageServer(`wss://${$page.url.host}/ws/pyright`, 'pyright', {
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
		websockets.forEach((x) => {
			try {
				x.close()
			} catch (err) {
				console.log('error disposing websocket', err)
			}
		})
	}
	async function loadMonaco() {
		monaco = await import('monaco-editor')

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
		const model = monaco.editor.createModel(code, lang, monaco.Uri.parse(`file:///${path}`))
		model.updateOptions({ tabSize: 4, insertSpaces: true })
		editor = monaco.editor.create(divEl as HTMLDivElement, {
			model: model,
			value: code,
			language: lang,
			automaticLayout,
			readOnly: readOnly,
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
			if (deno) {
				monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
					diagnosticCodesToIgnore: [2691]
				})
			} else {
				// compiler options
				monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
					target: monaco.languages.typescript.ScriptTarget.ES6,
					allowNonTsExtensions: true,
					noLib: true
				})

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
			const { MonacoServices } = await import('@codingame/monaco-languageclient')

			MonacoServices.install(monaco)
		}

		reloadWebsocket()

		return () => {
			if (editor) {
				try {
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
