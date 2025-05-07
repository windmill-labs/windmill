<script lang="ts">
	import { BROWSER } from 'esm-env'
	import { createEventDispatcher, onMount } from 'svelte'

	// import '@codingame/monaco-vscode-standalone-languages'
	import '@codingame/monaco-vscode-standalone-json-language-features'
	import '@codingame/monaco-vscode-standalone-typescript-language-features'
	import { editor as meditor } from 'monaco-editor'

	import { initializeVscode } from './vscode'
	import EditorTheme from './EditorTheme.svelte'
	import { buildWorkerDefinition } from '$lib/monaco_workers/build_workers'
	import Button from '$lib/components/common/button/Button.svelte'

	buildWorkerDefinition()

	const SIDE_BY_SIDE_MIN_WIDTH = 700

	export let automaticLayout = true
	export let fixedOverflowWidgets = true
	export let defaultLang: string | undefined = undefined
	export let defaultModifiedLang: string | undefined = undefined
	export let defaultOriginal: string | undefined = undefined
	export let defaultModified: string | undefined = undefined
	export let readOnly = false
	export let showButtons = false

	let diffEditor: meditor.IStandaloneDiffEditor | undefined
	let diffDivEl: HTMLDivElement | null = null
	let editorWidth: number = SIDE_BY_SIDE_MIN_WIDTH

	export let open = false
	async function loadDiffEditor() {
		await initializeVscode()

		if (!diffDivEl) {
			return
		}

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
		defaultOriginal = code
	}

	export function getOriginal(): string {
		return diffEditor?.getModel()?.original.getValue() ?? ''
	}

	export function setModified(code: string) {
		diffEditor?.getModel()?.modified?.setValue(code)
		defaultModified = code
	}

	export function getModified(): string {
		return diffEditor?.getModel()?.modified.getValue() ?? ''
	}

	export function show(): void {
		open = true
	}
	export function hide(): void {
		open = false
	}

	function onWidthChange(editorWidth: number) {
		diffEditor?.updateOptions({ renderSideBySide: editorWidth >= SIDE_BY_SIDE_MIN_WIDTH })
	}

	$: onWidthChange(editorWidth)

	$: open && diffDivEl && loadDiffEditor()

	onMount(() => {
		if (BROWSER) {
			return () => {
				diffEditor?.dispose()
			}
		}
	})

	const dispatch = createEventDispatcher<{
		hideDiffMode: void
		seeHistory: void
	}>()
</script>

{#if open}
	<EditorTheme />
	<div
		bind:this={diffDivEl}
		class="{$$props.class} editor nonmain-editor"
		bind:clientWidth={editorWidth}
	></div>
	{#if showButtons}
		<div
			class="absolute flex flex-row gap-2 bottom-10 left-1/2 z-10 -translate-x-1/2 rounded-md p-1 w-full justify-center"
		>
			<Button on:click={() => dispatch('seeHistory')} variant="contained" size="sm"
				>See changes history</Button
			>
			<Button on:click={() => dispatch('hideDiffMode')} variant="contained" size="sm" color="red"
				>Quit diff mode</Button
			>
		</div>
	{/if}
{/if}
