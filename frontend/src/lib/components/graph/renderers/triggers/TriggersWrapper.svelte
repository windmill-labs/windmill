<script lang="ts">
	import { NODE } from '../../util'
	import { createEventDispatcher } from 'svelte'
	import type { TriggerType } from '$lib/components/triggers/utils'
	import TriggersBadge from './TriggersBadge.svelte'
	import { Plus } from 'lucide-svelte'
	import InsertModuleInner from '$lib/components/flows/map/InsertModuleInner.svelte'
	import AddTriggersButton from '$lib/components/triggers/AddTriggersButton.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		path: string
		newItem: boolean
		selected: boolean
		isEditor?: boolean
		disableAi?: boolean
		bgColor: string
		bgHoverColor?: string
		showDraft?: boolean
		onSelect?: (triggerIndex: number) => void
		onAddDraftTrigger?: (type: TriggerType) => void
	}

	let {
		path,
		newItem,
		selected,
		isEditor = false,
		disableAi = false,
		bgColor,
		bgHoverColor = '',
		showDraft,
		onSelect,
		onAddDraftTrigger
	}: Props = $props()

	let showTriggerScriptPicker = $state(false)
	let numberOfTriggers = $state(0)

	const dispatch = createEventDispatcher()

	let hover = $state(false)
	let addTriggersButton = $state<AddTriggersButton | undefined>(undefined)
</script>

<div style={`width: ${NODE.width}px;`}>
	<button
		style="background-color: {hover && bgHoverColor ? bgHoverColor : bgColor};"
		class="relative flex w-full flex-row gap-1.5 px-2 p-1 items-center justify-center rounded-sm {selected
			? 'outline  outline-2  outline-gray-600 dark:bg-white/5 dark:outline-gray-400'
			: ''}"
		onclick={() => {
			dispatch('select')
		}}
		onmouseenter={() => (hover = true)}
		onmouseleave={() => (hover = false)}
	>
		<div
			class={twMerge(
				'flex flex-row items-center text-2xs font-normal',
				numberOfTriggers > 6 ? 'absolute left-0 -top-[20px]' : ''
			)}
		>
			Triggers
		</div>

		<TriggersBadge
			showOnlyWithCount={false}
			{showDraft}
			{path}
			{newItem}
			isFlow
			{selected}
			bind:numberOfTriggers
			limit={isEditor ? 7 : 8}
			{onSelect}
		/>

		{#if isEditor}
			<AddTriggersButton
				bind:this={addTriggersButton}
				onAddScheduledPoll={() => {
					showTriggerScriptPicker = true
				}}
				class="w-fit h-fit"
				triggerScriptPicker={showTriggerScriptPicker ? triggerScriptPicker : undefined}
				onClose={() => {
					showTriggerScriptPicker = false
				}}
				isEditor
				{onAddDraftTrigger}
			>
				<button
					class="hover:bg-slate-300 dark:hover:bg-slate-600 rounded-md outline-1 outline-dashed outline-secondary outline-offset-[-1px] text-xs w-[23px] h-[23px] relative center-center cursor-pointer text-secondary"
				>
					<Plus size={12} />
				</button>
			</AddTriggersButton>
		{/if}
	</button>
</div>

{#snippet triggerScriptPicker()}
	<div class="border rounded-lg shadow-lg bg-surface z5000">
		<InsertModuleInner
			{disableAi}
			on:new
			on:pickScript
			on:select
			on:open={() => {
				dispatch('openScheduledPoll')
			}}
			on:close={() => {
				addTriggersButton?.close()
			}}
			kind="trigger"
		/>
	</div>
{/snippet}
