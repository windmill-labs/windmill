<script lang="ts">
	/**
	 * Record control for the in-workspace raw-app viewer: captures the session as
	 * a step-per-interaction recording (see rawAppRecording) and downloads it as
	 * JSON, replayable offline on /pipeline_replay.
	 */
	import { base } from '$lib/base'
	import { Button } from '$lib/components/common'
	import { createRawAppRecording } from '$lib/components/recording/rawAppRecording.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Circle, Square } from 'lucide-svelte'
	import { onDestroy } from 'svelte'

	interface Props {
		/** The app bundle's iframe. Recording needs it same-origin, which holds
		 * unless the publisher opted the app into sandbox isolation. */
		iframe: HTMLIFrameElement | undefined
		workspace: string
		path: string
	}

	let { iframe, workspace, path }: Props = $props()

	const recorder = createRawAppRecording()

	function start() {
		if (!iframe) return
		if (!recorder.start(iframe, { appPath: path, workspace })) {
			sendUserToast(
				'Cannot record this app: its bundle runs sandbox-isolated, so its DOM is not readable',
				true
			)
			return
		}
		sendUserToast('Recording started — interact with the app, then stop to download it')
	}

	function stop() {
		const recording = recorder.stop()
		recorder.download(recording)
		sendUserToast(
			`Recording saved with ${recording.steps.length} step${recording.steps.length === 1 ? '' : 's'}. Replay it on ${base}/pipeline_replay`
		)
	}

	onDestroy(() => {
		if (recorder.active) recorder.stop()
	})
</script>

<div class="fixed bottom-4 left-4 z-50">
	{#if recorder.active}
		<div
			class="flex items-center gap-2 rounded-md border bg-surface px-2 py-1 shadow-md text-xs text-primary"
		>
			<Circle size={10} class="text-red-500 animate-pulse" fill="currentColor" />
			<span>{recorder.stepCount} step{recorder.stepCount === 1 ? '' : 's'}</span>
			<Button size="xs" variant="border" startIcon={{ icon: Square }} onclick={stop}>
				Stop & download
			</Button>
		</div>
	{:else}
		<Button size="xs" variant="subtle" startIcon={{ icon: Circle }} onclick={start}>Record</Button>
	{/if}
</div>
