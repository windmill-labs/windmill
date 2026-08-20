<script lang="ts" module>
	export const IMPORT_WIZARD_LABELS = ['Destination', 'Workspace', 'Import']
	/** The optional fourth: shown only for a project whose data tables need configuring. */
	export const IMPORT_WIZARD_SETUP_LABEL = 'Set up'
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import { goto } from '$lib/navigation'
	import Stepper from '$lib/components/common/stepper/Stepper.svelte'
	import { importIsRunning } from '$lib/importWizard/execution.svelte'
	import type { WizardStep } from '$lib/importWizard/plan'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		/** 1-based; Stepper is 0-based, hence the -1 below. */
		step: WizardStep
		/** Whether this import has a setup step at all — most projects do not. */
		hasSetup?: boolean
	}

	let { step, hasSetup = false }: Props = $props()

	// Most projects ship no data table migrations, so the wizard is three steps and
	// says so. A fourth appears only once there is something to configure.
	const tabs = $derived(
		hasSetup ? [...IMPORT_WIZARD_LABELS, IMPORT_WIZARD_SETUP_LABEL] : IMPORT_WIZARD_LABELS
	)

	// `maxReachedIndex` is the current step, so Stepper renders everything past it as
	// unreachable and only the steps behind it as clickable — the wizard has no way to
	// skip ahead, since each step decides what the next one asks.
	function onStepClick(index: number) {
		if (index >= step - 1) return
		// An import in flight owns the page: stepping back unmounts the step that is
		// awaiting the migration review, which would leave the run with no controls
		// and no way to resolve.
		if (importIsRunning()) {
			sendUserToast('Wait for the import to finish before going back.', true)
			return
		}
		// Every step shares one route, so going back is a `step` rewrite that leaves
		// the rest of the wizard's state in the URL alone.
		const params = new URLSearchParams($page.url.search)
		params.set('step', String(index + 1))
		goto(`/projects/import?${params}`)
	}
</script>

<div class="mb-5 flex justify-center">
	<!-- `small`: this steers a dialog, not a page. -->
	<Stepper
		{tabs}
		small
		selectedIndex={step - 1}
		maxReachedIndex={step - 1}
		on:click={(e) => onStepClick(e.detail.index)}
	/>
</div>
