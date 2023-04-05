<script lang="ts">
	import { browser } from '$app/environment'

	import { createHash, editorConfig, langToExt, updateOptions } from '$lib/editorUtils'
	import { editor as meditor, KeyCode, KeyMod, languages, Uri as mUri } from 'monaco-editor'
	import { createEventDispatcher, onDestroy, onMount } from 'svelte'

	import libStdContent from '$lib/es5.d.ts.txt?raw'
	import { buildWorkerDefinition } from 'monaco-editor-workers'

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

	languages.typescript.javascriptDefaults.setCompilerOptions({
		target: languages.typescript.ScriptTarget.Latest,
		allowNonTsExtensions: true,
		noLib: true
	})

	languages.json.jsonDefaults.setDiagnosticsOptions({
		validate: true,
		allowComments: false,
		schemas: [],
		enableSchemaRequest: true
	})

	let divEl: HTMLDivElement | null = null
	let editor: meditor.IStandaloneCodeEditor
	let model: meditor.ITextModel

	export let lang: string
	export let code: string = ''
	export let hash: string = createHash()
	export let cmdEnterAction: (() => void) | undefined = undefined
	export let formatAction: (() => void) | undefined = undefined
	export let automaticLayout = true
	export let extraLib: string = ''
	export let shouldBindKey: boolean = true
	export let autoHeight = false
	export let fixedOverflowWidgets = true

	const dispatch = createEventDispatcher()

	const uri = `file:///${hash}.${langToExt(lang)}`

	buildWorkerDefinition('../../../workers', import.meta.url, false)

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

	export function format() {
		if (editor) {
			code = getCode()
			editor.getAction('editor.action.formatDocument').run()
			if (formatAction) {
				formatAction()
				code = getCode()
			}
		}
	}

	export function focus() {
		editor?.focus()
	}

	let width = 0
	async function loadMonaco() {
		model = meditor.createModel(code, lang, mUri.parse(uri))

		model.updateOptions(updateOptions)
		let widgets: HTMLElement | undefined =
			document.getElementById('monaco-widgets-root') ?? undefined

		editor = meditor.create(divEl as HTMLDivElement, {
			...editorConfig(model, code, lang, automaticLayout, fixedOverflowWidgets),
			overflowWidgetsDomNode: widgets
		})

		let timeoutModel: NodeJS.Timeout | undefined = undefined
		editor.onDidChangeModelContent((event) => {
			timeoutModel && clearTimeout(timeoutModel)
			timeoutModel = setTimeout(() => {
				code = getCode()
				dispatch('change', { code })
			}, 200)
		})

		editor.onDidFocusEditorText(() => {
			dispatch('focus')

			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyS, function () {
				code = getCode()
				shouldBindKey && format && format()
			})
		})

		if (autoHeight) {
			const updateHeight = () => {
				const contentHeight = Math.min(1000, editor.getContentHeight())
				if (divEl) {
					divEl.style.height = `${contentHeight}px`
				}
				try {
					editor.layout({ width, height: contentHeight })
				} finally {
				}
			}
			editor.onDidContentSizeChange(updateHeight)
			updateHeight()
		}

		editor.onDidFocusEditorText(() => {
			editor.addCommand(KeyMod.CtrlCmd | KeyCode.KeyS, function () {
				code = getCode()
				shouldBindKey && format && format()
			})

			editor.addCommand(KeyMod.CtrlCmd | KeyCode.Enter, function () {
				code = getCode()
				shouldBindKey && cmdEnterAction && cmdEnterAction()
			})
			dispatch('focus')
		})

		editor.onDidBlurEditorText(() => {
			code = getCode()
		})

		if (lang == 'javascript') {
			const stdLib = { content: libStdContent, filePath: 'es5.d.ts' }
			if (extraLib != '') {
				languages.typescript.javascriptDefaults.setExtraLibs([
					{
						content: extraLib,
						filePath: 'windmill.d.ts'
					},
					stdLib
				])
			} else {
				languages.typescript.javascriptDefaults.setExtraLibs([stdLib])
			}
		}
	}

	onMount(() => {
		if (browser) {
			loadMonaco()
		}
	})

	onDestroy(() => {
		try {
			model && model.dispose()
			editor && editor.dispose()
		} catch (err) {}
	})
</script>

<div bind:this={divEl} class="{$$props.class ?? ''} editor" bind:clientWidth={width} />

<style lang="postcss">
	.editor {
		@apply rounded-lg p-0;
	}

	.small-editor {
		/* stylelint-disable-next-line unit-allowed-list */
		height: 26vh;
	}

	.few-lines-editor {
		/* stylelint-disable-next-line unit-allowed-list */
		height: 100px;
	}
</style>
