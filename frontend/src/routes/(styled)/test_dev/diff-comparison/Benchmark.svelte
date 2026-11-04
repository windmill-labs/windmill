<script lang="ts">
	import { tick } from 'svelte'
	import { Button } from '$lib/components/common'
	import DiffEditor from '$lib/components/DiffEditor.svelte'
	import ToolCodeDiffView from '$lib/components/copilot/chat/ToolCodeDiffView.svelte'
	import PierreDiff from '$lib/components/pierre/PierreDiff.svelte'
	import PierreCodeView from '$lib/components/pierre/PierreCodeView.svelte'
	import { switchMainThreadHighlighter } from '$lib/components/pierre/pierre'
	import { caseFileDiff, type ComparisonCase } from './cases'
	import { countNodes, formatMs, observeLongTasks, sleep, waitForSelector } from './measure'
	import { diffOptions, settingsPool, type PierreSettings } from './pierreSettings'

	let {
		diffCase,
		settings,
		onRunningChange
	}: {
		diffCase: ComparisonCase
		settings: PierreSettings
		/** Other Pierre views must unmount while running: switching backends disposes theirs. */
		onRunningChange?: (running: boolean) => void
	} = $props()

	type Renderer = {
		id: string
		label: string
		/** Matches once the first line is on screen. */
		painted: string
		/** Matches once syntax colors are applied. */
		highlighted: string
		pierre?: Partial<PierreSettings> & { virtualized?: boolean }
	}

	const PIERRE_PAINTED = '[data-line]'
	const PIERRE_HIGHLIGHTED = '[data-line] span[style]'

	const renderers: Renderer[] = [
		{
			id: 'monaco',
			label: 'Monaco',
			painted: '.view-line',
			highlighted: '.view-line span[class*="mtk"]:not(.mtk1)'
		},
		{
			id: 'native',
			label: 'Native (highlight.js)',
			painted: '[class*="diff-line-"]',
			highlighted: '[class*="hljs-"]'
		},
		...(
			[
				['shiki-js', false],
				['shiki-js', true],
				['shiki-wasm', false],
				['highlights', false],
				['highlights', true]
			] as const
		).map(([highlighter, useWorkers]) => ({
			id: `pierre-${highlighter}-${useWorkers}`,
			label: `Pierre ${highlighter}${useWorkers ? ' + workers' : ''}`,
			painted: PIERRE_PAINTED,
			highlighted: PIERRE_HIGHLIGHTED,
			pierre: { highlighter, useWorkers }
		})),
		...(['shiki-js', 'highlights'] as const).map((highlighter) => ({
			id: `pierre-codeview-${highlighter}`,
			label: `Pierre CodeView ${highlighter} (virtualized)`,
			painted: PIERRE_PAINTED,
			highlighted: PIERRE_HIGHLIGHTED,
			pierre: { virtualized: true, highlighter, useWorkers: false }
		}))
	]

	type Result = {
		run: number
		label: string
		painted?: number
		highlighted?: number
		longTasks?: number
		nodes: number
	}

	let results: Result[] = $state([])
	let current: Renderer | undefined = $state()
	let running = $state(false)
	let box: HTMLDivElement | undefined = $state()

	const currentSettings = $derived(current?.pierre ? { ...settings, ...current.pierre } : settings)
	const fileDiff = $derived(caseFileDiff(diffCase))

	async function runAll(rounds: number) {
		running = true
		onRunningChange?.(true)
		await tick()
		results = []
		for (let run = 1; run <= rounds; run++) {
			for (const r of renderers) {
				current = undefined
				await tick()
				const s = { ...settings, ...r.pierre }
				if (r.pierre && !s.useWorkers) await switchMainThreadHighlighter(s.highlighter)
				await sleep(300)
				const longTasks = observeLongTasks()
				const start = performance.now()
				current = r
				await tick()
				const painted = await waitForSelector(() => box, r.painted)
				const highlighted = await waitForSelector(() => box, r.highlighted, 10_000)
				await sleep(1000)
				results.push({
					run,
					label: r.label,
					painted: painted && painted - start,
					highlighted: highlighted && highlighted - start,
					longTasks: longTasks.stop(),
					nodes: box ? countNodes(box) : 0
				})
			}
		}
		current = undefined
		await tick()
		running = false
		onRunningChange?.(false)
	}
</script>

<section class="flex flex-col gap-2">
	<div class="flex items-center gap-2">
		<h2 class="text-sm font-semibold">Benchmark</h2>
		<span class="text-2xs text-hint">
			Mounts each renderer alone on the current case, twice. Run 1 pays for the first load of
			whatever this page has not used yet (code, grammars, workers); run 2 has those cached. A
			main-thread Pierre backend is rebuilt every time it is picked, since only one exists at a
			time, so its rows include loading that backend's languages and theme. Long tasks are
			main-thread blocks over 50 ms (Chromium only). "–" under Highlighted means no colors within 10
			s: the native view skips highlighting past 2,000 lines.
		</span>
	</div>
	<div class="flex gap-2">
		<Button unifiedSize="sm" variant="accent" disabled={running} onclick={() => runAll(2)}>
			Run benchmark
		</Button>
		{#if running && current}
			<span class="text-xs text-secondary">Rendering {current.label}…</span>
		{/if}
	</div>

	{#if results.length > 0}
		<table class="w-full text-xs">
			<thead>
				<tr class="text-left text-secondary">
					<th class="py-1 font-normal">Run</th>
					<th class="font-normal">Renderer</th>
					<th class="text-right font-normal">First line</th>
					<th class="text-right font-normal">Highlighted</th>
					<th class="text-right font-normal">Long tasks</th>
					<th class="text-right font-normal">DOM nodes</th>
				</tr>
			</thead>
			<tbody>
				{#each results as r (`${r.run}:${r.label}`)}
					<tr class="border-t border-gray-200 dark:border-gray-700">
						<td class="py-1">{r.run}</td>
						<td>{r.label}</td>
						<td class="text-right">{formatMs(r.painted)}</td>
						<td class="text-right">{formatMs(r.highlighted)}</td>
						<td class="text-right">{formatMs(r.longTasks)}</td>
						<td class="text-right">{r.nodes.toLocaleString()}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}

	<div bind:this={box} class="h-[400px] overflow-auto border {current ? '' : 'hidden'}">
		{#if current?.id === 'monaco'}
			<DiffEditor
				className="h-full"
				defaultLang={diffCase.diff.beforeLang ?? diffCase.diff.lang}
				defaultModifiedLang={diffCase.diff.lang}
				defaultOriginal={diffCase.diff.before}
				defaultModified={diffCase.diff.after}
				inlineDiff={settings.diffStyle === 'unified'}
				open
				readOnly
			/>
		{:else if current?.id === 'native'}
			<ToolCodeDiffView diff={diffCase.diff} />
		{:else if current?.pierre?.virtualized}
			<PierreCodeView
				class="h-full overflow-auto"
				items={[{ type: 'diff', id: diffCase.id, fileDiff }]}
				options={diffOptions(currentSettings)}
				workerPool={settingsPool(currentSettings)}
			/>
		{:else if current?.pierre}
			<PierreDiff
				before={diffCase.diff.before}
				after={diffCase.diff.after}
				filename={diffCase.filename}
				lang={diffCase.diff.lang}
				options={diffOptions(currentSettings)}
				workerPool={settingsPool(currentSettings)}
			/>
		{/if}
	</div>
</section>
