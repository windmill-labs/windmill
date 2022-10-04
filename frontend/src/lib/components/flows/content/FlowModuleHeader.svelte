<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { isEmptyFlowModule } from '$lib/components/flows/flowStateUtils'

	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'

	import {
		faArrowRotateBack,
		faArrowRotateForward,
		faBed,
		faCodeBranch,
		faSave,
		faStop,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
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
	$: moduleRetry = module.retry?.constant || module.retry?.exponential
</script>

<div class="flex flex-row space-x-2">
	{#if !shouldPick}
		<span
			class={classNames('badge', module.stop_after_if ? 'badge-on' : 'badge-off')}
			on:click={() => dispatch('toggleStopAfterIf')}
		>
			<Icon data={faStop} scale={0.8} />
		</span>
		<span
			class={classNames('badge', moduleRetry ? 'badge-on' : 'badge-off')}
			on:click={() => dispatch('toggleRetry')}
		>
			<Icon data={faArrowRotateForward} scale={0.8} />
		</span>
		<span
			class={classNames('badge', Boolean(module.suspend) ? 'badge-on' : 'badge-off')}
			on:click={() => dispatch('toggleSuspend')}
		>
			<Icon data={faBed} scale={0.8} />
		</span>
	{/if}
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

<style>
	.badge {
		@apply whitespace-nowrap text-sm font-medium border px-2.5 py-0.5 rounded cursor-pointer flex items-center;
	}

	.badge-on {
		@apply bg-blue-100 text-blue-800 hover:bg-blue-200;
	}

	.badge-off {
		@apply bg-gray-100 text-gray-800 hover:bg-gray-200;
	}
</style>
