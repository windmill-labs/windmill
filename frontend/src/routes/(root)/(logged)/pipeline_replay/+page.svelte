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
	import { Loader2, TriangleAlert, Upload } from 'lucide-svelte'
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
		const isObject = (v: unknown): v is Record<string, unknown> =>
			typeof v === 'object' && v !== null && !Array.isArray(v)
		// A non-object payload (JSON `null`, an array, a scalar) has no `.version`
		// to read — guard before dereferencing so it toasts instead of throwing.
		if (!isObject(data)) {
			sendUserToast('Invalid recording format', true)
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
			// Load-time check on caller-controlled input (upload / `?src=` fetch) so the
			// common malformed shapes toast here; the render boundary below is the
			// catch-all for anything deeper.
			const objectArray = (v: unknown) => Array.isArray(v) && v.every(isObject)
			const optionalObjectArray = (v: unknown) => v === undefined || objectArray(v)
			const optionalRecord = (v: unknown, valid: (x: Record<string, unknown>) => boolean) =>
				v === undefined || (isObject(v) && Object.values(v).every((x) => isObject(x) && valid(x)))
			const g = data.graph as Record<string, unknown> | null
			const validGraph =
				isObject(g) &&
				objectArray(g.runnables) &&
				objectArray(g.assets) &&
				objectArray(g.edges) &&
				Array.isArray(g.triggers) &&
				g.triggers.every((t) => isObject(t) && typeof t.trigger_kind === 'string') &&
				optionalObjectArray(g.macro_edges) &&
				optionalObjectArray(g.test_edges)
			const validTimeline =
				Array.isArray(data.timeline) &&
				data.timeline.every(
					(f: unknown) =>
						isObject(f) && isObject(f.statuses) && Object.values(f.statuses).every(isObject)
				)
			const validJobs =
				isObject(data.jobs) &&
				Object.values(data.jobs).every(
					(j) => isObject(j) && isObject(j.initial_job) && objectArray(j.events)
				)
			// A sample renders `rows`/`columns` unless it carries a non-empty `error`.
			const validSamples = optionalRecord(
				data.assetSamples,
				(s) =>
					(typeof s.error === 'string' && s.error !== '') ||
					(objectArray(s.rows) && objectArray(s.columns))
			)
			const validCodes = optionalRecord(
				data.codes,
				(c) => typeof c.content === 'string' && typeof c.language === 'string'
			)
			if (!(validGraph && validTimeline && validJobs && validSamples && validCodes)) {
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
			<!-- Catch-all for a malformed recording that slips past load-time validation:
			     a render/effect crash shows the failed state instead of a blank page. -->
			<svelte:boundary onerror={() => setActiveReplay(undefined)}>
				<PipelineRecordingReplay recording={pipelineRecording} />
				{#snippet failed()}
					<div class="flex flex-col items-center justify-center h-full gap-2 text-center">
						<TriangleAlert class="text-red-500" size={28} />
						<p class="max-w-md text-sm text-secondary">
							This recording could not be replayed — it may be malformed or from an incompatible
							version.
						</p>
						<Button variant="border" size="xs" onclick={quit} startIcon={{ icon: Upload }}>
							Load another recording
						</Button>
					</div>
				{/snippet}
			</svelte:boundary>
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
