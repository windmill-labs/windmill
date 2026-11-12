<script lang="ts">
	import { onDestroy } from 'svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { Alert, Skeleton } from '$lib/components/common'
	import { json } from 'svelte-highlight/languages'
	import { parseFence, type FenceParse } from './displayCatalog'

	const STREAM_QUIET_MS = 300

	let { code }: { code: string } = $props()

	let parsed: FenceParse = $derived(parseFence(code))
	// True until the source has gone quiet: a block still streaming in shows a placeholder,
	// not the parse error of its half-written JSON.
	let streaming = $state(true)
	let quietTimer: ReturnType<typeof setTimeout> | undefined

	$effect(() => {
		void code
		clearTimeout(quietTimer)
		quietTimer = setTimeout(() => (streaming = false), STREAM_QUIET_MS)
	})

	onDestroy(() => clearTimeout(quietTimer))
</script>

{#if parsed.ok}
	<div class="bg-surface p-1" data-wm-display="ok">
		<DisplayResult result={parsed.result} noControls hideAsJson disableExpand />
	</div>
{:else if streaming}
	<div class="p-2" data-wm-display="pending">
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
