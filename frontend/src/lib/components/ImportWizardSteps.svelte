<script lang="ts" module>
	export const IMPORT_WIZARD_LABELS = ['Destination', 'Workspace', 'Import']
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import { goto } from '$lib/navigation'
	import Stepper from '$lib/components/common/stepper/Stepper.svelte'

	interface Props {
		/** 1-based; Stepper is 0-based, hence the -1 below. */
		step: 1 | 2 | 3
	}

	let { step }: Props = $props()

	// `maxReachedIndex` is the current step, so Stepper renders everything past it as
	// unreachable and only the steps behind it as clickable — the wizard has no way to
	// skip ahead, since each step decides what the next one asks.
	function onStepClick(index: number) {
		if (index >= step - 1) return
		// All three steps share one route, so going back is a `step` rewrite that
		// leaves the rest of the wizard's state in the URL alone.
		const params = new URLSearchParams($page.url.search)
		params.set('step', String(index + 1))
		goto(`/projects/import?${params}`)
	}
</script>

<div class="mb-5 flex justify-center">
	<Stepper
		tabs={IMPORT_WIZARD_LABELS}
		selectedIndex={step - 1}
		maxReachedIndex={step - 1}
		on:click={(e) => onStepClick(e.detail.index)}
	/>
</div>
