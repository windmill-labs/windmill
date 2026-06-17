<script lang="ts">
	import { untrack } from 'svelte'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import AiChatLayout from './copilot/chat/AiChatLayout.svelte'
	import type { ScriptBuilderProps } from './script_builder'
	import { usePageDraftSync } from './usePageDraftSync.svelte'
	import { workspaceStore } from '$lib/stores'

	let { script: oldScript, disableAi, ...props }: ScriptBuilderProps = $props()

	// Stable per-user draft storage key. Mirrors the full-page editor keying on
	// the URL path; falls back through the SDK's path inputs.
	const draftStoragePath = untrack(() => props.initialPath || oldScript?.path || '')

	// Reuse the full-page script editor's draft orchestration (same as the flow
	// SDK) so the SDK gets autosave + the AutosaveIndicator (gated by ScriptBuilder
	// on `userDraftPath`) from one code path. `defaultValue` seeds the handle from
	// the consumer's script on first acquire (swallowed by the syncer's seed guard,
	// never POSTs). `useReactive` tolerates mounting before login (detached
	// local-only handle, no throw); the builder is gated on the workspace below so
	// edits aren't made into that detached handle and lost when it re-keys.
	const initialScript = untrack(() => oldScript)
	const draftSync = usePageDraftSync<ScriptBuilderProps['script']>({
		itemKind: 'script',
		path: () => draftStoragePath,
		workspace: () => $workspaceStore,
		defaultValue: initialScript
	})
</script>

<AiChatLayout noPadding {disableAi}>
	{#if $workspaceStore && draftSync.draft}
		<ScriptBuilder
			bind:script={draftSync.draft}
			userDraftPath={draftStoragePath}
			{disableAi}
			{...props}
		/>
	{/if}
</AiChatLayout>
