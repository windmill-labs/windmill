<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import { createEventDispatcher } from 'svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus, Star, Loader2, Trash, Pen, EllipsisVertical } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Item } from '$lib/utils'
	import { triggerIconMap, type Trigger } from './utils'
	import AddTriggersButton from './AddTriggersButton.svelte'
	// Props
	export let selectedTrigger: { path: string; type: string; isDraft?: boolean } | null = null
	export let triggers: Trigger[] = []

	// Component state
	let loading = false

	// Event handling
	const dispatch = createEventDispatcher<{
		select: Trigger
		delete: Trigger
		deleteDraft: { trigger: Trigger | undefined; keepSelection: boolean }
		edit: Trigger
	}>()

	const editTriggerItems = (trigger: Trigger): Item[] => [
		{
			displayName: 'Edit',
			action: () => {
				dispatch('edit', trigger)
			},
			icon: Pen,
			disabled: !trigger.canWrite
		}
	]

	// Select a trigger
	function selectTrigger(trigger: Trigger) {
		dispatch('select', trigger)
	}

	export function deleteDraft(trigger: Trigger | undefined, keepSelection: boolean = false) {
		dispatch('deleteDraft', { trigger, keepSelection })
	}
</script>

<div class="flex flex-col space-y-2 w-full">
	<div class="w-full">
		<AddTriggersButton on:addDraftTrigger setDropdownWidthToButtonWidth class="w-full">
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
			{#each triggers as trigger}
				<tr
					class={twMerge(
						'hover:bg-surface-hover cursor-pointer border-b border-t border-transparent',
						selectedTrigger &&
							selectedTrigger.path === trigger.path &&
							selectedTrigger.type === trigger.type &&
							selectedTrigger.isDraft === trigger.isDraft
							? 'bg-surface-hover '
							: ''
					)}
					on:click={() => selectTrigger(trigger)}
				>
					<td class="w-12 text-center py-2 px-2">
						<div class="flex justify-center items-center">
							<svelte:component
								this={triggerIconMap[trigger.type]}
								size={16}
								class={trigger.isDraft ? 'text-frost-400' : 'text-tertiary'}
							/>

							{#if trigger.isPrimary}
								<Star size={10} class="absolute -mt-3 ml-3 text-blue-400" />
							{/if}
						</div>
					</td>
					<td class="py-2 px-2 text-xs">
						<div class="flex items-center justify-between gap-2">
							<div class="flex items-center">
								<span class={trigger.isDraft ? 'text-frost-400 italic' : ''}>
									{trigger.isDraft ? `New ${trigger.type.replace(/s$/, '')} trigger` : trigger.path}
								</span>

								{#if trigger.isPrimary}
									<span
										class="ml-2 bg-blue-50 dark:bg-blue-900/40 px-1.5 py-0.5 rounded text-xs text-blue-700 dark:text-blue-100"
									>
										Primary
									</span>
								{/if}

								{#if trigger.isDraft}
									<span
										class="ml-2 text-2xs bg-frost-100 dark:bg-frost-900 text-frost-800 dark:text-frost-100 px-1.5 py-0.5 rounded"
									>
										Draft
									</span>
								{/if}
							</div>

							{#if ['schedule', 'http'].includes(trigger.type)}
								{#if trigger.isDraft}
									<Button
										size="xs"
										color="light"
										btnClasses="hover:bg-red-500 hover:text-white bg-transparent"
										startIcon={{ icon: Trash }}
										iconOnly
										on:click={() => deleteDraft(trigger)}
									/>
								{:else}
									<DropdownV2
										items={editTriggerItems(trigger)}
										placement="bottom-end"
										class="w-fit h-fit px-3"
									>
										<svelte:fragment slot="buttonReplacement">
											<EllipsisVertical size={14} />
										</svelte:fragment>
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
