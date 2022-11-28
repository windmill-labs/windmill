<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import BarChartComponent from '../components/charts/BarChartComponent.svelte'
	import DisplayComponent from '../components/DisplayComponent.svelte'
	import TableComponent from '../components/TableComponent.svelte'
	import TextComponent from '../components/common/TextComponent.svelte'
	import type { AppComponent, AppEditorContext } from '../types'
	import { displayData } from '../utils'
	import ButtonComponent from '../components/common/ButtonComponent.svelte'
	import PieChartComponent from '../components/charts/PieChartComponent.svelte'
	import CheckboxComponent from '../components/select/CheckboxComponent.svelte'

	export let component: AppComponent
	export let selected: boolean

	const { staticOutputs, mode } = getContext<AppEditorContext>('AppEditorContext')
</script>

<div class="h-full flex flex-col w-full">
	{#if selected}
		<span
			class={classNames(
				'text-white px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit absolute -top-5',
				selected ? 'bg-indigo-500' : 'bg-gray-500'
			)}
		>
			{displayData[component.type].name} - {component.id}
		</span>
	{/if}

	<div
		class={classNames(
			'border overflow-auto cursor-pointer  h-full bg-white',
			selected ? 'border-blue-500' : 'border-white',
			$mode === 'preview' ? 'border-white' : 'hover:border-blue-500'
		)}
	>
		{#if component.type === 'displaycomponent'}
			<DisplayComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'barchartcomponent'}
			<BarChartComponent
				{...component}
				bind:inputs={component.inputs}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'piechartcomponent'}
			<PieChartComponent
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
				bind:inputs={component.inputs}
			/>
		{:else if component.type === 'tablecomponent'}
			<TableComponent
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
				bind:inputs={component.inputs}
			/>
		{:else if component.type === 'textcomponent'}
			<TextComponent {...component} />
		{:else if component.type === 'buttoncomponent'}
			<ButtonComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'checkboxcomponent'}
			<CheckboxComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{/if}
	</div>
</div>
