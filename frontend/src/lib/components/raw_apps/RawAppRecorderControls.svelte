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
		sendUserToast(
			"Recording started. The app's DOM and the values you type are captured (passwords are " +
				'masked; add data-wm-no-record to an element to leave it out).'
		)
	}

	function stop() {
		const recording = recorder.stop()
		recorder.download(recording)
		sendUserToast(
			`Recording saved with ${recording.steps.length} step${recording.steps.length === 1 ? '' : 's'}. Replay it on ${base}/pipeline_replay`
		)
	}

	onDestroy(() => {
		// Leaving the page ends the session — hand over what was recorded rather
		// than drop minutes of it on a stray navigation, and say so, since the
		// download is one the user did not click for.
		if (!recorder.active) return
		recorder.download(recorder.stop())
		sendUserToast('Left the app — the recording so far was downloaded')
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
