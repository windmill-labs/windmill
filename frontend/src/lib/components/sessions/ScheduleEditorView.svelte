<script lang="ts">
	import ScheduleEditorInner from '$lib/components/triggers/schedules/ScheduleEditorInner.svelte'
	import { setTriggerWorkspace } from '$lib/components/triggers/triggerWorkspace'
	import { Button } from '$lib/components/common'
	import { ArrowLeft } from 'lucide-svelte'
	import { untrack } from 'svelte'

	let {
		path,
		workspaceId,
		onBack,
		onRemoved,
		onRenamed
	}: {
		/** The schedule this tab edits (the row its location deep-links). */
		path: string
		/** The session's acting workspace, which the whole trigger subtree reads
		 * through the `triggerWorkspace` resolver instead of `$workspaceStore`. */
		workspaceId: string
		/** Back to the list this editor was reached through; the tab replaced it. */
		onBack?: () => void
		/** The item at `fromPath` is gone — a draft-only one whose draft was discarded.
		 * Distinct from `onBack`, which moves whichever tab is active: the discard
		 * awaits a reload of the runnable, by which time the user may be looking at
		 * another tab, or have pointed this one somewhere else. */
		onRemoved?: (fromPath: string, fromWorkspace: string) => void
		/** The item at `fromPath` was saved under a different path. The tab addresses
		 * it by path — as do its label, the chat's ACTIVE PREVIEW and the draft key —
		 * so the tab has to follow, or all four keep naming an item that no longer
		 * exists. */
		onRenamed?: (newPath: string, fromPath: string, fromWorkspace: string) => void
	} = $props()

	// Captured at init, so it must read the current prop rather than close over it.
	setTriggerWorkspace(() => workspaceId)

	let editor = $state<ScheduleEditorInner | undefined>()

	// A save leaves the mounted editor holding the pre-save config as its deployed
	// baseline — the drawer hides that by closing, which inline is a no-op, so the
	// banner would go on claiming unsaved changes and Discard would restore the
	// value the save just replaced. Remounting re-reads the saved schedule.
	let generation = $state(0)

	// Loads the schedule this tab holds, on the instance the `{#key}` below just
	// mounted for it — `editor` is rebound per instance, so this re-runs per path
	// and per generation. `isFlow` is a first guess only — loadScheduleCfg sets it
	// from the loaded config — so the tab needs no knowledge beyond the path.
	$effect(() => {
		const e = editor
		if (!path || !e) return
		untrack(() => void e.openEdit(path, false))
	})
</script>

<div class="h-full w-full overflow-auto p-4">
	<!-- useDrawer=false renders the editor as a Section, the same inline form the
	     script/flow trigger panel mounts (see SchedulePanel). `customLabel` is that
	     Section's header, which is where the way back belongs. -->
	<!-- Keyed on the path as well: `openEdit` loads asynchronously, and re-pointing a
	     live editor would leave the previous schedule's load to finish into the new
	     one's form and deployed baseline. -->
	{#key `${path}#${generation}`}
		<!-- No `allowDraft`: it switches the toolbar to the trigger-panel branch, whose
		     deploy button is gated on the `trigger`/`isDeployed` a script or flow editor
		     supplies alongside a staged trigger. A standalone tab has neither, and would
		     be left permanently unsavable; this keeps the drawer's plain Save. -->
		<ScheduleEditorInner
			bind:this={editor}
			useDrawer={false}
			showDraftBanner
			{onRemoved}
			onUpdate={(saved: string | undefined, from: string, fromWs: string) => {
				// The write landed after the tab was pointed at another schedule: the
				// remount below would take that one back through a load it never asked
				// for, and the rename is not its rename.
				if (from !== path) return
				generation++
				// A draft-only schedule opens with its path editable (saving CREATEs),
				// so the save can land somewhere other than where the tab is pointed.
				if (saved && saved !== from) onRenamed?.(saved, from, fromWs)
			}}
		>
			{#snippet customLabel()}
				<div class="flex flex-row items-center gap-2 min-w-0">
					{#if onBack}
						<Button
							variant="subtle"
							unifiedSize="sm"
							startIcon={{ icon: ArrowLeft }}
							on:click={onBack}
							title="Back to Schedules"
							iconOnly
						/>
					{/if}
					<span class="text-sm font-semibold truncate">Schedule</span>
				</div>
			{/snippet}
		</ScheduleEditorInner>
	{/key}
</div>
