<script lang="ts">
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { untrack } from 'svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import type { Flow } from '$lib/gen'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { initFlowState } from '$lib/components/flows/flowState'
	import SessionItemNotFound from './SessionItemNotFound.svelte'

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

	let selectedId = $state('settings-metadata')
	let diffDrawer: DiffDrawer | undefined = $state()

	$effect(() => {
		if (workspaceId && path) {
			untrack(() => runtime.loadFlow(workspaceId, path))
		}
	})

	// In a session pane, "restore" just reloads from the current state — the
	// session target stays put. The Diff drawer's primary use here is viewing
	// the diff; restore is best-effort.
	async function restoreFromCurrentTarget() {
		diffDrawer?.closeDrawer()
		await runtime.loadFlow(workspaceId, path)
	}

	// Mark this editor as the "live editor" for the session's workspace so
	// the chat's `isLiveDraft` hint and `discard_local_draft` tool resolve to
	// this path. Same registration the regular /flows/edit page does on
	// mount, scoped to the session's (forked) workspace.
	$effect(() => {
		if (!workspaceId || !path) return
		UserDraft.setLiveEditorDraft({
			workspace: workspaceId,
			itemKind: 'flow',
			storagePath: path,
			effectivePath: runtime.flowStore.val?.path ?? path
		})
		return () =>
			UserDraft.clearLiveEditorDraft('flow', { workspace: workspaceId, storagePath: path })
	})

	// Bidirectional sync between this preview and `UserDraft<Flow>`. Same
	// one-way-reactive discipline as before: inbound tracks only the store,
	// outbound tracks only `flowStore.val`; the read on the "other side"
	// inside each effect goes through `untrack()`. Without that asymmetry, a
	// user keystroke would re-fire the inbound effect with the pre-keystroke
	// stored value and revert the edit.
	let lastInboundSig: string | undefined = $state(undefined)

	// Store → editor. Re-runs on UserDraft changes (AI write from this
	// session's chat or another session). flowStore reads are untracked so
	// the editor's own mutations don't refire this effect.
	$effect(() => {
		if (!workspaceId || !path) return
		const incoming = UserDraft.get<Flow>('flow', path, { workspace: workspaceId })
		if (!incoming) return
		const sig = JSON.stringify({ value: incoming.value, schema: incoming.schema })
		untrack(() => {
			if (runtime.loadedPath !== path) return
			if (sig === lastInboundSig) return
			const current = runtime.flowStore.val
			if (!current) return
			lastInboundSig = sig
			runtime.flowStore.val = {
				...current,
				value: incoming.value,
				schema: incoming.schema ?? current.schema,
				summary: incoming.summary ?? current.summary
			}
			// flowStateStore is keyed by module_id; after an AI write the set
			// of module ids may differ, so rebuild the UI state. This wipes
			// per-module test args / preview output for the new flow — a
			// known v1 trade-off, see the plan's caveats.
			void initFlowState(runtime.flowStore.val, runtime.flowStateStore)
		})
	})

	// Editor → store. Re-runs on any deep mutation of flowStore.val
	// (modules, schema, module bodies). The store read is untracked.
	// Debounced 150ms so a typing burst inside an inline rawscript editor
	// results in one serialise-and-write instead of one per keystroke.
	let outboundTimer: ReturnType<typeof setTimeout> | undefined
	$effect(() => {
		if (!workspaceId || !path) return
		if (runtime.loadedPath !== path) return
		const flow = runtime.flowStore.val
		if (!flow) return
		const sig = JSON.stringify({ value: flow.value, schema: flow.schema })
		if (sig === lastInboundSig) return
		if (outboundTimer) clearTimeout(outboundTimer)
		outboundTimer = setTimeout(() => {
			untrack(() => {
				const current = UserDraft.get<Flow>('flow', path, { workspace: workspaceId })
				if (current && JSON.stringify({ value: current.value, schema: current.schema }) === sig)
					return
				UserDraft.save('flow', path, flow, { workspace: workspaceId })
			})
		}, 150)
		return () => {
			if (outboundTimer) clearTimeout(outboundTimer)
		}
	})
</script>

{#if runtime.savedFlow.val}
	<DiffDrawer
		bind:this={diffDrawer}
		restoreDeployed={restoreFromCurrentTarget}
		restoreDraft={restoreFromCurrentTarget}
		isFlow
	/>
{/if}
{#if runtime.loadingFlow && !runtime.loadedPath}
	<div class="p-4 text-secondary text-sm">Loading flow {path}…</div>
{:else if runtime.notFound && !runtime.loadedPath}
	<SessionItemNotFound kind="flow" {path} {onNavigate} />
{:else}
	<FlowBuilder
		flowStore={runtime.flowStore}
		flowStateStore={runtime.flowStateStore}
		initialPath={path}
		newFlow={!runtime.savedFlow.val}
		{selectedId}
		loading={runtime.loadingFlow && !runtime.loadedPath}
		bind:savedFlow={runtime.savedFlow.val}
		{diffDrawer}
		{onNavigate}
	/>
{/if}
