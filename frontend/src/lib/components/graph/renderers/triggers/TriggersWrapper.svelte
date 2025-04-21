<script lang="ts">
	import { NODE } from '../../util'

	import { createEventDispatcher } from 'svelte'

	import TriggersBadge from './TriggersBadge.svelte'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import type { FlowModule } from '$lib/gen'
	import { twMerge } from 'tailwind-merge'

	export let path: string
	export let newItem: boolean
	export let selected: boolean
	export let isEditor: boolean = false
	export let disableAi: boolean = false
	export let modules: FlowModule[] = []
	export let bgColor: string
	export let bgHoverColor: string = ''

	const dispatch = createEventDispatcher()

	let hover = false
</script>

<div style={`width: ${NODE.width}px;`}>
	<button
		style="background-color: {hover && bgHoverColor ? bgHoverColor : bgColor};"
		class="flex w-full flex-row gap-1 px-2 p-1 items-center {selected
			? 'outline  outline-2  outline-gray-600 rounded-sm dark:bg-white/5 dark:outline-gray-400'
			: ''}"
		on:pointerdown={() => {
			dispatch('select')
		}}
		on:mouseenter={() => (hover = true)}
		on:mouseleave={() => (hover = false)}
	>
		<div class="flex flex-col mr-1 ml-1">
			<div class="flex flex-row items-center text-2xs font-normal"> Triggers </div>
		</div>
		<TriggersBadge showOnlyWithCount={false} {path} {newItem} isFlow {selected} on:select />
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
		{/if}
	</button>
</div>
