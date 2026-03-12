<script lang="ts">
	import FlowRecordingReplay from '$lib/components/recording/FlowRecordingReplay.svelte'
	import ScriptRecordingReplay from '$lib/components/recording/ScriptRecordingReplay.svelte'
	import type { FlowRecording, ScriptRecording } from '$lib/components/recording/types'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '$lib/components/common'
	import FileInput from '$lib/components/common/fileInput/FileInput.svelte'
	import { Upload } from 'lucide-svelte'
	import { setActiveReplay } from '$lib/components/recording/flowRecording.svelte'

	let flowRecording: FlowRecording | undefined = $state(undefined)
	let scriptRecording: ScriptRecording | undefined = $state(undefined)

	function handleFileChange(event: CustomEvent<(string | ArrayBuffer | null)[]>) {
		const content = event.detail?.[0]
		if (!content || typeof content !== 'string') return
		try {
			const data = JSON.parse(content)
			if (data.version !== 1) {
				sendUserToast('Invalid recording format', true)
				return
			}
			if (data.type === 'script') {
				if (!data.job) {
					sendUserToast('Invalid script recording format', true)
					return
				}
				scriptRecording = data as ScriptRecording
				flowRecording = undefined
			} else {
				// Flow recording (type === 'flow' or type is absent for backwards compat)
				if (!data.jobs) {
					sendUserToast('Invalid flow recording format', true)
					return
				}
				flowRecording = data as FlowRecording
				scriptRecording = undefined
			}
		} catch (err) {
			sendUserToast('Failed to load recording: ' + err, true)
		}
	}

	function quit() {
		setActiveReplay(undefined)
		flowRecording = undefined
		scriptRecording = undefined
	}
</script>

<div class="max-w-7xl mx-auto px-4 py-8 w-full">
	{#if flowRecording}
		<div class="flex justify-end mb-4">
			<Button variant="border" size="xs" on:click={quit} startIcon={{ icon: Upload }}>
				Load another recording
			</Button>
		</div>
		<FlowRecordingReplay recording={flowRecording} />
	{:else if scriptRecording}
		<div class="flex justify-end mb-4">
			<Button variant="border" size="xs" on:click={quit} startIcon={{ icon: Upload }}>
				Load another recording
			</Button>
		</div>
		<ScriptRecordingReplay recording={scriptRecording} />
	{:else}
		<div class="flex flex-col items-center justify-center min-h-[60vh]">
			<div class="flex flex-col items-center gap-2 max-w-md w-full">
				<h2 class="text-lg font-semibold text-emphasis">Replay a recording</h2>
				<p class="text-xs text-secondary mb-2">
					Upload a recording JSON file to replay a flow or script execution offline.
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
