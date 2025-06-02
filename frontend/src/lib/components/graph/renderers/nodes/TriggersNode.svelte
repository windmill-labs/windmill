<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import NodeWrapper from './NodeWrapper.svelte'
	import TriggersWrapper from '../triggers/TriggersWrapper.svelte'
	import type { TriggersCount } from '$lib/gen'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import { Maximize2, Minimize2, Calendar } from 'lucide-svelte'
	import { getStateColor, getStateHoverColor } from '../../util'
	import { setScheduledPollSchedule, type TriggerContext } from '$lib/components/triggers'
	import VirtualItemWrapper from '$lib/components/flows/map/VirtualItemWrapper.svelte'
	import { type Trigger, type TriggerType } from '$lib/components/triggers/utils'
	import { tick } from 'svelte'
	import type { TriggerN } from '../../graphBuilder.svelte'

	interface Props {
		data: TriggerN['data']
	}

	let { data }: Props = $props()

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')

	const { triggersCount, triggersState } = $state(getContext<TriggerContext>('TriggerContext'))

	function getScheduleCfg(primary: Trigger | undefined, triggersCount: TriggersCount | undefined) {
		return primary?.draftConfig
			? {
					enabled: primary?.draftConfig?.enabled,
					schedule: primary?.draftConfig?.schedule
				}
			: primary?.lightConfig
				? { enabled: primary?.lightConfig?.enabled, schedule: primary?.lightConfig?.schedule }
				: {
						enabled: !!triggersCount?.primary_schedule,
						schedule: triggersCount?.primary_schedule?.schedule
					}
	}
</script>

<NodeWrapper wrapperClass="shadow-md rounded-sm">
	{#snippet children({ darkMode })}
		{#if data.simplifiableFlow?.simplifiedFlow != true}
			<TriggersWrapper
				disableAi={data.disableAi}
				isEditor={data.isEditor}
				path={data.path}
				bgColor={getStateColor(undefined, darkMode)}
				bgHoverColor={getStateHoverColor(undefined, darkMode)}
				showDraft={data.isEditor ?? false}
				on:new={(e) => {
					data?.eventHandlers.insert({
						sourceId: 'Input',
						index: 0,
						kind: 'trigger',
						inlineScript: e.detail.inlineScript
					})
					data?.eventHandlers?.simplifyFlow(true)
				}}
				on:pickScript={(e) => {
					data?.eventHandlers.insert({
						sourceId: 'Input',
						index: 0,
						kind: 'trigger',
						script: e.detail
					})
					data?.eventHandlers?.simplifyFlow(true)
				}}
				on:openScheduledPoll={(e) => {
					const primarySchedule = triggersState.triggers.findIndex((t) => t.isPrimary && !t.isDraft)
					triggersState.selectedTriggerIndex = primarySchedule
				}}
				on:select={() => data?.eventHandlers?.select('triggers')}
				onSelect={async (triggerIndex: number) => {
					data?.eventHandlers?.select('triggers')
					await tick()
					triggersState.selectedTriggerIndex = triggerIndex
				}}
				onAddDraftTrigger={async (type: TriggerType) => {
					const newTrigger = triggersState.addDraftTrigger(triggersCount, type)
					data?.eventHandlers?.select('triggers')
					await tick()
					triggersState.selectedTriggerIndex = newTrigger
				}}
				selected={$selectedId == 'triggers'}
				newItem={data.newFlow}
			/>
		{:else}
			<VirtualItemWrapper
				label="Check for new events"
				selectable={true}
				selected={$selectedId == 'triggers'}
				id={'triggers'}
				bgColor={getStateColor(undefined, darkMode)}
				on:select={(e) => {
					data?.eventHandlers?.select(e.detail)
				}}
			>
				{#if triggersState.triggers.some((t) => t.isPrimary) || $triggersCount?.primary_schedule}
					{@const { enabled, schedule } = getScheduleCfg(
						triggersState.triggers.find((t) => t.isPrimary),
						$triggersCount
					)}
					<div class="text-2xs text-primary p-2 flex gap-2 items-center">
						<Calendar size={12} />
						<div>
							Schedule every {schedule}
							{enabled ? '' : ' (disabled)'}
						</div>
					</div>
				{:else}
					<button
						class="px-2 py-1 hover:bg-surface-inverse w-full hover:text-primary-inverse"
						onclick={() => {
							setScheduledPollSchedule(triggersState, triggersCount)
						}}
					>
						Set primary schedule
					</button>
				{/if}
			</VirtualItemWrapper>
		{/if}
		{#if data.simplifiableFlow != undefined}
			<button
				class="absolute -top-[10px] -right-[10px] rounded-full h-[20px] w-[20px] trash center-center text-secondary
	outline-[1px] outline dark:outline-gray-500 outline-gray-300 bg-surface duration-0 hover:bg-nord-950 hover:text-white"
				onclick={stopPropagation(
					preventDefault(() =>
						data?.eventHandlers?.simplifyFlow(!data.simplifiableFlow?.simplifiedFlow)
					)
				)}
				title={data.simplifiableFlow?.simplifiedFlow
					? 'Expand to full flow view'
					: 'Simplify flow view for scheduled poll'}
			>
				{#if data.simplifiableFlow?.simplifiedFlow}
					<Maximize2 size={12} strokeWidth={2} />
				{:else}
					<Minimize2 size={12} strokeWidth={2} />
				{/if}
			</button>
		{/if}
	{/snippet}
</NodeWrapper>
