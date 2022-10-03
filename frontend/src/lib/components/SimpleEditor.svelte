<script lang="ts" context="module">
	import * as monaco from 'monaco-editor'

	monaco.languages.typescript.javascriptDefaults.setCompilerOptions({
		target: monaco.languages.typescript.ScriptTarget.Latest,
		allowNonTsExtensions: true,
		noLib: true
	})

	monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
		validate: true,
		allowComments: false,
		schemas: [],
		enableSchemaRequest: true
	})
</script>

<script lang="ts">
	import { browser, dev } from '$app/env'

	import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
	import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
	import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'

	import { buildWorkerDefinition } from 'monaco-editor-workers'

	import { createEventDispatcher, onDestroy, onMount } from 'svelte'
	import { createHash, editorConfig, langToExt, updateOptions } from '$lib/editorUtils'

	let divEl: HTMLDivElement | null = null
	let editor: monaco.editor.IStandaloneCodeEditor

	export let lang: string
	export let code: string = ''
	export let hash: string = createHash()
	export let cmdEnterAction: (() => void) | undefined = undefined
	export let formatAction: (() => void) | undefined = undefined
	export let automaticLayout = true
	export let extraLib: string = ''
	export let extraLibPath: string = ''
	export let shouldBindKey: boolean = false

	const dispatch = createEventDispatcher()

	const uri = `file:///${hash}.${langToExt(lang)}`

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
		return editor?.getValue() ?? ''
	}

	export function insertAtCursor(code: string): void {
		if (editor) {
			editor.trigger('keyboard', 'type', { text: code })
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
				code = getCode()
			}
		}
	}

	async function loadMonaco() {
		const model = monaco.editor.createModel(code, lang, monaco.Uri.parse(uri))

		model.updateOptions(updateOptions)

		editor = monaco.editor.create(
			divEl as HTMLDivElement,
			editorConfig(model, code, lang, automaticLayout)
		)

		if (shouldBindKey) {
			editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, function () {
				format()
			})

			editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, function () {
				if (cmdEnterAction) {
					code = getCode()
					cmdEnterAction()
				}
			})
		}

		let timeoutModel: NodeJS.Timeout | undefined = undefined
		editor.onDidChangeModelContent((event) => {
			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				code = getCode()
			}, 500)
			dispatch('change')
		})

		editor.onDidFocusEditorText(() => {
			dispatch('focus')
		})

		editor.onDidBlurEditorText(() => {
			code = getCode()
			dispatch('blur')
		})

		if (lang == 'javascript' && extraLib != '' && extraLibPath != '') {
			monaco.languages.typescript.javascriptDefaults.setExtraLibs([
				{
					content: extraLib,
					filePath: extraLibPath
				}
			])
		}
	}

	onMount(() => {
		if (browser) {
			loadMonaco()
		}
	})

	onDestroy(() => {
		try {
			editor && editor.dispose()
		} catch (err) {}
	})
</script>

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
