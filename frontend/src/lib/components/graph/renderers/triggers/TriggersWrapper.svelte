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

	const dispatch = createEventDispatcher()
</script>

<div style={`width: ${NODE.width}px;`}>
	<div class="flex flex-row mx-auto w-min">
		<button
			class="flex flex-row gap-2 px-2 border p-1 rounded-md bg-surface shadow-md items-center {selected
				? 'outline outline-offset-1 outline-1  outline-slate-900 dark:bg-white/5 dark:outline-slate-800/60 dark:border-gray-400'
				: ''}"
			on:click={() => {
				dispatch('select')
			}}
		>
			<div class="flex flex-col">
				<div class="flex flex-row items-center text-2xs font-normal"> Triggers </div>
			</div>
			<TriggersBadge showOnlyWithCount={false} {path} {newItem} isFlow {selected} on:select />
			{#if isEditor}
				<InsertModuleButton
					{disableAi}
					on:new
					on:pickScript
					on:select
					kind="trigger"
					index={0}
					{modules}
					class={twMerge(
						'hover:bg-surface-hover rounded-md border text-xs w-6 h-6 relative center-center cursor-pointer bg-surface outline-0'
					)}
				/>
			{/if}
		</button>
	</div>
</div>
