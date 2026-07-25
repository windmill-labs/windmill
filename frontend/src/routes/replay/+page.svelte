<script lang="ts">
	/**
	 * Public, chrome-less player for a raw-app session recording — no login, and
	 * embeddable in an iframe. Windmill stores nothing: the recording is either a
	 * file the visitor opens locally or a URL they host themselves (`?src=`), so a
	 * demo can live next to the docs or README that links it.
	 */
	import RawAppRecordingReplay from '$lib/components/recording/RawAppRecordingReplay.svelte'
	import {
		fetchRecording,
		isAppRecording,
		MAX_RECORDING_BYTES
	} from '$lib/components/recording/rawAppRecordingLoad'
	import type { RawAppRecording } from '$lib/components/recording/types'
	import { Loader2, TriangleAlert, Upload } from 'lucide-svelte'
	import { onMount } from 'svelte'

	let recording = $state<RawAppRecording | undefined>(undefined)
	let loading = $state(false)
	let error = $state<string | undefined>(undefined)
	let progress = $state<number | undefined>(undefined)

	/** Recording kinds the in-workspace player handles. Named rather than echoed:
	 * `type` comes from whatever `?src=` points at, so an arbitrary payload must
	 * never reach the page as text. */
	const IN_WORKSPACE_KINDS = ['script', 'flow', 'pipeline']

	function accept(data: unknown): boolean {
		if (!isAppRecording(data)) {
			// Flow/script/pipeline recordings replay job streams, which need a session:
			// they belong to the in-workspace player, not this public page.
			const kind = (data as any)?.type
			// Carry the source along so the in-workspace player can pick it up without
			// the visitor having to reconstruct the URL.
			const src = new URL(window.location.href).searchParams.get('src')
			const where = `/pipeline_replay${src ? `?src=${encodeURIComponent(src)}` : ''}`
			error = IN_WORKSPACE_KINDS.includes(kind)
				? `This is a ${kind} recording — open it in your workspace at ${where}.`
				: 'That file is not a raw-app session recording this player understands.'
			return false
		}
		error = undefined
		recording = data
		return true
	}

	async function loadFromUrl(src: string) {
		loading = true
		error = undefined
		try {
			accept(
				await fetchRecording(src, (loaded, total) => {
					progress = total ? Math.round((loaded / total) * 100) : undefined
				})
			)
		} catch (e) {
			error = `Could not load the recording: ${e instanceof Error ? e.message : e}`
		} finally {
			loading = false
		}
	}

	async function onFile(e: Event) {
		const input = e.target as HTMLInputElement
		const file = input.files?.[0]
		// Cleared so picking the same file again after an error still fires `change`.
		input.value = ''
		if (!file) return
		// The same cap the URL path enforces while streaming: reading a multi-hundred-MB
		// file into a string and parsing it would take the tab down before validation.
		if (file.size > MAX_RECORDING_BYTES) {
			error = `That file is too large (${file.size} bytes).`
			return
		}
		try {
			accept(JSON.parse(await file.text()))
		} catch (err) {
			error = `Could not read the file: ${err instanceof Error ? err.message : err}`
		}
	}

	onMount(() => {
		const src = new URL(window.location.href).searchParams.get('src')
		if (src) loadFromUrl(src)
	})
</script>

<svelte:head><title>Replay{recording ? ` — ${recording.app_path}` : ''}</title></svelte:head>

<div class="h-screen w-screen p-3 bg-surface">
	{#if recording}
		<RawAppRecordingReplay {recording} />
	{:else if loading}
		<div class="h-full flex flex-col items-center justify-center gap-2 text-sm text-secondary">
			<Loader2 size={24} class="animate-spin text-blue-500" />
			Downloading the recording{progress !== undefined ? ` — ${progress}%` : ''}…
		</div>
	{:else}
		<div class="h-full flex flex-col items-center justify-center gap-3 text-center px-4">
			<h1 class="text-lg font-semibold text-emphasis">Replay an app recording</h1>
			<p class="text-xs text-secondary max-w-lg">
				Open a recording file, or point this page at one you host yourself with
				<span class="font-mono">?src=&lt;url&gt;</span>. Nothing is uploaded to Windmill — the
				recording is read in your browser.
			</p>
			{#if error}
				<p class="flex items-center gap-1 text-xs text-red-600 dark:text-red-400">
					<TriangleAlert size={14} />
					{error}
				</p>
			{/if}
			<label
				class="inline-flex items-center gap-2 text-xs border rounded-md px-3 py-2 cursor-pointer hover:bg-surface-hover"
			>
				<Upload size={14} /> Choose a recording file
				<input type="file" accept=".json" class="hidden" onchange={onFile} />
			</label>
		</div>
	{/if}
</div>
