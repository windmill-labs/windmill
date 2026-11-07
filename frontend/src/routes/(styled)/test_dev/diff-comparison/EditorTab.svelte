<script lang="ts">
	import { onMount } from 'svelte'
	import { get } from 'svelte/store'
	import type { Editor, EditPredictProvider, Marker } from '@pierre/diffs/edit'
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import PierreFileEditor from '$lib/components/pierre/PierreFileEditor.svelte'
	import { getFimCompletion } from '$lib/components/copilot/lib'
	import { loadCopilot } from '$lib/components/copilot/loadCopilot'
	import { copilotInfo } from '$lib/aiStore'
	import { workspaceStore } from '$lib/stores'
	import { randomUUID } from '$lib/utils/uuid'
	import { Redo2, Undo2 } from 'lucide-svelte'
	import { lint } from './lint'
	import { REVIEW_BEFORE } from './reviewSample'

	// Changing the prediction mode recreates the editor; the key carries the draft and undo
	// history over. Per tab instance, so reopening the tab starts from the sample again.
	const editStateKey = randomUUID()
	let pierreCode = $state(REVIEW_BEFORE)
	let monacoCode = $state(REVIEW_BEFORE)
	let markers: Marker[] = $state.raw(lint(REVIEW_BEFORE))
	let editor: Editor<'file', undefined, undefined> | undefined = $state()
	let predictionMode = $state<'eager' | 'subtle' | 'off'>('eager')
	let lastPrediction: string | undefined = $state()

	const model = $derived($copilotInfo.codeCompletionModel)

	onMount(() => {
		if ($workspaceStore) loadCopilot($workspaceStore)
	})

	let lintTimer: ReturnType<typeof setTimeout> | undefined
	function onChange(code: string) {
		pierreCode = code
		clearTimeout(lintTimer)
		lintTimer = setTimeout(() => (markers = lint(code)), 300)
	}

	// Fill-in-the-middle at the caret, through the same AI proxy as Monaco's autocomplete.
	const provider: EditPredictProvider = {
		async predict(request, { signal }) {
			const providerModel = get(copilotInfo).codeCompletionModel
			if (!providerModel) throw new Error('No code completion model configured')
			const abort = new AbortController()
			signal.addEventListener('abort', () => abort.abort())
			const prefix = request.excerptText.slice(0, request.cursorOffsetInExcerpt)
			const suffix = request.excerptText.slice(request.cursorOffsetInExcerpt)
			const start = performance.now()
			const completion = await getFimCompletion(prefix, suffix, providerModel, abort)
			const prefixLines = prefix.split('\n')
			const at = {
				line: request.excerptStartLine + prefixLines.length - 1,
				character: prefixLines[prefixLines.length - 1].length
			}
			lastPrediction = `${Math.round(performance.now() - start)} ms: ${JSON.stringify(completion ?? '')}`
			if (!completion) return { edits: [], newCursor: at }
			const inserted = completion.split('\n')
			return {
				edits: [{ range: { start: at, end: at }, newText: completion }],
				newCursor:
					inserted.length === 1
						? { line: at.line, character: at.character + completion.length }
						: {
								line: at.line + inserted.length - 1,
								character: inserted[inserted.length - 1].length
							}
			}
		}
	}

	const editorOptions = $derived(
		predictionMode === 'off' ? {} : { editPrediction: { provider, mode: predictionMode } }
	)
</script>

<div class="grid gap-4 xl:grid-cols-2">
	<section class="flex min-w-0 flex-col gap-2">
		<h2 class="text-sm font-semibold">Pierre editor</h2>
		<div class="flex flex-wrap items-center gap-3 text-xs">
			<label class="flex items-center gap-1">
				Edit prediction
				<Select
					size="sm"
					items={[
						{ value: 'eager', label: 'eager (as you type)' },
						{ value: 'subtle', label: 'subtle (tap Alt)' },
						{ value: 'off', label: 'off' }
					]}
					bind:value={predictionMode}
				/>
			</label>
			<span class="text-secondary">
				{model
					? `Model: ${model.provider}/${model.model}`
					: 'No code completion model in this workspace: set one in workspace AI settings'}
			</span>
			{#if editor}
				<Button
					unifiedSize="xs"
					variant="subtle"
					iconOnly
					startIcon={{ icon: Undo2 }}
					onclick={() => editor?.undo()}
				/>
				<Button
					unifiedSize="xs"
					variant="subtle"
					iconOnly
					startIcon={{ icon: Redo2 }}
					onclick={() => editor?.redo()}
				/>
			{/if}
		</div>
		<p class="text-2xs text-hint">
			Tab accepts a prediction, Esc dismisses it. Select text for the selection action. Markers come
			from a demo linter and re-anchor as you type. Last prediction: {lastPrediction ?? '–'}
		</p>
		<div class="h-[560px] overflow-auto border">
			{#key predictionMode}
				<PierreFileEditor
					code={REVIEW_BEFORE}
					{editStateKey}
					filename="billing.ts"
					lang="typescript"
					{editorOptions}
					{markers}
					{onChange}
					onEditor={(e) => (editor = e)}
				>
					{#snippet selectionAction(ctx)}
						<div class="flex gap-1 rounded-md border bg-surface p-1 shadow-md">
							<Button
								unifiedSize="xs"
								variant="subtle"
								onclick={() => {
									ctx.replaceSelectionText(
										ctx
											.getSelectionText()
											.split('\n')
											.map((l) => `// ${l}`)
											.join('\n')
									)
									ctx.close()
								}}>Comment out</Button
							>
							<Button
								unifiedSize="xs"
								variant="subtle"
								onclick={() => {
									navigator.clipboard.writeText(ctx.getSelectionText())
									ctx.close()
								}}>Copy</Button
							>
						</div>
					{/snippet}
				</PierreFileEditor>
			{/key}
		</div>
		<span class="text-2xs text-hint">
			{pierreCode.split('\n').length} lines, {markers.length} markers
		</span>
	</section>

	<section class="flex min-w-0 flex-col gap-2">
		<h2 class="text-sm font-semibold">Monaco (SimpleEditor)</h2>
		<p class="text-2xs text-hint">
			Monaco brings the TypeScript language service (completions, hovers, real diagnostics) that
			Pierre's editor does not have.
		</p>
		<div class="h-[590px] border">
			<SimpleEditor lang="typescript" bind:code={monacoCode} class="h-full" />
		</div>
	</section>
</div>
