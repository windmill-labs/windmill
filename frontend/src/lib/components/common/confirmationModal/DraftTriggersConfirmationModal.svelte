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
		isFlow?: boolean
	}

	let { open = $bindable(false), draftTriggers = [], isFlow = false }: Props = $props()

	let selectedTriggers: Trigger[] = $state(draftTriggers)

	const dispatch = createEventDispatcher<{
		canceled: void
		confirmed: { selectedTriggers: Trigger[] }
	}>()

	function toggleTrigger(trigger: Trigger, selected: 'discard' | 'deploy') {
		if (selected === 'discard') {
			if (trigger.isDraft) {
				selectedTriggers = selectedTriggers.filter((t) => !t.isDraft || t.id !== trigger.id)
			} else {
				selectedTriggers = selectedTriggers.filter(
					(t) => t.isDraft || t.type !== trigger.type || t.path !== trigger.path
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
		const adminOnly =
			trigger.type === 'http' &&
			!$userStore?.is_admin &&
			!$userStore?.is_super_admin &&
			trigger.isDraft

		const invalidConfig = !trigger.draftConfig?.canSave

		return invalidConfig ? 'invalid-config' : adminOnly ? 'admin-only' : 'deploy'
	}

	$effect(() => {
		open &&
			(selectedTriggers = [...draftTriggers].filter((t) => checkSavePermissions(t) === 'deploy'))
	})
</script>

<ConfirmationModal
	{open}
	title="Draft triggers detected !"
	confirmationText={isFlow ? 'Deploy Flow' : 'Deploy Script'}
	type="reload"
	showIcon={false}
	on:canceled={() => dispatch('canceled')}
	on:confirmed={() => dispatch('confirmed', { selectedTriggers })}
>
	<div class="flex flex-col w-full gap-8 pb-4">
		<div class="text-secondary text-sm">
			{`${isFlow ? 'Your flow' : 'Your script'} has draft triggers. Select which draft triggers to deploy with the ${isFlow ? 'flow' : 'script'}. Undeployed
			draft triggers will be permanently deleted.`}
		</div>

		<div class={draftTriggers.length > 5 ? 'h-[300px]' : ''}>
			<DataTable size="sm" tableFixed={true}>
				<thead>
					<tr class="bg-gray-50 dark:bg-gray-700 text-secondary dark:text-gray-300 text-xs">
						<th class="text-left py-2 px-4">Triggers to deploy</th>
						<th class="w-32 text-center py-2 px-1 justify-center"> </th>
					</tr>
				</thead>
				<tbody>
					{#each draftTriggers as trigger}
						{@const SvelteComponent = triggerIconMap[trigger.type]}
						{@const permission = checkSavePermissions(trigger)}
						{@const isSelectedTrigger = isSelected(selectedTriggers, trigger)}
						<tr
							class={twMerge(
								'transition-colors h-12 border-t border-gray-200 dark:border-gray-700 whitespace-nowrap',
								permission === 'deploy' ? 'hover:bg-surface-hover ' : ''
							)}
						>
							<td class={twMerge('text-center py-1 px-4')}>
								<div class="flex flex-row items-center gap-2">
									<div class="relative flex justify-center items-center">
										<SvelteComponent
											size={14}
											class={isSelectedTrigger
												? 'text-blue-500'
												: 'text-gray-500 dark:text-gray-400'}
										/>
										{#if trigger.isPrimary}
											<Star size={8} class="absolute -mt-3 ml-3 text-blue-400" />
										{/if}
									</div>
									<div class="flex grow min-w-0 items-center text-left">
										<TriggerLabel {trigger} discard={!isSelectedTrigger} />
									</div>
								</div>
							</td>

							<td class="text-left py-1">
								{#if permission === 'deploy'}
									<div class="flex justify-start">
										<ToggleButtonGroup
											let:item
											class="w-fit h-fit"
											selected={isSelectedTrigger ? 'deploy' : 'discard'}
											on:selected={(e) => toggleTrigger(trigger, e.detail)}
										>
											<ToggleButton
												label={!trigger.isDraft && trigger.draftConfig ? 'Reset' : 'Discard'}
												value={'discard'}
												{item}
												small
												class="data-[state=on]:text-white data-[state=on]:bg-red-400 w-[54px] justify-center"
											/>
											<ToggleButton
												label={!trigger.isDraft && trigger.draftConfig ? 'Update' : 'Deploy'}
												value={'deploy'}
												{item}
												small
												class="data-[state=on]:bg-marine-400 data-[state=on]:text-white data-[state=on]:dark:bg-marine-50 data-[state=on]:dark:text-primary-inverse w-[54px] justify-center"
											/>
										</ToggleButtonGroup>
									</div>
								{:else if permission === 'admin-only'}
									<div
										class="text-xs font-semibold px-1.5 py-1.5 bg-red-400 text-white rounded whitespace-nowrap w-[114px] text-center"
										title="Only admins can deploy http triggers"
									>
										Admin only
									</div>
								{:else if permission === 'invalid-config'}
									<div
										class="text-xs font-semibold px-1.5 py-1.5 bg-red-400 text-white rounded whitespace-nowrap w-[114px] text-center"
										title="Invalid config"
									>
										Invalid config
									</div>
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
