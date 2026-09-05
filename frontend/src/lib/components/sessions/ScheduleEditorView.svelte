<script lang="ts">
	import ScheduleEditorInner from '$lib/components/triggers/schedules/ScheduleEditorInner.svelte'
	import { setTriggerWorkspace } from '$lib/components/triggers/triggerWorkspace'
	import { untrack } from 'svelte'

	let {
		path,
		workspaceId
	}: {
		/** The schedule this tab edits (the row its location deep-links). */
		path: string
		/** The session's acting workspace, which the whole trigger subtree reads
		 * through the `triggerWorkspace` resolver instead of `$workspaceStore`. */
		workspaceId: string
	} = $props()

	// Captured at init, so it must read the current prop rather than close over it.
	setTriggerWorkspace(() => workspaceId)

	let editor = $state<ScheduleEditorInner | undefined>()

	// Load whenever the tab is pointed at another schedule; the component keeps
	// its identity across that, as it does for the drawer's row-to-row switch.
	// `isFlow` is a first guess only — loadScheduleCfg sets it from the loaded
	// config — so the tab needs no knowledge of the target beyond the path.
	$effect(() => {
		const p = path
		const e = editor
		if (!p || !e) return
		untrack(() => void e.openEdit(p, false))
	})
</script>

<div class="h-full w-full overflow-auto p-4">
	<!-- useDrawer=false renders the editor as a Section, the same inline form the
	     script/flow trigger panel mounts (see SchedulePanel). -->
	<ScheduleEditorInner bind:this={editor} useDrawer={false} allowDraft showDraftBanner />
</div>
