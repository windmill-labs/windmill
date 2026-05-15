<script lang="ts">
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { untrack } from 'svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import {
		globalDraftStore,
		type FlowDraftValue
	} from '$lib/components/copilot/chat/global/draftStore.svelte'
	import { initFlowState } from '$lib/components/flows/flowState'
	import { applyDraftValueToFlow, flowToDraftValue } from './flowDraftCodec'

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

	// Bidirectional sync between this preview and the global AI chat's
	// in-memory draft store. Same one-way-reactive discipline as the
	// script case in ScriptEditorView.svelte — inbound tracks only the
	// store, outbound tracks only `flowStore.val`; the "other side" read
	// inside each effect goes through `untrack()`. Without that
	// asymmetry, a user keystroke would re-fire the inbound effect with
	// the pre-keystroke store value and revert the edit.
	let lastInboundSig: string | undefined = $state(undefined)

	// Store → editor. Re-runs when globalDraftStore changes (AI write
	// from this or another session). The flowStore read is untracked so
	// the editor's own mutations don't refire this effect.
	$effect(() => {
		if (!workspaceId || !path) return
		const draft = globalDraftStore.getFlowDraft(workspaceId, path)
		if (!draft || !draft.value || typeof draft.value !== 'object' || !('value' in draft.value))
			return
		const incoming = draft.value as FlowDraftValue
		const sig = JSON.stringify(incoming)
		untrack(() => {
			if (runtime.loadedPath !== path) return
			if (sig === lastInboundSig) return
			const current = runtime.flowStore.val
			if (!current) return
			lastInboundSig = sig
			runtime.flowStore.val = applyDraftValueToFlow(current, incoming)
			// flowStateStore is keyed by module_id; after an AI write the set
			// of module ids may differ, so rebuild the UI state. This wipes
			// per-module test args / preview output for the new flow — a
			// known v1 trade-off, see the plan's caveats.
			void initFlowState(runtime.flowStore.val, runtime.flowStateStore)
		})
	})

	// Editor → store. Re-runs on any deep mutation of flowStore.val
	// (modules, schema, module bodies). The store read is untracked.
	// Debounced 150ms so a typing burst inside an inline rawscript
	// editor results in one serialise-and-write instead of one per
	// keystroke.
	let outboundTimer: ReturnType<typeof setTimeout> | undefined
	$effect(() => {
		if (!workspaceId || !path) return
		if (runtime.loadedPath !== path) return
		const flow = runtime.flowStore.val
		if (!flow) return
		const sig = JSON.stringify(flowToDraftValue(flow))
		if (sig === lastInboundSig) return
		if (outboundTimer) clearTimeout(outboundTimer)
		outboundTimer = setTimeout(() => {
			untrack(() => {
				const current = globalDraftStore.getFlowDraft(workspaceId, path)
				if (current?.value && JSON.stringify(current.value) === sig) return
				globalDraftStore.setDraft(workspaceId, {
					type: 'flow',
					path,
					summary: flow.summary,
					value: flowToDraftValue(flow),
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
	isFlow
/>
{#if runtime.loadingFlow && !runtime.loadedPath}
	<div class="p-4 text-secondary text-sm">Loading flow {path}…</div>
{:else if runtime.notFound && !runtime.loadedPath}
	<div class="p-4 text-secondary text-sm">Flow not found at path {path}</div>
{:else}
	<FlowBuilder
		flowStore={runtime.flowStore}
		flowStateStore={runtime.flowStateStore}
		initialPath={path}
		newFlow={false}
		{selectedId}
		loading={runtime.loadingFlow && !runtime.loadedPath}
		bind:savedFlow={runtime.savedFlow.val}
		{diffDrawer}
		{onNavigate}
	/>
{/if}
