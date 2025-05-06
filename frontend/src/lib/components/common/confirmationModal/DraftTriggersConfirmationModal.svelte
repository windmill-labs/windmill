<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { createEventDispatcher } from 'svelte'
	import type { Trigger } from '$lib/components/triggers/utils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import { twMerge } from 'tailwind-merge'
	import TriggerLabel from '$lib/components/triggers/TriggerLabel.svelte'
	import { triggerIconMap } from '$lib/components/triggers/utils'
	import { Star } from 'lucide-svelte'
	import ToggleButtonGroup from '../toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../toggleButton-v2/ToggleButton.svelte'
	import { userStore } from '$lib/stores'

	interface Props {
		open?: boolean
		draftTriggers?: Trigger[]
	}

	let { open = $bindable(false), draftTriggers = [] }: Props = $props()

	let selectedTriggers: Trigger[] = $state(draftTriggers)

	const dispatch = createEventDispatcher<{
		canceled: void
		confirmed: { selectedTriggers: Trigger[] }
	}>()

	function toggleTrigger(trigger: Trigger, selected: 'discard' | 'deploy') {
		if (selected === 'discard') {
			if (trigger.isDraft) {
				selectedTriggers = selectedTriggers.filter((t) => t.id !== trigger.id)
			} else {
				selectedTriggers = selectedTriggers.filter(
					(t) => t.path !== trigger.path && t.type !== trigger.type
				)
			}
		} else if (!isSelected(selectedTriggers, trigger)) {
			selectedTriggers = [...selectedTriggers, trigger]
		}
	}

	function isSelected(triggers: Trigger[], trigger: Trigger): boolean {
		if (trigger.isDraft) {
			return triggers.some((t) => t.id === trigger.id)
		} else {
			return triggers.some((t) => t.path === trigger.path && t.type === trigger.type)
		}
	}

	function checkSavePermissions(trigger: Trigger) {
		// Creating http trigger is forbidden for non-admin users
		return (
			trigger.type !== 'http' ||
			$userStore?.is_admin ||
			$userStore?.is_super_admin ||
			!trigger.isDraft
		)
	}

	$effect(() => {
		open && (selectedTriggers = [...draftTriggers].filter(checkSavePermissions))
	})
</script>

<ConfirmationModal
	{open}
	title="Draft triggers detected !"
	confirmationText="Deploy Flow"
	type="reload"
	showIcon={false}
	on:canceled={() => dispatch('canceled')}
	on:confirmed={() => dispatch('confirmed', { selectedTriggers })}
>
	<div class="flex flex-col w-full gap-8 pb-4">
		<div class="text-secondary text-sm">
			Your flow has draft triggers. Select which draft triggers to deploy with your flow. Undeployed
			draft triggers will be permanently deleted.
		</div>

		<div class={draftTriggers.length > 5 ? 'h-[300px]' : ''}>
			<DataTable size="sm" tableFixed={true}>
				<thead>
					<tr class="bg-gray-50 dark:bg-gray-800 text-secondary dark:text-gray-300 text-xs">
						<th class="text-left py-2 px-4">Trigger to deploy</th>
						<th class="w-32 text-center py-2 px-1 justify-center"> </th>
					</tr>
				</thead>
				<tbody>
					{#each draftTriggers as trigger}
						{@const SvelteComponent = triggerIconMap[trigger.type]}
						{@const canSave = checkSavePermissions(trigger)}
						<tr
							class={twMerge(
								'transition-colors h-12 border-t border-gray-200 dark:border-gray-700 whitespace-nowrap',
								!canSave ? '' : 'hover:bg-surface-hover '
							)}
						>
							<td
								class={twMerge(
									'text-center py-1 px-4',
									isSelected(selectedTriggers, trigger) ? '' : 'opacity-50'
								)}
							>
								<div class="flex flex-row items-center gap-2">
									<div class="relative flex justify-center items-center">
										<SvelteComponent
											size={14}
											class={isSelected(selectedTriggers, trigger)
												? 'text-blue-500'
												: 'text-gray-500 dark:text-gray-400'}
										/>
										{#if trigger.isPrimary}
											<Star size={8} class="absolute -mt-3 ml-3 text-blue-400" />
										{/if}
									</div>
									<div class="grow min-w-0 items-center text-left">
										<TriggerLabel {trigger} />
									</div>
								</div>
							</td>

							<td class="text-center py-1">
								{#if canSave}
									<div class="flex justify-center">
										<ToggleButtonGroup
											let:item
											class="w-fit h-fit"
											selected={isSelected(selectedTriggers, trigger) ? 'deploy' : 'discard'}
											on:selected={(e) => toggleTrigger(trigger, e.detail)}
										>
											<ToggleButton
												label="Discard"
												value={'discard'}
												{item}
												small
												class="data-[state=on]:text-red-500"
											/>
											<ToggleButton
												label="Deploy"
												value={'deploy'}
												{item}
												small
												class="data-[state=on]:text-blue-500"
											/>
										</ToggleButtonGroup>
									</div>
								{:else}
									<span
										class="text-2xs px-1.5 py-1.5 bg-yellow-50 dark:bg-yellow-900/40 text-yellow-800 dark:text-yellow-100/90 rounded whitespace-nowrap"
										title="Only admins can deploy http triggers"
									>
										Admin only
									</span>
								{/if}
							</td>
						</tr>
					{/each}

					{#if draftTriggers.length === 0}
						<tr>
							<td colspan="3" class="text-center py-6 text-gray-500 dark:text-gray-400 text-sm">
								No draft triggers found
							</td>
						</tr>
					{/if}
				</tbody>
			</DataTable>
		</div>
	</div>
</ConfirmationModal>
