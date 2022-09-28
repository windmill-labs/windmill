<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { isEmptyFlowModule } from '$lib/components/flows/flowStateUtils'

	import type { FlowModule } from '$lib/gen'
	import { faCodeBranch, faSave, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { Button } from 'flowbite-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { FlowEditorContext } from '../types'

	export let module: FlowModule

	let confirmationModalOpen = false

	$: shouldPick = isEmptyFlowModule(module)

	const dispatch = createEventDispatcher()
	const { selectedId, select } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<div class="flex flex-row space-x-2" on:click|stopPropagation={() => undefined}>
	{#if module.value.type === 'script' && !shouldPick}
		<Button size="xs" color="alternative" on:click={() => dispatch('fork')}>
			<Icon data={faCodeBranch} class="mr-2" />
			Fork
		</Button>
	{/if}

	{#if module.value.type === 'rawscript' && !shouldPick}
		<Button size="xs" color="alternative" on:click={() => dispatch('createScriptFromInlineScript')}>
			<Icon data={faSave} class="mr-2" />
			Save to workspace
		</Button>
	{/if}
	<Button
		size="xs"
		color="alternative"
		on:click={() => {
			confirmationModalOpen = true
		}}
	>
		<Icon data={faTrashAlt} class="mr-2" />
		{$selectedId.includes('failure') ? 'Delete error handler' : 'Remove step'}
	</Button>
</div>

<ConfirmationModal
	open={confirmationModalOpen}
	title="Remove step"
	description="Are you sure you want to remove this step?"
	confirmationText="Remove"
	on:canceled={() => {
		confirmationModalOpen = false
	}}
	on:confirmed={() => {
		select('settings')
		dispatch('delete')
		confirmationModalOpen = false
	}}
/>
