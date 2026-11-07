<script lang="ts">
	import { tick } from 'svelte'
	import { parseDiffFromFile, type CodeViewItem } from '@pierre/diffs'
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import PierreCodeView from '$lib/components/pierre/PierreCodeView.svelte'
	import { switchMainThreadHighlighter } from '$lib/components/pierre/pierre'
	import { countNodes, formatMs, observeLongTasks, sleep, waitForSelector } from './measure'
	import { DEFAULT_SETTINGS, diffOptions, useSettingsPool } from './pierreSettings.svelte'
	import PierreSettingsBar from './PierreSettingsBar.svelte'

	type Side = 'pierre' | 'monaco'
	type Stats = { painted?: number; longTasks?: number; nodes: number }

	// More Monaco editors than this freezes the tab for minutes.
	const MONACO_MAX = 200
	let count = $state(50)
	let settings = $state({ ...DEFAULT_SETTINGS, useWorkers: true })
	let shown: Side | undefined = $state()
	let stats: Partial<Record<Side, Stats>> = $state({})
	let box: HTMLDivElement | undefined = $state()

	function fileVersions(i: number) {
		const lines = Array.from(
			{ length: 40 },
			(_, l) => `export const value_${i}_${l} = compute(${l}, 'item ${i}')`
		)
		const after = [...lines]
		after[5] = `export const value_${i}_5 = compute(5, 'item ${i}', { cached: true })`
		after.splice(
			20,
			2,
			`// merged 20 and 21`,
			`export const merged_${i} = [value_${i}_20, value_${i}_21]`
		)
		after.push(`export default value_${i}_0`)
		return { name: `f/module_${i}/index.ts`, before: lines.join('\n'), after: after.join('\n') }
	}

	const files = $derived(Array.from({ length: count }, (_, i) => fileVersions(i)))
	const items: CodeViewItem<undefined>[] = $derived(
		files.map((f) => ({
			type: 'diff',
			id: f.name,
			fileDiff: parseDiffFromFile(
				{ name: f.name, contents: f.before },
				{ name: f.name, contents: f.after }
			)
		}))
	)
	const pool = useSettingsPool(() => settings)

	async function show(side: Side) {
		stats[side] = undefined
		shown = undefined
		await tick()
		if (side === 'pierre' && !settings.useWorkers) {
			await switchMainThreadHighlighter(settings.highlighter)
		}
		await sleep(300)
		const longTasks = observeLongTasks()
		const start = performance.now()
		shown = side
		await tick()
		const painted = await waitForSelector(
			() => box,
			side === 'pierre' ? '[data-line]' : '.view-line'
		)
		await sleep(1500)
		stats[side] = {
			painted: painted && painted - start,
			longTasks: longTasks.stop(),
			nodes: box ? countNodes(box) : 0
		}
	}
</script>

<div class="flex flex-col gap-3">
	<p class="text-2xs text-hint">
		A review of many changed files in one scroll, as a deployment or fork-merge review would show
		them. Pierre's CodeView virtualizes the whole list; Monaco needs one editor per file.
	</p>
	<div class="flex flex-wrap items-center gap-3 text-xs">
		<label class="flex items-center gap-1">
			Files
			<Select
				size="sm"
				items={[10, 50, 200, 1000].map((n) => ({ value: n, label: `${n}` }))}
				bind:value={count}
			/>
		</label>
		<Button unifiedSize="sm" variant="accent" onclick={() => show('pierre')}>Render Pierre</Button>
		<Button unifiedSize="sm" variant="default" onclick={() => show('monaco')}>
			Render Monaco{count > MONACO_MAX ? ` (first ${MONACO_MAX})` : ''}
		</Button>
		{#each ['pierre', 'monaco'] as const as side (side)}
			{@const s = stats[side]}
			{#if s}
				<span class="text-secondary">
					{side}: first line {formatMs(s.painted)}, long tasks {formatMs(s.longTasks)},
					{s.nodes.toLocaleString()} nodes
				</span>
			{/if}
		{/each}
	</div>
	<PierreSettingsBar bind:settings />

	<div bind:this={box} class="h-[640px] overflow-auto border">
		{#if shown === 'pierre'}
			{#key `${settings.useWorkers}:${settings.highlighter}`}
				{#await settings.useWorkers ? undefined : switchMainThreadHighlighter(settings.highlighter) then}
					<PierreCodeView
						class="h-full overflow-auto"
						{items}
						options={diffOptions(settings)}
						workerPool={pool.current}
					/>
				{/await}
			{/key}
		{:else if shown === 'monaco'}
			{#each files.slice(0, MONACO_MAX) as f (f.name)}
				<div class="border-b px-2 py-1 text-xs font-semibold">{f.name}</div>
				<div class="h-[300px]">
					<DiffEditor
						className="h-full"
						defaultLang="typescript"
						defaultModifiedLang="typescript"
						defaultOriginal={f.before}
						defaultModified={f.after}
						inlineDiff={settings.diffStyle === 'unified'}
						disableAutoInline
						open
						readOnly
					/>
				</div>
			{/each}
		{/if}
	</div>
</div>
