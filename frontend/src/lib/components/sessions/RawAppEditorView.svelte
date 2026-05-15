<script lang="ts">
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { untrack } from 'svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import {
		globalDraftStore,
		type AppDraftValue
	} from '$lib/components/copilot/chat/global/draftStore.svelte'
	import { applyDraftValueToRawApp, rawAppToDraftValue } from './appDraftCodec'

	let {
		runtime,
		path,
		workspaceId,
		onNavigate
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
	} = $props()

	let diffDrawer: DiffDrawer | undefined = $state()

	$effect(() => {
		if (workspaceId && path) {
			untrack(() => runtime.loadRawApp(workspaceId, path))
		}
	})

	async function restoreFromCurrentTarget() {
		diffDrawer?.closeDrawer()
		await runtime.loadRawApp(workspaceId, path)
	}

	// Bidirectional sync with the global AI chat's globalDraftStore.
	// Same one-way-reactive discipline as ScriptEditorView / FlowEditorView:
	// inbound tracks only the store, outbound tracks only rawApp.val; each
	// side's read into the other goes through untrack() to break the
	// keystroke-revert race.
	let lastInboundSig: string | undefined = $state(undefined)

	// Store → editor. Re-runs on globalDraftStore changes (AI write).
	$effect(() => {
		if (!workspaceId || !path) return
		const draft = globalDraftStore.getAppDraft(workspaceId, path)
		if (
			!draft ||
			!draft.value ||
			typeof draft.value !== 'object' ||
			!('files' in (draft.value as object))
		)
			return
		const incoming = draft.value as AppDraftValue
		const sig = JSON.stringify(incoming)
		untrack(() => {
			if (runtime.loadedRawAppPath !== path) return
			if (sig === lastInboundSig) return
			const current = runtime.rawApp.val
			if (!current) return
			lastInboundSig = sig
			runtime.rawApp.val = applyDraftValueToRawApp(current, incoming)
		})
	})

	// Editor → store. Debounced 150ms so a typing burst inside a frontend
	// file's Monaco editor coalesces into one store write.
	let outboundTimer: ReturnType<typeof setTimeout> | undefined
	$effect(() => {
		if (!workspaceId || !path) return
		if (runtime.loadedRawAppPath !== path) return
		const raw = runtime.rawApp.val
		if (!raw) return
		const sig = JSON.stringify(rawAppToDraftValue(raw))
		if (sig === lastInboundSig) return
		if (outboundTimer) clearTimeout(outboundTimer)
		outboundTimer = setTimeout(() => {
			untrack(() => {
				const current = globalDraftStore.getAppDraft(workspaceId, path)
				if (current?.value && JSON.stringify(current.value) === sig) return
				globalDraftStore.setDraft(workspaceId, {
					type: 'app',
					path,
					summary: raw.summary,
					value: rawAppToDraftValue(raw),
					isDraft: true
				})
			})
		}, 150)
		return () => {
			if (outboundTimer) clearTimeout(outboundTimer)
		}
	})
</script>

<DiffDrawer
	bind:this={diffDrawer}
	restoreDeployed={restoreFromCurrentTarget}
	restoreDraft={restoreFromCurrentTarget}
/>
{#if runtime.loadingRawApp && !runtime.loadedRawAppPath}
	<div class="p-4 text-secondary text-sm">Loading raw app {path}…</div>
{:else if runtime.notFoundRawApp && !runtime.loadedRawAppPath}
	<div class="p-4 text-secondary text-sm">Raw app not found at path {path}</div>
{:else if runtime.rawApp.val}
	<RawAppEditor
		bind:files={runtime.rawApp.val.files}
		bind:runnables={runtime.rawApp.val.runnables}
		bind:data={runtime.rawApp.val.data}
		bind:summary={runtime.rawApp.val.summary}
		newPath={runtime.rawApp.val.path}
		{path}
		policy={runtime.rawApp.val.policy}
		bind:savedApp={runtime.savedRawApp.val}
		newApp={false}
		{diffDrawer}
		{onNavigate}
	/>
{/if}
