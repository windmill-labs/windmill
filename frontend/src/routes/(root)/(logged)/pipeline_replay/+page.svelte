<script lang="ts">
	import FlowRecordingReplay from '$lib/components/recording/FlowRecordingReplay.svelte'
	import ScriptRecordingReplay from '$lib/components/recording/ScriptRecordingReplay.svelte'
	import PipelineRecordingReplay from '$lib/components/recording/PipelineRecordingReplay.svelte'
	import type {
		FlowRecording,
		PipelineRecording,
		ScriptRecording
	} from '$lib/components/recording/types'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '$lib/components/common'
	import FileInput from '$lib/components/common/fileInput/FileInput.svelte'
	import { Loader2, Upload } from 'lucide-svelte'
	import { setActiveReplay } from '$lib/components/recording/flowRecording.svelte'
	import { onMount } from 'svelte'

	let flowRecording: FlowRecording | undefined = $state(undefined)
	let scriptRecording: ScriptRecording | undefined = $state(undefined)
	let pipelineRecording: PipelineRecording | undefined = $state(undefined)

	// Auto-download state: when the page is opened with `?src=<url>` it fetches
	// the recording JSON at that URL (with progress) instead of showing the
	// drag-and-drop. Lets a recording be shared as a deep link.
	let downloading = $state(false)
	let downloadPercent = $state<number | undefined>(undefined)
	let downloadedBytes = $state(0)
	let downloadError = $state<string | undefined>(undefined)

	function reset() {
		flowRecording = undefined
		scriptRecording = undefined
		pipelineRecording = undefined
	}

	/** Parse recording JSON text and route it to the right player. Returns false
	 * (and toasts) when the payload isn't a recognized recording. */
	function loadRecordingFromText(content: string): boolean {
		let data: any
		try {
			data = JSON.parse(content)
		} catch (err) {
			sendUserToast('Failed to load recording: ' + err, true)
			return false
		}
		if (data.version !== 1) {
			sendUserToast('Invalid recording format', true)
			return false
		}
		if (data.type === 'script') {
			if (!data.job) {
				sendUserToast('Invalid script recording format', true)
				return false
			}
			reset()
			scriptRecording = data as ScriptRecording
		} else if (data.type === 'pipeline') {
			// Validate the nested shapes the player/canvas iterate — the graph's
			// `runnables`/`assets` arrays and each timeline frame's `statuses` object —
			// not just the top-level containers. This input is caller-controlled
			// (file upload or a `?src=` fetch), so a structurally wrong payload must
			// hit the toast rather than crash AssetGraphCanvas or the timeline stepper.
			const isObject = (v: unknown): v is Record<string, unknown> =>
				typeof v === 'object' && v !== null && !Array.isArray(v)
			const g = data.graph
			const validPipeline =
				isObject(g) &&
				Array.isArray(g.runnables) &&
				Array.isArray(g.assets) &&
				Array.isArray(g.edges) &&
				Array.isArray(data.timeline) &&
				data.timeline.every((f: unknown) => isObject(f) && isObject(f.statuses)) &&
				isObject(data.jobs)
			if (!validPipeline) {
				sendUserToast('Invalid pipeline recording format', true)
				return false
			}
			reset()
			pipelineRecording = data as PipelineRecording
		} else {
			// Flow recording (type === 'flow' or type is absent for backwards compat)
			if (!data.jobs) {
				sendUserToast('Invalid flow recording format', true)
				return false
			}
			reset()
			flowRecording = data as FlowRecording
		}
		return true
	}

	function handleFileChange(event: CustomEvent<(string | ArrayBuffer | null)[]>) {
		const content = event.detail?.[0]
		if (!content || typeof content !== 'string') return
		loadRecordingFromText(content)
	}

	/** Fetch a recording JSON from `url`, streaming so a progress bar can show,
	 * then hand it to the players. Falls back to the drag-and-drop on failure. */
	async function loadFromUrl(url: string) {
		downloading = true
		downloadError = undefined
		downloadPercent = undefined
		downloadedBytes = 0
		try {
			const res = await fetch(url)
			if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText}`)
			const total = Number(res.headers.get('content-length')) || 0
			const reader = res.body?.getReader()
			let text: string
			if (reader) {
				const chunks: Uint8Array[] = []
				for (;;) {
					const { done, value } = await reader.read()
					if (done) break
					if (value) {
						chunks.push(value)
						downloadedBytes += value.length
						if (total) downloadPercent = Math.round((downloadedBytes / total) * 100)
					}
				}
				text = await new Blob(chunks as BlobPart[]).text()
			} else {
				text = await res.text()
			}
			if (!loadRecordingFromText(text)) {
				downloadError = 'The downloaded file is not a valid recording.'
			}
		} catch (err) {
			downloadError = `Could not download the recording: ${err instanceof Error ? err.message : err}`
		} finally {
			downloading = false
		}
	}

	onMount(() => {
		const src = new URL(window.location.href).searchParams.get('src')
		if (src) loadFromUrl(src)
	})

	function quit() {
		setActiveReplay(undefined)
		reset()
	}

	function fmtBytes(n: number): string {
		if (n < 1024) return `${n} B`
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`
		return `${(n / (1024 * 1024)).toFixed(1)} MB`
	}
</script>

<!-- The pipeline player fills the viewport (graph left, detail right, like the
     pipeline editor); the flow/script players keep the centered scrolling page. -->
<div
	class={pipelineRecording
		? 'flex flex-col h-full w-full px-4 py-4 min-h-0'
		: 'max-w-7xl mx-auto px-4 py-8 w-full'}
>
	{#if flowRecording}
		<div class="flex justify-end mb-4">
			<Button variant="border" size="xs" onclick={quit} startIcon={{ icon: Upload }}>
				Load another recording
			</Button>
		</div>
		<FlowRecordingReplay recording={flowRecording} />
	{:else if scriptRecording}
		<div class="flex justify-end mb-4">
			<Button variant="border" size="xs" onclick={quit} startIcon={{ icon: Upload }}>
				Load another recording
			</Button>
		</div>
		<ScriptRecordingReplay recording={scriptRecording} />
	{:else if pipelineRecording}
		<div class="flex justify-end mb-2 shrink-0">
			<Button variant="border" size="xs" onclick={quit} startIcon={{ icon: Upload }}>
				Load another recording
			</Button>
		</div>
		<div class="flex-1 min-h-0">
			<PipelineRecordingReplay recording={pipelineRecording} />
		</div>
	{:else if downloading}
		<div class="flex flex-col items-center justify-center min-h-[60vh]">
			<div class="flex flex-col items-center gap-3 max-w-md w-full">
				<Loader2 class="animate-spin text-blue-500" size={28} />
				<h2 class="text-lg font-semibold text-emphasis">Downloading recording…</h2>
				{#if downloadPercent !== undefined}
					<div class="w-full h-2 rounded-full bg-surface-secondary overflow-hidden">
						<div class="h-full bg-blue-500 transition-all" style="width: {downloadPercent}%"></div>
					</div>
					<p class="text-2xs text-tertiary">{downloadPercent}% · {fmtBytes(downloadedBytes)}</p>
				{:else}
					<p class="text-2xs text-tertiary">{fmtBytes(downloadedBytes)}</p>
				{/if}
			</div>
		</div>
	{:else}
		<div class="flex flex-col items-center justify-center min-h-[60vh]">
			<div class="flex flex-col items-center gap-2 max-w-md w-full">
				<h2 class="text-lg font-semibold text-emphasis">Replay a recording</h2>
				<p class="text-xs text-secondary mb-2">
					Upload a recording JSON file to replay a flow, script or data-pipeline execution offline.
				</p>
				{#if downloadError}
					<p class="text-xs text-red-600 dark:text-red-400 mb-1 text-center">{downloadError}</p>
				{/if}
				<FileInput accept=".json" convertTo="text" class="w-full" on:change={handleFileChange}>
					Drag and drop a recording file
				</FileInput>
			</div>
		</div>
	{/if}
</div>
