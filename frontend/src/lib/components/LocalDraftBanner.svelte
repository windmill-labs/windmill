<script lang="ts">
	import { Button } from '$lib/components/common'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { classes } from '$lib/components/common/alert/model'
	import {
		cleanValueProperties,
		orderedYamlStringify,
		replaceFalseWithUndefined,
		type Value
	} from '$lib/utils'
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

	/** Same cleaning + YAML serialization the DiffDrawer applies before
	 * comparing. Without it the banner would fire on differences the
	 * drawer treats as no-op (toggle defaults, `false ↔ undefined`,
	 * key ordering noise) — exactly the case where the user clicks
	 * "Show diff" and sees the "No changes detected" empty state. */
	function diffKey(value: unknown): string {
		try {
			return orderedYamlStringify(cleanValueProperties(replaceFalseWithUndefined(value as Value)))
		} catch {
			return ''
		}
	}

	// Suppress the banner when:
	//   • There's no deployed baseline (brand-new entity — "Show diff"
	//     would early-return and "Discard" is semantically backwards),
	//     OR
	//   • Deployed and current are equal under the DiffDrawer's own
	//     comparison. The callers' `show` is a coarser "form differs
	//     from baseline" check that can stale-fire after a save lands
	//     or when `false`/`undefined` toggle noise flips a field.
	let visible = $derived.by(() => {
		if (!show) return false
		const deployed = getDeployed()
		if (deployed == null) return false
		return diffKey(deployed) !== diffKey(getCurrent())
	})

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
