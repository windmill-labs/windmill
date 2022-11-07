<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import BarChartComponent from '../components/BarChartComponent.svelte'
	import DisplayComponent from '../components/DisplayComponent.svelte'
	import RunFormComponent from '../components/RunFormComponent.svelte'
	import TableComponent from '../components/TableComponent.svelte'
	import type { AppComponent, AppEditorContext } from '../types'

	export let component: AppComponent
	export let selected: boolean

	const { staticOutputs, app } = getContext<AppEditorContext>('AppEditorContext')
</script>

{#if component}
	<div class="h-full flex flex-col w-full">
		<div
			class={classNames(
				'p-2 border overflow-auto cursor-pointer hover:bg-blue-100 h-full bg-white relative',
				selected ? 'border-indigo-400' : 'border-gray-400'
			)}
		>
			<span
				class={classNames(
					' text-white px-2 text-xs py-1 font-bold rounded-t-sm w-fit absolute top-0 right-0',
					selected ? 'bg-indigo-500' : 'bg-gray-500'
				)}
			>
				{component.type}
			</span>
			{#if component.type === 'runformcomponent'}
				<RunFormComponent
					{...component}
					bind:staticOutputs={$staticOutputs[component.id]}
					triggerable={$app.policy.triggerables[component.id]}
				/>
			{:else if component.type === 'displaycomponent'}
				<DisplayComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
			{:else if component.type === 'barchartcomponent'}
				<BarChartComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
			{:else if component.type === 'tablecomponent'}
				<TableComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
			{/if}
		</div>
	</div>
{/if}
