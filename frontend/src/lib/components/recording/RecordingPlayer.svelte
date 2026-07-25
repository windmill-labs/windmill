<script lang="ts">
	/**
	 * Mounts a validated recording in the player its kind needs. Shared by the
	 * in-workspace page and the public one so both agree on the dispatch, the
	 * layout each kind wants and the render-failure fallback.
	 */
	import FlowRecordingReplay from './FlowRecordingReplay.svelte'
	import ScriptRecordingReplay from './ScriptRecordingReplay.svelte'
	import PipelineRecordingReplay from './PipelineRecordingReplay.svelte'
	import RawAppRecordingReplay from './RawAppRecordingReplay.svelte'
	import { setActiveReplay } from './flowRecording.svelte'
	import type { LoadedRecording } from './recordingLoad'
	import { Button } from '$lib/components/common'
	import { TriangleAlert, Upload } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		loaded: LoadedRecording
		/** Set to offer a way back to the recording picker, here and in the failure
		 * state. The public page omits it and stays chrome-less. */
		onreset?: () => void
		class?: string
	}

	let { loaded, onreset, class: className = '' }: Props = $props()

	// The pipeline and app players fill the viewport (steps left, detail right,
	// like their editors); the flow/script players keep a centered scrolling page.
	let fillsViewport = $derived(loaded.kind === 'pipeline' || loaded.kind === 'app')

	function reset() {
		setActiveReplay(undefined)
		onreset?.()
	}
</script>

<div
	class={twMerge(
		'w-full',
		fillsViewport ? 'flex flex-col h-full min-h-0' : 'max-w-7xl mx-auto',
		className
	)}
>
	{#if onreset}
		<div class="flex justify-end mb-2 shrink-0">
			<Button variant="border" size="xs" onclick={reset} startIcon={{ icon: Upload }}>
				Load another recording
			</Button>
		</div>
	{/if}
	<div class={fillsViewport ? 'flex-1 min-h-0' : ''}>
		<svelte:boundary onerror={() => setActiveReplay(undefined)}>
			{#if loaded.kind === 'app'}
				<RawAppRecordingReplay recording={loaded.recording} />
			{:else if loaded.kind === 'pipeline'}
				<PipelineRecordingReplay recording={loaded.recording} />
			{:else if loaded.kind === 'script'}
				<ScriptRecordingReplay recording={loaded.recording} />
			{:else}
				<FlowRecordingReplay recording={loaded.recording} />
			{/if}
			<!-- Shown when a malformed recording crashes on render or in an effect;
			     load-time validation and the JobLoader guards cover the rest. -->
			{#snippet failed()}
				<div class="flex flex-col items-center justify-center h-full gap-2 text-center">
					<TriangleAlert class="text-red-500" size={28} />
					<p class="max-w-md text-sm text-secondary">
						This recording could not be replayed — it may be malformed or from an incompatible
						version.
					</p>
					{#if onreset}
						<Button variant="border" size="xs" onclick={reset} startIcon={{ icon: Upload }}>
							Load another recording
						</Button>
					{/if}
				</div>
			{/snippet}
		</svelte:boundary>
	</div>
</div>
