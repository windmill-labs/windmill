<script lang="ts">
	import NodeWrapper from './NodeWrapper.svelte'
	import TriggersWrapper from '../triggers/TriggersWrapper.svelte'
	import { type GraphEventHandlers, type SimplifiableFlow } from '../../graphBuilder'
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import { Maximize2, Minimize2, Calendar } from 'lucide-svelte'
	import { getStateColor } from '../../util'
	import type { TriggerContext } from '$lib/components/triggers'
	import VirtualItemWrapper from '$lib/components/flows/map/VirtualItemWrapper.svelte'

	export let data: {
		path: string
		isEditor: boolean
		newFlow: boolean
		extra_perms: Record<string, any>
		eventHandlers: GraphEventHandlers
		modules: FlowModule[]
		index: number
		disableAi: boolean
		simplifiableFlow: SimplifiableFlow
	}

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')

	const { primarySchedule, triggersCount, selectedTrigger } =
		getContext<TriggerContext>('TriggerContext')
</script>

<NodeWrapper wrapperClass="shadow-none" let:darkMode>
	{#if data.simplifiableFlow?.simplifiedFlow != true}
		<TriggersWrapper
			disableAi={data.disableAi}
			isEditor={data.isEditor}
			path={data.path}
			on:new={(e) => {
				data?.eventHandlers.insert({
					modules: data.modules,
					index: 0,
					kind: 'trigger',
					inlineScript: e.detail.inlineScript
				})
				data?.eventHandlers?.simplifyFlow(true)
			}}
			on:pickScript={(e) => {
				data?.eventHandlers.insert({
					modules: data.modules,
					index: 0,
					kind: 'trigger',
					script: e.detail
				})
				data?.eventHandlers?.simplifyFlow(true)
			}}
			on:openScheduledPoll={(e) => {
				$selectedTrigger = 'scheduledPoll'
			}}
			on:select={(e) => {
				data?.eventHandlers?.select('triggers')
			}}
			on:delete={(e) => {
				data.eventHandlers.delete(e, '')
			}}
			selected={$selectedId == 'triggers'}
			newItem={data.newFlow}
			modules={data.modules}
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
			{#if $primarySchedule}
				<div class="text-2xs text-primary p-2">
					<Calendar />
					Schedule every {$primarySchedule.cron}
					{$primarySchedule?.enabled ? '' : ' (disabled)'}
				</div>
			{:else}
				<button
					class="px-2 py-1 hover:bg-surface-inverse w-full hover:text-primary-inverse"
					on:click={() => {
						$primarySchedule = {
							enabled: true,
							summary: 'Check for new events every 5 minutes',
							cron: '0 */5 * * * *',
							timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
							args: {}
						}
						if ($triggersCount) {
							$triggersCount = {
								...($triggersCount ?? {}),
								schedule_count: ($triggersCount?.schedule_count ?? 0) + 1,
								primary_schedule: { schedule: $primarySchedule.cron }
							}
						}
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
outline-[1px] outline dark:outline-gray-500 outline-gray-300 bg-surface duration-150 hover:bg-nord-950 hover:text-white"
			on:click|preventDefault|stopPropagation={() =>
				data?.eventHandlers?.simplifyFlow(!data.simplifiableFlow?.simplifiedFlow)}
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
</NodeWrapper>
