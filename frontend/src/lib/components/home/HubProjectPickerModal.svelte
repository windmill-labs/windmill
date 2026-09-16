<script lang="ts">
	import { untrack } from 'svelte'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import HubTemplatePicker from './HubTemplatePicker.svelte'
	import type { HubProjectPick } from '$lib/hubProject'

	interface Props {
		open: boolean
		/** A project was chosen. The host closes this and opens the import dialog on it. */
		onPick: (project: HubProjectPick) => void
		onClose: () => void
	}

	let { open, onPick, onClose }: Props = $props()

	// Bound, not one-way, for the same reason the import dialog binds it: `Modal` dispatches
	// `confirmed`/`canceled` only, so the X, Escape and the backdrop are visible to the caller
	// through this value and nowhere else.
	let modalOpen = $state(false)
	let wasOpen = false
	$effect(() => {
		const shouldBeOpen = open
		if (shouldBeOpen !== untrack(() => modalOpen)) {
			modalOpen = shouldBeOpen
			if (shouldBeOpen) logFeatureUsage('home', 'template_picker_open', { key: 'new_menu' })
		}
	})
	$effect(() => {
		const isOpen = modalOpen
		untrack(() => {
			if (!isOpen && wasOpen) onClose()
			wasOpen = isOpen
		})
	})
</script>

<!-- A dialog rather than the empty state's popover: this one opens from an item inside an
     already-open dropdown, and a popover anchored there leaves two melt layers arguing over
     focus and dismissal. The height is fixed so the list inside has a definite box to page in,
     the same requirement the popover meets with `fitViewport`. -->
<!-- `kind="X"`: picking a card is the action, so a Cancel button under the list would be the
     only thing in the dialog that looks like one. -->
<Modal bind:open={modalOpen} kind="X" title="Import a hub project" class="sm:!max-w-[560px]">
	<div class="flex h-[min(70vh,520px)] flex-col">
		<HubTemplatePicker fullWidth {onPick} />
	</div>
</Modal>
