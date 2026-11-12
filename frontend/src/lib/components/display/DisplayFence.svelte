<script lang="ts">
	import { onDestroy } from 'svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { Alert, Skeleton } from '$lib/components/common'
	import { json } from 'svelte-highlight/languages'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { fenceKind, parseFence, type FenceParse } from './displayCatalog'

	const STREAM_QUIET_MS = 300

	let { code }: { code: string } = $props()

	let parsed: FenceParse = $derived(parseFence(code))
	// True while the source keeps changing in quick succession (a reply streaming in): a
	// half-written block shows a placeholder, not an error, until the text goes quiet. Starts
	// true so a stream's first chunk does not flash the error before the second arrives.
	let streaming = $state(true)
	let sawStream = false
	let logged = false
	let quietTimer: ReturnType<typeof setTimeout> | undefined

	$effect(() => {
		void code
		const burst = quietTimer !== undefined
		clearTimeout(quietTimer)
		if (burst) {
			streaming = true
			sawStream = true
		}
		quietTimer = setTimeout(() => {
			quietTimer = undefined
			streaming = false
			logOnce()
		}, STREAM_QUIET_MS)
	})

	// Counts blocks seen streaming in, i.e. written by the model in this view: not re-renders of
	// a stored transcript, nor artifacts, which arrive whole.
	function logOnce() {
		if (logged || !sawStream) return
		logged = true
		logFeatureUsage('ai_chat', 'display_block', {
			key: parsed.ok ? fenceKind(parsed.result) : 'invalid'
		})
	}

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
