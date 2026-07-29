<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import type { Trigger } from '$lib/components/triggers/utils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import { twMerge } from 'tailwind-merge'
	import TriggerLabel from '$lib/components/triggers/TriggerLabel.svelte'
	import { triggerDraftKind, triggerIconMap } from '$lib/components/triggers/utils'
	import { Star } from 'lucide-svelte'
	import ToggleButtonGroup from '../toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../toggleButton-v2/ToggleButton.svelte'
	import { userStore } from '$lib/stores'
	import Badge from '../badge/Badge.svelte'

	interface Props {
		open?: boolean
		draftTriggers?: Trigger[]
		isFlow?: boolean
	}

	let { open = $bindable(false), draftTriggers = [], isFlow = false }: Props = $props()

	let selectedTriggers: Trigger[] = $state(untrack(() => draftTriggers))

	const dispatch = createEventDispatcher<{
		canceled: void
		confirmed: { selectedTriggers: Trigger[] }
	}>()

	/** Identity across the list. Draft-backed triggers are keyed by their draft
	 * path; the kinds without a draft row have no path until they deploy, so they
	 * fall back to the client id assigned when they were added. */
	function triggerKey(trigger: Trigger): string {
		return `${trigger.type}:${trigger.path ?? trigger.id ?? ''}`
	}

	function toggleTrigger(trigger: Trigger, selected: 'skip' | 'deploy') {
		if (selected === 'skip') {
			selectedTriggers = selectedTriggers.filter((t) => triggerKey(t) !== triggerKey(trigger))
		} else if (!isSelected(selectedTriggers, trigger)) {
			selectedTriggers = [...selectedTriggers, trigger]
		}
	}

	function isSelected(triggers: Trigger[], trigger: Trigger): boolean {
		return triggers.some((t) => triggerKey(t) === triggerKey(trigger))
	}

	function checkSavePermissions(trigger: Trigger) {
		// Creating http trigger is forbidden for non-admin users
		const adminOnly =
			trigger.type === 'http' &&
			!$userStore?.is_admin &&
			!$userStore?.is_super_admin &&
			trigger.isDraft

		// Only the kinds without a draft row carry their config here. For the rest
		// it lives server-side, where an incomplete one surfaces as a deploy error —
		// non-destructive, since a failed deploy leaves the draft in place.
		const invalidConfig = !triggerDraftKind(trigger.type) && !trigger.draftConfig?.canSave

		return invalidConfig ? 'invalid-config' : adminOnly ? 'admin-only' : 'deploy'
	}

	$effect(() => {
		open &&
			(selectedTriggers = [...draftTriggers].filter((t) => checkSavePermissions(t) === 'deploy'))
	})
</script>

<ConfirmationModal
	{open}
	title="Undeployed trigger changes"
	confirmationText={isFlow ? 'Deploy Flow' : 'Deploy Script'}
	type="reload"
	showIcon={false}
	on:canceled={() => dispatch('canceled')}
	on:confirmed={() => dispatch('confirmed', { selectedTriggers })}
>
	<div class="flex flex-col w-full gap-8 pb-4">
		<div class="text-secondary text-sm">
			{`${isFlow ? 'Your flow' : 'Your script'} has undeployed trigger changes. Select which to deploy alongside the ${isFlow ? 'flow' : 'script'}; the rest stay as drafts.`}
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
											class={isSelectedTrigger ? 'text-accent' : 'text-hint'}
										/>
										{#if trigger.isPrimary}
											<Star size={8} class="absolute -mt-3 ml-3 text-accent" />
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
											class="w-fit h-fit"
											selected={isSelectedTrigger ? 'deploy' : 'skip'}
											on:selected={(e) => toggleTrigger(trigger, e.detail)}
										>
											{#snippet children({ item })}
												<ToggleButton
													label="Skip"
													value={'skip'}
													{item}
													small
													class="data-[state=on]:text-white data-[state=on]:bg-red-400 justify-center"
												/>
												<ToggleButton
													label={trigger.isDraft ? 'Deploy' : 'Update'}
													value={'deploy'}
													{item}
													small
													class="data-[state=on]:bg-surface-accent-primary data-[state=on]:text-white justify-center"
												/>
											{/snippet}
										</ToggleButtonGroup>
									</div>
								{:else if permission === 'admin-only'}
									<Badge color="red">Admin only</Badge>
								{:else if permission === 'invalid-config'}
									<Badge color="red">Invalid config</Badge>
								{/if}
							</td>
						</tr>
					{/each}

					{#if draftTriggers.length === 0}
						<tr>
							<td colspan="3" class="text-center py-6 text-gray-500 dark:text-gray-400 text-sm">
								No undeployed trigger changes
							</td>
						</tr>
					{/if}
				</tbody>
			</DataTable>
		</div>
	</div>
</ConfirmationModal>
