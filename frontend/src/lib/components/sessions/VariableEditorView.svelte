<script lang="ts">
	import VariableEditor from '$lib/components/VariableEditor.svelte'
	import { untrack } from 'svelte'

	let {
		path,
		workspaceId,
		onBack,
		onRenamed
	}: {
		/** The variable this tab edits (the row its location deep-links). */
		path: string
		/** The session's acting workspace, which the editor operates on instead of
		 * `$workspaceStore` (the nav workspace, which a session leaves put). */
		workspaceId: string
		/** Back to the list this editor was reached through; the tab replaced it. */
		onBack?: () => void
		/** The item was saved under a different path. The tab addresses it by path —
		 * as do its label, the chat's ACTIVE PREVIEW and the draft key — so the tab
		 * has to follow, or all four keep naming an item that no longer exists. */
		onRenamed?: (newPath: string) => void
	} = $props()

	let editor = $state<VariableEditor | undefined>()

	// Re-selects when the tab is pointed at another variable; the component keeps
	// its identity across that, as it does for the drawer's row-to-row switch.
	$effect(() => {
		const p = path
		const e = editor
		if (!p || !e) return
		untrack(() => e.editVariable(p))
	})
</script>

<VariableEditor
	bind:this={editor}
	useDrawer={false}
	workspace={workspaceId}
	{onBack}
	onSaved={(saved) => {
		if (saved && saved !== path) onRenamed?.(saved)
	}}
/>
