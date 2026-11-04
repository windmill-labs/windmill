<script lang="ts">
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import ToolCodeDiffView from '$lib/components/copilot/chat/ToolCodeDiffView.svelte'
	import PierreDiff from '$lib/components/pierre/PierreDiff.svelte'
	import PierreCodeView from '$lib/components/pierre/PierreCodeView.svelte'
	import Benchmark from './Benchmark.svelte'
	import PierreSettingsBar from './PierreSettingsBar.svelte'
	import WorkspaceScriptPicker from './WorkspaceScriptPicker.svelte'
	import { switchMainThreadHighlighter } from '$lib/components/pierre/pierre'
	import { caseFileDiff, cases, type ComparisonCase } from './cases'
	import { DEFAULT_SETTINGS, diffOptions, settingsPool } from './pierreSettings'

	let settings = $state({ ...DEFAULT_SETTINGS })
	let virtualized = $state(false)
	let benchmarking = $state(false)
	let workspaceCase: ComparisonCase | undefined = $state()
	let activeCaseId = $state(cases[0].id)
	const activeCase = $derived(
		activeCaseId === workspaceCase?.id
			? workspaceCase
			: (cases.find((c) => c.id === activeCaseId) ?? cases[0])
	)
	const options = $derived(diffOptions(settings))
	const pool = $derived(settingsPool(settings))
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-wrap gap-2">
		{#each cases as item (item.id)}
			<Button
				unifiedSize="xs"
				variant="subtle"
				selected={activeCaseId === item.id}
				aria-pressed={activeCaseId === item.id}
				onclick={() => (activeCaseId = item.id)}>{item.label}</Button
			>
		{/each}
	</div>
	<WorkspaceScriptPicker
		onLoaded={(c) => {
			workspaceCase = c
			activeCaseId = c.id
		}}
	/>

	<div class="grid gap-4 lg:grid-cols-2">
		<section class="min-w-0">
			<h2 class="mb-2 text-sm font-semibold">Monaco</h2>
			<div class="h-[400px] overflow-hidden border">
				{#key `${activeCase.id}:${settings.diffStyle}`}
					<DiffEditor
						className="h-full"
						defaultLang={activeCase.diff.beforeLang ?? activeCase.diff.lang}
						defaultModifiedLang={activeCase.diff.lang}
						defaultOriginal={activeCase.diff.before}
						defaultModified={activeCase.diff.after}
						inlineDiff={settings.diffStyle === 'unified'}
						disableAutoInline
						open
						readOnly
					/>
				{/key}
			</div>
		</section>

		<section class="min-w-0">
			<h2 class="mb-2 text-sm font-semibold">Native (highlight.js)</h2>
			<div class="h-[400px] overflow-auto border">
				{#key activeCase.id}
					<ToolCodeDiffView diff={activeCase.diff} />
				{/key}
			</div>
		</section>
	</div>

	<section class="flex min-w-0 flex-col gap-2">
		<div class="flex items-center gap-4">
			<h2 class="text-sm font-semibold">Pierre</h2>
			<Toggle size="xs" bind:checked={virtualized} options={{ right: 'Virtualized (CodeView)' }} />
		</div>
		<PierreSettingsBar bind:settings />
		<div class="h-[400px] overflow-auto border">
			<!-- A component's worker pool is fixed at construction. -->
			{#key `${settings.useWorkers}:${settings.highlighter}:${virtualized}`}
				{#if benchmarking}
					<p class="p-4 text-xs text-hint">Hidden while the benchmark runs.</p>
				{:else}
					{#await settings.useWorkers ? undefined : switchMainThreadHighlighter(settings.highlighter) then}
						{@render pierre()}
					{/await}
				{/if}
			{/key}
		</div>
	</section>

	<Benchmark diffCase={activeCase} {settings} onRunningChange={(r) => (benchmarking = r)} />
</div>

{#snippet pierre()}
	{#if virtualized}
		<PierreCodeView
			class="h-full overflow-auto"
			items={[{ type: 'diff', id: activeCase.id, fileDiff: caseFileDiff(activeCase) }]}
			{options}
			workerPool={pool}
		/>
	{:else}
		<PierreDiff
			before={activeCase.diff.before}
			after={activeCase.diff.after}
			filename={activeCase.filename}
			lang={activeCase.diff.lang}
			beforeLang={activeCase.diff.beforeLang}
			{options}
			workerPool={pool}
		/>
	{/if}
{/snippet}
