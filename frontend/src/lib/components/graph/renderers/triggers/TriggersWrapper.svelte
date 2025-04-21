<script lang="ts">
	import { NODE } from '../../util'
	import { createEventDispatcher } from 'svelte'
	import type { Trigger } from '$lib/components/triggers/utils'
	import TriggersBadge from './TriggersBadge.svelte'
	import TriggersBadgeV2 from './TriggersBadgeV2.svelte'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import type { FlowModule } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'
	import AddTriggersButton from '$lib/components/triggers/AddTriggersButton.svelte'
	import { Plus } from 'lucide-svelte'

	export let path: string
	export let newItem: boolean
	export let selected: boolean
	export let isEditor: boolean = false
	export let disableAi: boolean = false
	export let modules: FlowModule[] = []
	export let bgColor: string
	export let triggers: Trigger[] = []

	const dispatch = createEventDispatcher()
</script>

<div style={`width: ${NODE.width}px;`}>
	<button
		style="background-color: {bgColor} !important;"
		class="flex w-full flex-row gap-1 px-2 p-1 items-center {selected
			? 'outline  outline-2  outline-gray-600 rounded-sm dark:bg-white/5 dark:outline-gray-400'
			: ''}"
		on:click={() => {
			dispatch('select')
		}}
	>
		<div class="flex flex-col mr-1 ml-1">
			<div class="flex flex-row items-center text-2xs font-normal"> Triggers </div>
		</div>
		{#if isEditor}
			<TriggersBadgeV2
				showOnlyWithCount={false}
				{path}
				{newItem}
				isFlow
				{selected}
				{triggers}
				on:select
			/>
		{:else}
			<TriggersBadge showOnlyWithCount={false} {path} {newItem} isFlow {selected} on:select />
		{/if}
		{#if isEditor}
			<InsertModuleButton
				{disableAi}
				on:new
				on:pickScript
				on:select
				on:open={() => {
					dispatch('openScheduledPoll')
				}}
				kind="trigger"
				index={0}
				{modules}
				class={twMerge(
					'hover:bg-surface-hover rounded-md border text-xs w-[23px] h-[23px] relative center-center cursor-pointer bg-surface outline-0'
				)}
			/>
			<AddTriggersButton on:addDraftTrigger class="w-fit">
				<button
					class={twMerge(
						'hover:bg-slate-300 rounded-md border border-dashed border-secondary text-xs w-[23px] h-[23px] relative center-center cursor-pointer text-secondary'
					)}
				>
					<Plus size={12} />
				</button>
			</AddTriggersButton>
		{/if}
	</button>
</div>
