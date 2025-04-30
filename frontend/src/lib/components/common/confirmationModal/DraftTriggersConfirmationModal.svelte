<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { createEventDispatcher } from 'svelte'
	import type { Trigger } from '$lib/components/triggers/utils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import { twMerge } from 'tailwind-merge'
	import TriggerLabel from '$lib/components/triggers/TriggerLabel.svelte'
	import { triggerIconMap } from '$lib/components/triggers/utils'
	import { Star, Check } from 'lucide-svelte'

	interface Props {
		open?: boolean
		draftTriggers?: Trigger[]
	}

	let { open = $bindable(false), draftTriggers = [] }: Props = $props()

	let selectedTriggers: Trigger[] = $state([])
	let selectAll = $state(false)
	let tableHeight = $state(0)

	const dispatch = createEventDispatcher<{
		canceled: void
		confirmed: { selectedTriggers: Trigger[] }
	}>()

	function toggleTrigger(trigger: Trigger) {
		if (isSelected(selectedTriggers, trigger)) {
			selectedTriggers = selectedTriggers.filter((t) => t.id !== trigger.id)
		} else {
			selectedTriggers = [...selectedTriggers, trigger]
		}
	}

	function isSelected(triggers: Trigger[], trigger: Trigger): boolean {
		return triggers.some((t) => t.id === trigger.id)
	}

	function handleSelectAll() {
		if (!selectAll) {
			selectedTriggers = []
		} else {
			selectedTriggers = [...draftTriggers]
		}
	}

	// Update selectAll based on selectedTriggers
	$effect(() => {
		selectAll = selectedTriggers.length === draftTriggers.length && draftTriggers.length > 0
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

		<div bind:clientHeight={tableHeight} class={draftTriggers.length > 5 ? 'h-[300px]' : ''}>
			<DataTable size="sm" tableFixed={true}>
				<thead>
					<tr class="bg-gray-50 dark:bg-gray-800 text-secondary dark:text-gray-300 text-xs">
						<th class="text-left py-2 px-4">Trigger to deploy</th>
						<th class="w-16 text-center py-2 px-1">
							<div class="center-center">
								<div
									class={twMerge(
										'h-4 w-4 rounded border center-center cursor-pointer transition-colors',
										selectAll
											? 'border-blue-500 bg-blue-500 text-white'
											: 'border-gray-300 dark:border-gray-600'
									)}
									onclick={() => {
										selectAll = !selectAll
										handleSelectAll()
									}}
									onkeydown={(e) => e.key === 'Enter' && handleSelectAll()}
									tabindex="0"
									role="checkbox"
									aria-checked={selectAll}
									id="select-all-triggers"
								>
									{#if selectAll}
										<Check size={12} />
									{/if}
								</div>
							</div>
						</th>
					</tr>
				</thead>
				<tbody>
					{#each draftTriggers as trigger}
						{@const SvelteComponent = triggerIconMap[trigger.type]}
						<tr
							class={twMerge(
								'hover:bg-surface-hover transition-colors h-12 border-t border-gray-200 dark:border-gray-700 cursor-pointer',
								isSelected(selectedTriggers, trigger) ? 'bg-blue-50 dark:bg-blue-900/20' : ''
							)}
							onclick={() => toggleTrigger(trigger)}
						>
							<td class="text-center py-1 px-4">
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
									<!-- svelte-ignore a11y_click_events_have_key_events -->
									<!-- svelte-ignore a11y_no_static_element_interactions -->
									<div class="grow min-w-0 items-center text-left">
										<TriggerLabel {trigger} />
									</div>
								</div>
							</td>

							<td class="text-center py-1">
								<div class="flex justify-center">
									<div
										class={twMerge(
											'h-4 w-4 rounded border center-center cursor-pointer transition-colors',
											isSelected(selectedTriggers, trigger)
												? 'border-blue-500 bg-blue-500 text-white'
												: 'border-gray-300 dark:border-gray-600'
										)}
										onkeydown={(e) => e.key === 'Enter' && toggleTrigger(trigger)}
										tabindex="0"
										role="checkbox"
										aria-checked={isSelected(selectedTriggers, trigger)}
									>
										{#if isSelected(selectedTriggers, trigger)}
											<Check size={12} />
										{/if}
									</div>
								</div>
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
