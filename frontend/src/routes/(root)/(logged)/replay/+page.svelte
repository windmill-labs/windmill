<script lang="ts">
	import FlowRecordingReplay from '$lib/components/recording/FlowRecordingReplay.svelte'
	import type { FlowRecording } from '$lib/components/recording/types'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '$lib/components/common'
	import FileInput from '$lib/components/common/fileInput/FileInput.svelte'
	import { Upload } from 'lucide-svelte'
	import { setActiveReplay } from '$lib/components/recording/flowRecording.svelte'

	let recording: FlowRecording | undefined = $state(undefined)

	function handleFileChange(event: CustomEvent<(string | ArrayBuffer | null)[]>) {
		const content = event.detail?.[0]
		if (!content || typeof content !== 'string') return
		try {
			const data = JSON.parse(content) as FlowRecording
			if (data.version !== 1 || !data.jobs) {
				sendUserToast('Invalid recording format', true)
				return
			}
			recording = data
		} catch (err) {
			sendUserToast('Failed to load recording: ' + err, true)
		}
	}

	function quit() {
		setActiveReplay(undefined)
		recording = undefined
	}
</script>

<div class="max-w-7xl mx-auto px-4 py-8 w-full">
	{#if recording}
		<div class="flex justify-end mb-4">
			<Button variant="border" size="xs" on:click={quit} startIcon={{ icon: Upload }}>
				Load another recording
			</Button>
		</div>
		<FlowRecordingReplay {recording} />
	{:else}
		<div class="flex flex-col items-center justify-center min-h-[60vh]">
			<div class="flex flex-col items-center gap-2 max-w-md w-full">
				<h2 class="text-lg font-semibold text-emphasis">Replay a flow recording</h2>
				<p class="text-xs text-secondary mb-2">
					Upload a recording JSON file to replay a flow execution offline.
				</p>
				<FileInput
					accept=".json"
					convertTo="text"
					class="w-full"
					on:change={handleFileChange}
				>
					Drag and drop a recording file
				</FileInput>
			</div>
		</div>
	{/if}
</div>
