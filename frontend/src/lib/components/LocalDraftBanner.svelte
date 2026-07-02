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
	import { fade } from 'svelte/transition'

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
		/**
		 * Whether to reserve the banner's fixed-height slot (so toggling the
		 * "unsaved changes" state doesn't shift the content below). Pass this when
		 * `getDeployed()` reads a non-reactive source (the trigger editors back
		 * their baseline with a plain `let`); otherwise it defaults to
		 * `getDeployed() != null`, which is reactive for `$state`-backed baselines.
		 */
		reserveSpace?: boolean
	}

	let {
		show,
		getDeployed,
		getCurrent,
		onDiscard,
		disabled = false,
		title = 'Deployed <> Local changes',
		reserveSpace
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

	// A deployed baseline means the banner *can* toggle on/off while the
	// user edits this entity, so we reserve its fixed-height slot up front
	// (see the template) to keep the banner appearing/disappearing from
	// shifting the content below. Brand-new entities have no baseline — the
	// banner can never show there, so we reserve nothing and add no gap.
	let hasBaseline = $derived(reserveSpace ?? getDeployed() != null)

	// Suppress the banner when:
	//   • There's no deployed baseline (brand-new entity — "Show diff"
	//     would early-return and "Discard" is semantically backwards),
	//     OR
	//   • Deployed and current are equal under the DiffDrawer's own
	//     comparison. The callers' `show` is a coarser "form differs
	//     from baseline" check that can stale-fire after a save lands
	//     or when `false`/`undefined` toggle noise flips a field.
	let visible = $derived.by(() => {
		if (!show || !hasBaseline) return false
		return diffKey(getDeployed()) !== diffKey(getCurrent())
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

{#if hasBaseline}
	<!-- The banner is h-8 (action-button height + a couple px so the buttons don't
	     touch the edges). We reserve only about a THIRD of that height when idle
	     (h-2.5) and grow to the full h-8 when the banner shows: this keeps the empty
	     gap small when there are no unsaved changes while limiting the content shift
	     on appear to ~22px. The height animates so the shift reads as a smooth expand. -->
	<div class={twMerge('shrink-0 transition-[height] duration-150', visible ? 'h-8' : 'h-2.5')}>
		{#if visible}
			<div
				transition:fade|local={{ duration: 120 }}
				class={twMerge(
					'flex flex-row items-center justify-between gap-2 px-4 h-full',
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
	</div>
{/if}
