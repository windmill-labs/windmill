<script lang="ts">
	import { untrack } from 'svelte'
	import AppEditor from './apps/editor/AppEditor.svelte'
	import type { AppEditorProps } from './apps/types'
	import { workspaceStore } from '$lib/stores'

	let { app: oldApp, ...props }: AppEditorProps = $props()

	let app = $state(untrack(() => oldApp))
</script>

<!-- Gate on a resolved workspace: AppEditor acquires its UserDraft handle at init
     from the (non-reactive) workspace, so mounting it before one exists would
     leave autosave permanently detached. Embedders set the workspace before
     rendering; the test_dev header sets it on mount. -->
{#if $workspaceStore}
	<AppEditor {app} {...props} />
{/if}
