<script lang="ts">
	import { Button } from '$lib/components/common'
	import { getAstNode, type HastNode } from 'svelte-exmarkdown'
	import { editor as meditor } from 'monaco-editor'
	import { getContext } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import { initializeVscode } from '$lib/components/vscode'
	import type { AIChatContext } from './core'

	const astNode = getAstNode()

	const {
		originalCode,
		loading: loadingContext,
		currentReply,
		applyCode
	} = getContext<AIChatContext>('AIChatContext')

	$: code = $astNode.children?.[0]?.children?.[0]?.value

	$: language =
		($astNode.children?.[0]?.properties?.class as string | undefined)?.split('-')[1] ?? 'typescript'

	let loading = true
	function shouldStopLoading(astNode: HastNode, replying: boolean) {
		if (!replying || $currentReply.length > (astNode.position?.end.offset ?? 0)) {
			loading = false
		}
	}
	$: shouldStopLoading($astNode, $loadingContext)

	let diffEl: HTMLDivElement | undefined
	let diffEditor: meditor.IStandaloneDiffEditor | undefined
	async function setDiffEditor(diffEl: HTMLDivElement) {
		await initializeVscode()

		diffEditor = meditor.createDiffEditor(diffEl, {
			automaticLayout: true,
			renderSideBySide: false,
			hideUnchangedRegions: {
				enabled: true
			},
			originalEditable: false,
			readOnly: true,
			renderGutterMenu: false,
			renderOverviewRuler: false,
			scrollBeyondLastLine: false,
			overviewRulerLanes: 0,
			lineNumbersMinChars: 0,
			lightbulb: {
				enabled: meditor.ShowLightbulbIconMode.Off
			},
			scrollbar: {
				alwaysConsumeMouseWheel: false
			}
		})

		diffEditor.setModel({
			original: meditor.createModel($originalCode, language),
			modified: meditor.createModel(code ?? '', language)
		})

		const originalEditor = diffEditor.getOriginalEditor()
		const modifiedEditor = diffEditor.getModifiedEditor()

		originalEditor.onDidContentSizeChange((e) => {
			diffEl.style.height = `${e.contentHeight}px`
		})

		modifiedEditor.onDidContentSizeChange((e) => {
			diffEl.style.height = `${e.contentHeight}px`
		})

		updateCode(code ?? '')
	}

	function updateCode(code: string) {
		const modified = diffEditor?.getModifiedEditor()

		if (!modified) return

		const modifiedModel = modified.getModel()
		if (modifiedModel) {
			modifiedModel.setValue(code ?? '')
		}
	}
	$: updateCode(code ?? '')

	$: diffEl && setDiffEditor(diffEl)
</script>

<div class="flex flex-col gap-0.5 rounded-lg relative not-prose">
	<div class="flex justify-end">
		<Button
			color="dark"
			size="xs2"
			on:click={() => {
				applyCode(code ?? '')
			}}
		>
			Apply
		</Button>
	</div>

	<div
		class="relative w-full border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden"
	>
		{#if loading}
			<div class="flex flex-row gap-1 p-2 items-center justify-center">
				<Loader2 class="w-4 h-4 animate-spin" /> Generating code...
			</div>
		{:else}
			<div bind:this={diffEl} class="w-full h-full" />
		{/if}
	</div>
</div>
