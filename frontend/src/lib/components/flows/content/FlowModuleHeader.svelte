<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { isEmptyFlowModule } from '$lib/components/flows/flowStateUtils'

	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'

	import { faCodeBranch, faSave, faStop, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher, getContext } from 'svelte'
	import Icon from 'svelte-awesome'
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
	<span
		class={classNames(
			'whitespace-nowrap text-sm font-medium mr-2 px-2.5 py-0.5 rounded cursor-pointer flex items-center',
			module.stop_after_if
				? 'bg-sky-100 text-sky-800 hover:bg-sky-200'
				: 'bg-gray-100 text-gray-800 hover:bg-gray-200'
		)}
		on:click={() => dispatch('toggleStopAfterIf')}
	>
		<Icon data={faStop} class="mr-2" scale={0.8} />
		{module.stop_after_if ? `Early stop o` : 'Early stop x'}
	</span>
	<span
		class={classNames(
			'whitespace-nowrap text-sm font-medium mr-2 px-2.5 py-0.5 rounded cursor-pointer flex items-center',
			module.stop_after_if
				? 'bg-sky-100 text-sky-800 hover:bg-sky-200'
				: 'bg-gray-100 text-gray-800 hover:bg-gray-200'
		)}
		on:click={() => dispatch('toggleRetry')}
	>
		<Icon data={faStop} class="mr-2" scale={0.8} />
		{module.retry ? `Retry o` : 'Retry x'}
	</span>
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
