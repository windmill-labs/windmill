<script lang="ts">
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import { untrack } from 'svelte'

	let {
		path,
		workspaceId,
		onBack,
		onRemoved,
		onSavedTo
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
		/** The item at `fromPath` was saved, to `newPath`. Every tab on it has to hear:
		 * a rename moves the ones addressing it by path — their label, the chat's
		 * ACTIVE PREVIEW and the draft key all do — and a plain save moves none but
		 * leaves the others holding a baseline the deploy has replaced. */
		onSavedTo?: (newPath: string, fromPath: string, fromWorkspace: string) => void
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
	onRestored={(ws, restored) => onSavedTo?.(restored, restored, ws)}
	onSaved={(saved, from, fromWs) => {
		if (saved && from && fromWs) onSavedTo?.(saved, from, fromWs)
	}}
/>
