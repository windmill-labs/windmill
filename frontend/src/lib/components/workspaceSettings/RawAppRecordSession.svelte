<script lang="ts">
	/**
	 * Records a raw app's session for its Hub page: the app runs full-screen, the
	 * user drives it, and what they did becomes a step-by-step recording visitors
	 * can replay before forking the project. A raw app cannot demo itself with a
	 * job log the way a script or flow does — this is its equivalent.
	 */
	import { Button } from '$lib/components/common'
	import RawAppPreview from '$lib/components/raw_apps/RawAppPreview.svelte'
	import RawAppRecordingReplay from '$lib/components/recording/RawAppRecordingReplay.svelte'
	import { createRawAppRecording } from '$lib/components/recording/rawAppRecording.svelte'
	import type { RawAppRecording } from '$lib/components/recording/types'
	import type { Runnable } from '$lib/components/raw_apps/rawAppPolicy'
	import { AppService } from '$lib/gen'
	import { userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Check, Circle, Download, Loader2, Square } from 'lucide-svelte'
	import { onDestroy, setContext } from 'svelte'

	interface Props {
		workspace: string
		path: string
		/** Saves the finished recording against the Hub item. Absent while the
		 * project has no Hub draft yet, which leaves Download as the only action. */
		onsave?: (recording: RawAppRecording) => Promise<boolean>
	}

	let { workspace, path, onsave }: Props = $props()

	const recorder = createRawAppRecording()

	let app = $state<any>(undefined)
	// The publisher's isolation opt-in decides this, exactly as in the viewer:
	// forcing the unsandboxed path here would run a bundle its own author marked
	// untrusted same-origin with the session of whoever is recording it. An
	// isolated app simply cannot be recorded — its DOM is unreachable by design.
	let sandboxed = $derived(app?.policy?.sandbox === true)
	setContext('IS_APP_UNSANDBOXED', {
		get value() {
			return app !== undefined && !sandboxed
		}
	})
	let loadError = $state<string | undefined>(undefined)
	let iframe = $state<HTMLIFrameElement | undefined>(undefined)
	let recording = $state<RawAppRecording | undefined>(undefined)
	let saving = $state(false)

	async function load() {
		try {
			const loaded: any = await AppService.getAppByPath({ workspace, path })
			if (!loaded?.bundle_secret) {
				loaded.bundle_secret = await AppService.getPublicSecretOfLatestVersionOfApp({
					workspace,
					path
				})
			}
			app = loaded
		} catch (e: any) {
			loadError = e?.body ?? e?.message ?? String(e)
		}
	}
	load()

	// Showing the replay unmounts the preview, so "Record again" has to wait for the
	// fresh one: `iframe` still points at the removed document until it mounts and
	// hands its own back.
	let startWhenReady = $state(false)

	function start() {
		if (recording) {
			recording = undefined
			startWhenReady = true
			return
		}
		beginRecording()
	}

	function beginRecording() {
		if (!iframe) return
		if (!recorder.start(iframe, { appPath: path, workspace })) {
			sendUserToast('Cannot record this app: its bundle runs sandbox-isolated', true)
			return
		}
		sendUserToast(
			'Recording — walk through the app as a visitor would. Passwords are masked; mark ' +
				'sensitive elements with data-wm-no-record to leave them out.'
		)
	}

	function onIframe(el: HTMLIFrameElement | undefined) {
		iframe = el
		if (el && startWhenReady) {
			startWhenReady = false
			beginRecording()
		}
	}

	async function stop() {
		recording = await recorder.stop()
	}

	async function save() {
		if (!recording || !onsave) return
		saving = true
		try {
			if (await onsave(recording)) recording = undefined
		} finally {
			saving = false
		}
	}

	onDestroy(() => {
		// Not awaited: the drawer is going away, and the recorder resolves on its own
		// once the document it was reading is gone.
		if (recorder.active) recorder.stop()
	})
</script>

<div class="flex flex-col h-full min-h-0 gap-2">
	<div class="flex items-center gap-2 shrink-0 flex-wrap">
		<span class="text-sm font-semibold text-primary">{path}</span>
		{#if sandboxed}
			<span class="text-xs text-secondary">Sandbox-isolated: not recordable</span>
		{:else if recorder.active || recorder.stopping}
			<span class="flex items-center gap-1 text-xs text-primary">
				<Circle size={10} class="text-red-500 animate-pulse" fill="currentColor" />
				{recorder.stepCount} step{recorder.stepCount === 1 ? '' : 's'}
			</span>
			<Button
				size="xs"
				variant="border"
				loading={recorder.stopping}
				startIcon={{ icon: Square }}
				onclick={stop}
			>
				{recorder.stopping ? 'Waiting for the job…' : 'Stop recording'}
			</Button>
		{:else}
			<!-- While the replay is showing there is no preview and so no iframe; the
			     re-record path remounts one and starts against it. -->
			<Button
				size="xs"
				variant="accent"
				disabled={!iframe && !recording}
				startIcon={{ icon: Circle }}
				onclick={start}
			>
				{recording ? 'Record again' : 'Start recording'}
			</Button>
		{/if}
		{#if recording}
			<span class="text-xs text-secondary">
				{recording.steps.length} step{recording.steps.length === 1 ? '' : 's'} captured
			</span>
			<div class="ml-auto flex items-center gap-2">
				<Button
					size="xs"
					variant="border"
					startIcon={{ icon: Download }}
					onclick={() => recorder.download(recording!)}
				>
					Download
				</Button>
				{#if onsave}
					<Button
						size="xs"
						variant="accent"
						loading={saving}
						startIcon={{ icon: Check }}
						onclick={save}
					>
						Save as recording
					</Button>
				{/if}
			</div>
		{/if}
	</div>

	{#if loadError}
		<div class="text-sm text-red-600 dark:text-red-400">Could not load the app: {loadError}</div>
	{:else if sandboxed}
		<div class="text-sm text-secondary max-w-2xl">
			This app is opted into sandbox isolation, so it runs in an opaque-origin frame that nothing on
			this page can read — including the recorder. Turn isolation off in the app's settings to
			record a demo of it.
		</div>
	{:else if !app}
		<div class="flex items-center gap-2 text-sm text-secondary">
			<Loader2 size={14} class="animate-spin" /> Loading the app…
		</div>
	{:else if recording}
		<!-- What was captured, in the same player the Hub page will use. -->
		<div class="flex-1 min-h-0">
			<RawAppRecordingReplay {recording} />
		</div>
	{:else}
		<div class="flex-1 min-h-0 border rounded-md overflow-hidden">
			<RawAppPreview
				{workspace}
				user={$userStore}
				secret={app.bundle_secret}
				{path}
				runnables={(app.value?.runnables ?? {}) as Record<string, Runnable>}
				oniframe={onIframe}
			/>
		</div>
	{/if}
</div>
