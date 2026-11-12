<script lang="ts">
	import { onDestroy } from 'svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { Alert, Skeleton } from '$lib/components/common'
	import { json } from 'svelte-highlight/languages'
	import ChartResult from './ChartResult.svelte'
	import { fenceItems, parseFence, tableRows, type FenceParse } from './displayCatalog'

	const STREAM_QUIET_MS = 300

	let {
		code,
		staticRender = false
	}: {
		code: string
		/** Plain tables instead of the interactive grid, for print: a paginated grid on paper
		 * keeps only its first page of rows. */
		staticRender?: boolean
	} = $props()

	let parsed: FenceParse = $derived(parseFence(code))
	// True until the source has gone quiet: a block still streaming in shows a placeholder,
	// not the parse error of its half-written JSON.
	let streaming = $state(true)
	let quietTimer: ReturnType<typeof setTimeout> | undefined

	$effect(() => {
		void code
		streaming = true
		clearTimeout(quietTimer)
		quietTimer = setTimeout(() => (streaming = false), STREAM_QUIET_MS)
	})

	onDestroy(() => clearTimeout(quietTimer))
</script>

{#if parsed.ok && staticRender}
	<div class="flex flex-col gap-2" data-wm-display="ok">
		{#each fenceItems(parsed.result) as item, i (i)}
			{#if 'chart' in item}
				<ChartResult chart={item.chart} />
			{:else}
				{@const table = tableRows(item)}
				<!-- A plain table on purpose: the document's own styles lay it out for paper. -->
				<table>
					<thead>
						<tr>
							{#each table.columns as column, c (c)}<th>{column}</th>{/each}
						</tr>
					</thead>
					<tbody>
						{#each table.rows as row, r (r)}
							<tr
								>{#each row as cell, c (c)}<td>{cell ?? ''}</td>{/each}</tr
							>
						{/each}
					</tbody>
				</table>
			{/if}
		{/each}
	</div>
{:else if parsed.ok}
	<div class="bg-surface p-1" data-wm-display="ok">
		<DisplayResult result={parsed.result} noControls hideAsJson disableExpand />
	</div>
{:else if streaming}
	<div class="p-2" data-wm-display="pending" data-display-pending="true">
		<Skeleton layout={[[12]]} />
	</div>
{:else}
	<div class="flex flex-col" data-wm-display="invalid">
		<Alert type="warning" size="xs" title="This display block could not be rendered" notRounded>
			<pre class="whitespace-pre-wrap text-2xs">{parsed.error}</pre>
		</Alert>
		<HighlightCode
			className="px-3 py-2.5 !text-xs"
			{code}
			highlightLanguage={json}
			language={undefined}
			buttonsOnHover
			customScrollbarX
		/>
	</div>
{/if}
