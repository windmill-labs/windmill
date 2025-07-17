<script lang="ts">
	import { Trash } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { triggerIconMap, type Trigger } from './utils'
	import TriggerLabel from './TriggerLabel.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		onDelete: (() => void) | undefined
		trigger?: Trigger
		small?: boolean
	}

	let { onDelete, trigger = undefined, small = false }: Props = $props()

	let confirmationModalOpen = $state(false)
</script>

<ConfirmationModal
	title={`Are you sure you want to delete this ${trigger?.isDraft ? 'draft' : 'deployed'} trigger ?`}
	confirmationText="Delete"
	open={confirmationModalOpen}
	on:canceled={() => {
		confirmationModalOpen = false
	}}
	on:confirmed={() => {
		onDelete?.()
		confirmationModalOpen = false
	}}
>
	{#if trigger !== undefined}
		{@const IconComponent = triggerIconMap[trigger.type]}
		<div class="flex flex-row gap-2 items-center grow min-w-0 pr-2 rounded-md my-4">
			<IconComponent
				size={16}
				class={twMerge(trigger.isDraft ? 'text-frost-400' : '', 'shrink-0')}
			/>
			<TriggerLabel {trigger} />
		</div>
	{/if}
</ConfirmationModal>

<Button
	size="xs"
	startIcon={{ icon: Trash }}
	iconOnly
	color={'light'}
	on:click={() => {
		confirmationModalOpen = true
	}}
	btnClasses={twMerge(small ? 'px-1 py-1' : '', 'bg-transparent hover:bg-red-500 hover:text-white')}
/>
