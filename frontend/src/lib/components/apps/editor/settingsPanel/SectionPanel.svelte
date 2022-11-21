<script lang="ts">
	import { Alert } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import type { AppSection } from '../../types'
	import PanelSection from './common/PanelSection.svelte'

	export let section: AppSection | undefined

	const dispatch = createEventDispatcher()

	let deleteConfirmedCallback: (() => void) | undefined = undefined
	$: open = Boolean(deleteConfirmedCallback)

	function deleteSection(event: CustomEvent<PointerEvent>) {
		if (
			section &&
			Array.isArray(section.components) &&
			section?.components.length > 0 &&
			!event.detail.shiftKey
		) {
			deleteConfirmedCallback = () => {
				dispatch('remove')
				deleteConfirmedCallback = undefined
			}
		} else {
			dispatch('remove')
		}
	}
</script>

{#if section}
	<div class="flex flex-col w-full divide-y">
		<span class="text-sm border-y w-full py-1 px-2 bg-gray-800 text-white">Section editor</span>

		<PanelSection title="Danger zone">
			<Button
				size="xs"
				variant="border"
				color="red"
				startIcon={{ icon: faTrashAlt }}
				on:click={deleteSection}
			>
				Delete section
			</Button>
		</PanelSection>
	</div>
{/if}

<ConfirmationModal
	{open}
	title="Remove section"
	confirmationText="Remove"
	on:canceled={() => {
		deleteConfirmedCallback = undefined
	}}
	on:confirmed={() => {
		if (deleteConfirmedCallback) {
			deleteConfirmedCallback()
		}
		deleteConfirmedCallback = undefined
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove this section?</span>
		<Alert type="info" title="Bypass confirmation">
			<div>
				You can press
				<Badge color="dark-gray">SHIFT</Badge>
				while removing a resource to bypass confirmation.
			</div>
		</Alert>
	</div>
</ConfirmationModal>
