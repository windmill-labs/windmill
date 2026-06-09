<script lang="ts">
	import { Button } from '$lib/components/common'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { classes } from '$lib/components/common/alert/model'
	import { type Value } from '$lib/utils'
	import { AlertCircle, Diff } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { slide } from 'svelte/transition'

	interface Props {
		/** Whether there are unsaved local changes relative to the deployed baseline. */
		show: boolean
		/** Deployed/backend baseline config (the "original" side of the diff). */
		getDeployed: () => unknown
		/** Current form config — the local autosave (the "current" side of the diff). */
		getCurrent: () => unknown
		/** Drop the local changes and reset the form to the deployed baseline. */
		onDiscard: () => void | Promise<void>
		/** When true (e.g. no write access), hide the Discard action. */
		disabled?: boolean
		/** Diff drawer title. */
		title?: string
	}

	let {
		show,
		getDeployed,
		getCurrent,
		onDiscard,
		disabled = false,
		title = 'Deployed <> Local changes'
	}: Props = $props()

	// Suppress the banner when there's no deployed baseline to diff
	// against — a brand-new entity (variable/resource/trigger with no
	// deployed row yet) has deployed == null, so "Show diff" would do
	// nothing (the drawer early-returns) and "Discard changes" is
	// semantically backwards (there's nothing to revert to). The
	// callers' own `show` is computed off `current != deployed` which
	// is trivially true in that case, so the gate has to live here.
	let visible = $derived(show && getDeployed() != null)

	let diffDrawer: DiffDrawer | undefined = $state()

	function showDiff() {
		const deployed = getDeployed()
		if (deployed == null) return
		// Snapshot both sides at click time. They are typically Svelte `$state`
		// proxies (resource/variable's `initialStates[ws]` and the draft handle's
		// cell); without snapshot the diff drawer would re-read them reactively
		// and update as the user keeps typing behind it.
		const original = $state.snapshot(deployed) as Value
		const current = $state.snapshot(getCurrent()) as Value
		diffDrawer?.openDrawer()
		// Mirror the inline Discard's `disabled` gate inside the diff drawer —
		// otherwise a read-only user could still trigger onDiscard via the
		// drawer button even though we hid the banner's inline action.
		diffDrawer?.setDiff({
			mode: 'simple',
			original,
			current,
			title,
			button: disabled
				? undefined
				: {
						text: 'Discard changes',
						onClick: async () => {
							await onDiscard()
							diffDrawer?.closeDrawer()
						}
					}
		})
	}
</script>

<DiffDrawer bind:this={diffDrawer} />

{#if visible}
	<div
		transition:slide|local={{ duration: 120 }}
		class={twMerge(
			'flex flex-row items-center justify-between gap-2 px-4 py-1',
			classes.warning.bgClass,
			'!border-0 !rounded-none'
		)}
	>
		<div class="flex flex-row items-center gap-2 min-w-0">
			<AlertCircle class={classes.warning.iconClass} size={16} />
			<span class={twMerge('text-xs font-semibold truncate', classes.warning.titleClass)}>
				You have unsaved changes
			</span>
		</div>
		<div class="flex flex-row items-center gap-2 shrink-0">
			<Button
				unifiedSize="sm"
				variant="default"
				startIcon={{ icon: Diff }}
				btnClasses={classes.warning.titleClass}
				on:click={showDiff}>Show diff</Button
			>
			{#if !disabled}
				<Button
					unifiedSize="sm"
					variant="subtle"
					btnClasses={classes.warning.titleClass}
					on:click={onDiscard}>Discard</Button
				>
			{/if}
		</div>
	</div>
{/if}
