<script lang="ts">
	import { BROWSER } from 'esm-env'
	import { onMount } from 'svelte'

	import { editor as meditor } from 'monaco-editor'

	import '@codingame/monaco-vscode-theme-defaults-default-extension'
	import '@codingame/monaco-vscode-json-default-extension'
	import '@codingame/monaco-vscode-standalone-json-language-features'
	import '@codingame/monaco-vscode-standalone-css-language-features'
	import '@codingame/monaco-vscode-standalone-html-language-features'
	import '@codingame/monaco-vscode-standalone-typescript-language-features'
	import '@codingame/monaco-vscode-typescript-basics-default-extension'
	import '@codingame/monaco-vscode-typescript-language-features-default-extension'
	import '@codingame/monaco-vscode-go-default-extension'
	import '@codingame/monaco-vscode-javascript-default-extension'
	import '@codingame/monaco-vscode-powershell-default-extension'
	import '@codingame/monaco-vscode-python-default-extension'
	import '@codingame/monaco-vscode-shellscript-default-extension'
	import '@codingame/monaco-vscode-sql-default-extension'

	import { initializeVscode } from './vscode'
	import EditorTheme from './EditorTheme.svelte'
	import { configureMonacoWorkers } from './build_workers'

	configureMonacoWorkers()

	const SIDE_BY_SIDE_MIN_WIDTH = 700

	export let automaticLayout = true
	export let fixedOverflowWidgets = true
	export let defaultLang: string | undefined = undefined
	export let defaultModifiedLang: string | undefined = undefined
	export let defaultOriginal: string | undefined = undefined
	export let defaultModified: string | undefined = undefined
	export let readOnly = false

	let diffEditor: meditor.IStandaloneDiffEditor | undefined
	let diffDivEl: HTMLDivElement | null = null
	let editorWidth: number = SIDE_BY_SIDE_MIN_WIDTH

	async function loadDiffEditor() {
		await initializeVscode()

		diffEditor = meditor.createDiffEditor(diffDivEl!, {
			automaticLayout,
			renderSideBySide: editorWidth >= SIDE_BY_SIDE_MIN_WIDTH,
			originalEditable: false,
			readOnly,
			minimap: {
				enabled: false
			},
			fixedOverflowWidgets,
			scrollBeyondLastLine: false,
			lineDecorationsWidth: 15,
			lineNumbersMinChars: 2,
			scrollbar: { alwaysConsumeMouseWheel: false }
		})
		if (
			defaultOriginal !== undefined &&
			defaultModified !== undefined &&
			defaultLang !== undefined
		) {
			setupModel(defaultLang, defaultOriginal, defaultModified, defaultModifiedLang)
		}
	}

	export function setupModel(
		lang: string,
		original?: string,
		modified?: string,
		modifiedLang?: string
	) {
		diffEditor?.setModel({
			original: meditor.createModel('', lang),
			modified: meditor.createModel('', modifiedLang ?? lang)
		})
		if (original) {
			setOriginal(original)
		}
		if (modified) {
			setModified(modified)
		}
	}

	export function setOriginal(code: string) {
		diffEditor?.getModel()?.original?.setValue(code)
	}

	export function getOriginal(): string {
		return diffEditor?.getModel()?.original.getValue() ?? ''
	}

	export function setModified(code: string) {
		diffEditor?.getModel()?.modified?.setValue(code)
	}

	export function getModified(): string {
		return diffEditor?.getModel()?.modified.getValue() ?? ''
	}

	export function show(): void {
		diffDivEl?.classList.remove('hidden')
	}
	export function hide(): void {
		diffDivEl?.classList.add('hidden')
	}

	function onWidthChange(editorWidth: number) {
		diffEditor?.updateOptions({ renderSideBySide: editorWidth >= SIDE_BY_SIDE_MIN_WIDTH })
	}

	$: onWidthChange(editorWidth)

	onMount(() => {
		if (BROWSER) {
			loadDiffEditor()
			return () => {
				diffEditor?.dispose()
			}
		}
	})
</script>

<EditorTheme />

<div bind:this={diffDivEl} class="{$$props.class} editor" bind:clientWidth={editorWidth} />
