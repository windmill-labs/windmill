<script lang="ts">
	/**
	 * Public, chrome-less player for a Windmill recording — no login, and
	 * embeddable in an iframe. Windmill stores nothing: the recording is either a
	 * file the visitor opens locally or a URL they host themselves (`?src=`), so a
	 * demo can live next to the docs or README that links it. Every kind of
	 * recording (app session, flow, script, pipeline run) plays here, offline.
	 */
	import RecordingPlayer from '$lib/components/recording/RecordingPlayer.svelte'
	import { setOfflineReplay } from '$lib/components/recording/offlineReplay.svelte'
	import {
		fetchRecording,
		MAX_RECORDING_BYTES,
		parseRecording
	} from '$lib/components/recording/rawAppRecordingLoad'
	import type { LoadedRecording } from '$lib/components/recording/rawAppRecordingLoad'
	import { Loader2, TriangleAlert, Upload } from 'lucide-svelte'
	import { onDestroy, onMount } from 'svelte'

	let loaded = $state<LoadedRecording | undefined>(undefined)
	let loading = $state(false)
	let error = $state<string | undefined>(undefined)
	let progress = $state<number | undefined>(undefined)

	function accept(data: unknown): boolean {
		const res = parseRecording(data)
		if (!res.ok) {
			error = res.error
			return false
		}
		error = undefined
		loaded = res.loaded
		return true
	}

	async function loadFromUrl(src: string) {
		loading = true
		error = undefined
		try {
			accept(
				await fetchRecording(src, (bytes, total) => {
					progress = total ? Math.round((bytes / total) * 100) : undefined
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
		// Same cap the `?src=` fetch enforces: reading a multi-hundred-MB file into a
		// string is enough to take the tab down before validation gets a say.
		if (file.size > MAX_RECORDING_BYTES) {
			error = `That file is too large to replay (${file.size} bytes).`
			return
		}
		try {
			accept(JSON.parse(await file.text()))
		} catch (err) {
			error = `Could not read the file: ${err instanceof Error ? err.message : err}`
		}
	}

	onMount(() => {
		setOfflineReplay(true)
		const src = new URL(window.location.href).searchParams.get('src')
		if (src) loadFromUrl(src)
	})

	onDestroy(() => setOfflineReplay(false))

	let title = $derived.by(() => {
		if (!loaded) return 'Replay'
		switch (loaded.kind) {
			case 'app':
				return `Replay — ${loaded.recording.app_path}`
			case 'script':
				return `Replay — ${loaded.recording.script_path}`
			case 'pipeline':
				return `Replay — ${loaded.recording.folder}`
			case 'flow':
				return `Replay — ${loaded.recording.flow_path}`
		}
	})
</script>

<svelte:head><title>{title}</title></svelte:head>

<div class="h-screen w-screen p-3 bg-surface overflow-auto">
	{#if loaded}
		<RecordingPlayer {loaded} hideHeader onreset={() => (loaded = undefined)} />
	{:else if loading}
		<div class="h-full flex flex-col items-center justify-center gap-2 text-sm text-secondary">
			<Loader2 size={24} class="animate-spin text-blue-500" />
			Downloading the recording{progress !== undefined ? ` — ${progress}%` : ''}…
		</div>
	{:else}
		<div class="h-full flex flex-col items-center justify-center gap-3 text-center px-4">
			<h1 class="text-lg font-semibold text-emphasis">Replay a recording</h1>
			<p class="text-xs text-secondary max-w-lg">
				Open a recording file, or point this page at one you host yourself with
				<span class="font-mono">?src=&lt;url&gt;</span>. Nothing is uploaded to Windmill — the
				recording is read in your browser and replayed from it alone.
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
