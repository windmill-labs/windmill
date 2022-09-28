<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { isEmptyFlowModule } from '$lib/components/flows/flowStateUtils'

	import type { FlowModule } from '$lib/gen'

	import { faCodeBranch, faSave, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import type { FlowModuleWidthContext } from './FlowModule.svelte'
	import RemoveStepConfirmationModal from './RemoveStepConfirmationModal.svelte'

	export let module: FlowModule

	let confirmationModalOpen = false

	$: shouldPick = isEmptyFlowModule(module)

	const dispatch = createEventDispatcher()
	const { selectedId, select } = getContext<FlowEditorContext>('FlowEditorContext')
	const { width, threshold } = getContext<FlowModuleWidthContext>('FlowModuleWidth')
	$: iconOnly = $width < threshold
</script>

<div class="flex flex-row space-x-2">
	{#if module.value.type === 'script' && !shouldPick}
		<Button
			size="xs"
			color="light"
			variant="border"
			on:click={() => dispatch('fork')}
			startIcon={{ icon: faCodeBranch }}
			{iconOnly}
		>
			Fork
		</Button>
	{/if}

	{#if module.value.type === 'rawscript' && !shouldPick}
		<Button
			size="xs"
			color="light"
			variant="border"
			startIcon={{ icon: faSave }}
			on:click={() => dispatch('createScriptFromInlineScript')}
			{iconOnly}
		>
			Save to workspace
		</Button>
	{/if}
	<Button
		size="xs"
		color="light"
		variant="border"
		startIcon={{ icon: faTrashAlt }}
		{iconOnly}
		on:click={(event) => {
			if (event.shiftKey || shouldPick) {
				dispatch('delete')
				select('settings')
			} else {
				confirmationModalOpen = true
			}
		}}
	>
		{$selectedId.includes('failure') ? 'Delete error handler' : 'Remove step'}
	</Button>
</div>

<RemoveStepConfirmationModal
	bind:open={confirmationModalOpen}
	on:confirmed={() => {
		dispatch('delete')
		setTimeout(() => {
			select('settings')
		})
	}}
/>
