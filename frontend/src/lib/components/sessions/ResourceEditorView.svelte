<script lang="ts">
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import { untrack } from 'svelte'

	let {
		path,
		workspaceId,
		onBack
	}: {
		/** The resource this tab edits (the row its location deep-links). */
		path: string
		/** The session's acting workspace, which the editor operates on instead of
		 * `$workspaceStore` (the nav workspace, which a session leaves put). */
		workspaceId: string
		/** Back to the list this editor was reached through; the tab replaced it. */
		onBack?: () => void
	} = $props()

	let editor = $state<ResourceEditorDrawer | undefined>()

	// Re-selects when the tab is pointed at another resource; the component keeps
	// its identity across that, as it does for the drawer's row-to-row switch.
	$effect(() => {
		const p = path
		const e = editor
		if (!p || !e) return
		untrack(() => void e.initEdit(p))
	})
</script>

<ResourceEditorDrawer bind:this={editor} useDrawer={false} workspace={workspaceId} {onBack} />
