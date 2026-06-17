<script lang="ts">
	import { untrack } from 'svelte'
	import AiChatLayout from './copilot/chat/AiChatLayout.svelte'
	import type { FlowBuilderProps } from './flow_builder'
	import FlowBuilder from './FlowBuilder.svelte'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { workspaceStore } from '$lib/stores'
	import type { OpenFlow } from '$lib/gen'

	let {
		flowStore: oldFlowStore,
		flowStateStore: oldFlowStateStore,
		disableAi,
		light,
		...props
	}: FlowBuilderProps & { light?: boolean } = $props()

	let flowStateStore = $state(untrack(() => oldFlowStateStore))

	let trialRender = $state(true)

	if (untrack(() => light)) {
		setTimeout(() => {
			trialRender = false
		}, 1000 * 300)
	}

	// Stable per-user draft storage key. Captured once so editing the flow's path
	// (which lives in `draft_path`, not the storage key) can't re-key the autosave
	// handle and orphan the draft. Mirrors the full-page editor keying on the URL
	// path; falls back through the SDK's path inputs.
	const draftStoragePath = untrack(
		() =>
			props.initialPath ||
			props.pathStoreInit ||
			(oldFlowStore.val as { path?: string } | undefined)?.path ||
			''
	)

	// Autosave the SDK editor the same way the full-page editor does: route the
	// bound store through a per-user UserDraft handle so edits persist as a draft
	// and the AutosaveIndicator (which FlowBuilder gates on
	// `liveEditorDraftStoragePath`) appears. Guarded on workspace + path: the SDK
	// can mount before a workspace exists, and `useMany` would otherwise throw
	// resolving the (absent) global workspace. `canBeDisabled` for the auto-save
	// toggle, matching the full-page editor.
	const draftHandles = UserDraft.useMany<OpenFlow>(() => {
		const ws = $workspaceStore
		return ws && draftStoragePath
			? [{ itemKind: 'flow', path: draftStoragePath, workspace: ws, canBeDisabled: true }]
			: []
	})
	const draftHandle = $derived(draftHandles[0])

	// Bound store the builder reads/writes. Backed by the draft handle when one is
	// live; falls back to the consumer-provided store before a workspace exists so
	// the editor still works without autosave.
	const flowStore = {
		get val() {
			return draftHandle?.draft ?? oldFlowStore.val
		},
		set val(v: OpenFlow) {
			if (draftHandle) {
				draftHandle.draft = v
			} else {
				oldFlowStore.val = v
			}
		}
	}

	// Seed the handle once per acquired path from the consumer's loaded flow. The
	// first cell write after acquire is swallowed by the syncer's seed guard, so
	// this primes the baseline without POSTing — the user's first edit is the
	// first real save. Skipped when a draft already lives in the cell (e.g. a
	// remount onto an already-edited path) so it doesn't clobber pending edits.
	let seededPath: string | undefined = undefined
	$effect(() => {
		const handle = draftHandle
		const p = draftStoragePath
		if (!handle || !p) return
		untrack(() => {
			if (seededPath === p) return
			if (handle.draft === undefined) {
				handle.draft = oldFlowStore.val
			}
			seededPath = p
		})
	})
</script>

{#if trialRender}
	<AiChatLayout noPadding={true} {disableAi}>
		{#if light}<div class="bg-red-500 absolute z-10">Trial version</div>{/if}
		<FlowBuilder
			{flowStore}
			{flowStateStore}
			{disableAi}
			{...props}
			liveEditorDraftStoragePath={draftStoragePath || undefined}
		/>
	</AiChatLayout>
{:else}
	<div class="flex flex-col items-center justify-center h-screen">
		<div class="text-2xl font-bold"
			>Windmill Whitelabel SDK is in trial mode and disabled itself after 5 minutes</div
		>
	</div>
{/if}
