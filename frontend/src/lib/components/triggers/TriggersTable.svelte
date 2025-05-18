<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus, Star, Loader2, Trash, EllipsisVertical, RotateCcw } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Item } from '$lib/utils'
	import { triggerIconMap, type Trigger, type TriggerType } from './utils'
	import AddTriggersButton from './AddTriggersButton.svelte'
	import TriggerLabel from './TriggerLabel.svelte'

	interface Props {
		// Props
		selectedTrigger?: number | undefined
		triggers?: Trigger[]
		isEditor?: boolean
		onSelect?: (trigger: number) => void
		onAddDraftTrigger?: (type: TriggerType) => void
		onDeleteDraft?: (triggerIndex: number) => void
		onReset?: (triggerIndex: number) => void
	}

	let {
		selectedTrigger = undefined,
		triggers = [],
		isEditor = false,
		onSelect,
		onAddDraftTrigger,
		onDeleteDraft,
		onReset
	}: Props = $props()

	// Component state
	let loading = false

	const editTriggerItems = (triggerIndex: number, hasDraft: boolean): Item[] =>
		[
			{
				displayName: 'Reset to deployed version',
				action: () => {
					onReset?.(triggerIndex)
				},
				icon: RotateCcw,
				hidden: !hasDraft
			}
		].filter((item) => item.hidden !== true)
</script>

<div class="flex flex-col space-y-2 w-full">
	<div class="w-full">
		<AddTriggersButton {onAddDraftTrigger} setDropdownWidthToButtonWidth class="w-full" {isEditor}>
			<Button
				size="xs"
				color="blue"
				startIcon={{ icon: Plus }}
				nonCaptureEvent
				btnClasses="w-full justify-center"
			>
				<span>Add trigger</span>
			</Button>
		</AddTriggersButton>
	</div>
	<DataTable {loading} size="sm" tableFixed={true}>
		<tbody>
			{#each triggers as trigger, index}
				{@const SvelteComponent = triggerIconMap[trigger.type]}
				<tr
					class={twMerge(
						'hover:bg-surface-hover cursor-pointer h-10',
						selectedTrigger === index ? 'bg-surface-hover ' : ''
					)}
					onclick={() => onSelect?.(index)}
				>
					<td class="w-12 text-center py-2 px-2">
						<div class="relative flex justify-center items-center">
							<SvelteComponent
								size={16}
								class={trigger.isDraft ? 'text-frost-400' : 'text-tertiary'}
							/>

							{#if trigger.isPrimary}
								<Star size={10} class="absolute -mt-3 ml-3 text-blue-400" />
							{/if}
						</div>
					</td>
					<td class="py-2 px-2 text-xs">
						<div class="flex items-center justify-between gap-1">
							<div class="flex items-center grow min-w-0">
								<TriggerLabel {trigger} />
							</div>

							{#if !['email', 'webhook', 'cli'].includes(trigger.type)}
								{#if trigger.isDraft}
									<Button
										size="xs"
										color="light"
										btnClasses="hover:bg-red-500 hover:text-white bg-transparent px-1"
										startIcon={{ icon: Trash }}
										iconOnly
										on:click={() => onDeleteDraft?.(index)}
									/>
								{:else}
									<DropdownV2
										items={editTriggerItems(index, !!trigger.draftConfig && !trigger.isDraft)}
										placement="bottom-end"
										class="w-fit h-fit px-3"
									>
										{#snippet buttonReplacement()}
											<div class="-m-2 w-4 h-6 flex items-center justify-end">
												<EllipsisVertical size={14} />
											</div>
										{/snippet}
									</DropdownV2>
								{/if}
							{/if}
						</div>
					</td>
				</tr>
			{/each}

			{#if !loading && triggers.length === 0}
				<tr>
					<td colspan="2" class="text-center py-4 text-tertiary text-sm"> No triggers found </td>
				</tr>
			{/if}
			{#if loading && triggers.length === 0}
				<tr>
					<td colspan="2" class="text-center py-4 text-tertiary text-sm">
						<div class="flex justify-center items-center gap-2">
							<Loader2 class="animate-spin" size={16} />
							<span>Loading triggers...</span>
						</div>
					</td>
				</tr>
			{/if}
		</tbody>
	</DataTable>
</div>
