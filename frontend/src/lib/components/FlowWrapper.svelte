<script lang="ts">
	import { untrack } from 'svelte'
	import AiChatLayout from './copilot/chat/AiChatLayout.svelte'
	import type { FlowBuilderProps } from './flow_builder'
	import FlowBuilder from './FlowBuilder.svelte'
	import { usePageDraftSync } from './usePageDraftSync.svelte'
	import { workspaceStore } from '$lib/stores'
	import { selectDraftStoragePath } from '$lib/mintDraftPath'
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
	// path; falls back through the SDK's path inputs. For a brand-new flow with no
	// caller path this mints a `u/<user>/draft_<uuid>` key — the SDK equivalent of
	// the `/flows/add` redirect — so autosave attaches instead of the handle
	// detaching (local-only, never POSTs).
	const draftStoragePath = untrack(() =>
		selectDraftStoragePath({
			providedPaths: [
				props.initialPath,
				props.pathStoreInit,
				(oldFlowStore.val as { path?: string } | undefined)?.path
			],
			isNewItem: !!props.newFlow
		})
	)

	// Reuse the full-page flow editor's draft orchestration so the SDK gets
	// autosave + the AutosaveIndicator (gated by FlowBuilder on
	// `liveEditorDraftStoragePath`) — and `recordRemoteSync`/`seedBaseline`/
	// `discardIf` if it ever loads a server draft — from one code path. The SDK
	// may mount before login, so `useReactive` hands out a detached local-only
	// handle until `$workspaceStore` resolves (no throw). The builder itself is
	// gated on the workspace below so no edits are made into that detached handle
	// — they'd be lost when it re-keys to the real entry on login.
	// `defaultValue` seeds the handle from the consumer's loaded flow on first
	// acquire (swallowed by the seed guard, never POSTs) — captured once so it
	// doesn't churn the reconcile.
	const initialFlow = untrack(() => oldFlowStore.val)
	const draftSync = usePageDraftSync<OpenFlow>({
		itemKind: 'flow',
		path: () => draftStoragePath,
		workspace: () => $workspaceStore,
		defaultValue: initialFlow
	})

	// Bound store the builder reads/writes, backed by the draft handle. Falls back
	// to the consumer-provided value in the first-render window before the handle
	// is acquired.
	const flowStore = {
		get val() {
			return draftSync.draft ?? oldFlowStore.val
		},
		set val(v: OpenFlow) {
			draftSync.draft = v
		}
	}
</script>

{#if trialRender}
	<AiChatLayout noPadding={true} {disableAi}>
		{#if light}<div class="bg-red-500 absolute z-10">Trial version</div>{/if}
		<!-- Gate on a resolved workspace: the draft handle is detached (local-only)
		     until one exists, so editing before then would be lost on the re-key. -->
		{#if $workspaceStore}
			<FlowBuilder
				{flowStore}
				{flowStateStore}
				{disableAi}
				{...props}
				liveEditorDraftStoragePath={draftStoragePath || undefined}
			/>
		{/if}
	</AiChatLayout>
{:else}
	<div class="flex flex-col items-center justify-center h-screen">
		<div class="text-2xl font-bold"
			>Windmill Whitelabel SDK is in trial mode and disabled itself after 5 minutes</div
		>
	</div>
{/if}
