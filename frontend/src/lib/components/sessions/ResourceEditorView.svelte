<script lang="ts">
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import { untrack } from 'svelte'

	let {
		path,
		workspaceId,
		onBack,
		onRemoved,
		onRenamed
	}: {
		/** The resource this tab edits (the row its location deep-links). */
		path: string
		/** The session's acting workspace, which the editor operates on instead of
		 * `$workspaceStore` (the nav workspace, which a session leaves put). */
		workspaceId: string
		/** Back to the list this editor was reached through; the tab replaced it. */
		onBack?: () => void
		/** The item at `fromPath` is gone — a draft-only one whose draft was discarded.
		 * Distinct from `onBack`, which moves whichever tab is active: only this tab is
		 * the one to send back, and only while it still shows that item. */
		onRemoved?: (fromPath: string, fromWorkspace: string) => void
		/** The item at `fromPath` was saved under a different path. The tab addresses
		 * it by path — as do its label, the chat's ACTIVE PREVIEW and the draft key —
		 * so the tab has to follow, or all four keep naming an item that no longer
		 * exists. */
		onRenamed?: (newPath: string, fromPath: string, fromWorkspace: string) => void
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

<ResourceEditorDrawer
	bind:this={editor}
	useDrawer={false}
	workspace={workspaceId}
	{onBack}
	{onRemoved}
	onSaved={(saved, from, fromWs) => {
		if (saved && from && fromWs && saved !== from) onRenamed?.(saved, from, fromWs)
	}}
/>
