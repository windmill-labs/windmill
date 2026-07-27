<script lang="ts">
	import RecordingPlayer from '$lib/components/recording/RecordingPlayer.svelte'
	import {
		fetchRecording,
		parseRecording,
		type LoadedRecording
	} from '$lib/components/recording/rawAppRecordingLoad'
	import { sendUserToast } from '$lib/toast'
	import FileInput from '$lib/components/common/fileInput/FileInput.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { onMount } from 'svelte'

	let loaded: LoadedRecording | undefined = $state(undefined)

	// Auto-download state: when the page is opened with `?src=<url>` it fetches
	// the recording JSON at that URL (with progress) instead of showing the
	// drag-and-drop. Lets a recording be shared as a deep link.
	let downloading = $state(false)
	let downloadPercent = $state<number | undefined>(undefined)
	let downloadedBytes = $state(0)
	let downloadError = $state<string | undefined>(undefined)

	/** Validate a parsed recording and route it to the right player. Returns false
	 * (and toasts) when the payload isn't a recognized recording. */
	function accept(data: unknown): boolean {
		const res = parseRecording(data)
		if (!res.ok) {
			sendUserToast(res.error, true)
			return false
		}
		loaded = res.loaded
		return true
	}

	function handleFileChange(event: CustomEvent<(string | ArrayBuffer | null)[]>) {
		const content = event.detail?.[0]
		if (!content || typeof content !== 'string') return
		try {
			accept(JSON.parse(content))
		} catch (err) {
			sendUserToast('Failed to load recording: ' + err, true)
		}
	}

	/** Fetch a recording JSON from `url`, streaming so a progress bar can show,
	 * then hand it to the players. Falls back to the drag-and-drop on failure. */
	async function loadFromUrl(url: string) {
		downloading = true
		downloadError = undefined
		downloadPercent = undefined
		downloadedBytes = 0
		try {
			const data = await fetchRecording(url, (bytes, total) => {
				downloadedBytes = bytes
				downloadPercent = total ? Math.round((bytes / total) * 100) : undefined
			})
			if (!accept(data)) {
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

	function fmtBytes(n: number): string {
		if (n < 1024) return `${n} B`
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`
		return `${(n / (1024 * 1024)).toFixed(1)} MB`
	}
</script>

{#if loaded}
	<RecordingPlayer {loaded} onreset={() => (loaded = undefined)} class="px-4 py-4" />
{:else if downloading}
	<div class="flex flex-col items-center justify-center min-h-[60vh] px-4">
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
	<div class="flex flex-col items-center justify-center min-h-[60vh] px-4">
		<div class="flex flex-col items-center gap-2 max-w-md w-full">
			<h2 class="text-lg font-semibold text-emphasis">Replay a recording</h2>
			<p class="text-xs text-secondary mb-2">
				Upload a recording JSON file to replay a flow, script or data-pipeline execution — or a
				raw-app session — offline.
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
