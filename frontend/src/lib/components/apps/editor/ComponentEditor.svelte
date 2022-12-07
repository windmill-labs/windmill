<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import BarChartComponent from '../components/dataDisplay/AppBarChart.svelte'
	import DisplayComponent from '../components/DisplayComponent.svelte'
	import TableComponent from '../components/dataDisplay/AppTable.svelte'
	import TextComponent from '../components/dataDisplay/AppText.svelte'
	import type { AppComponent, AppEditorContext } from '../types'
	import ButtonComponent from '../components/buttons/AppButton.svelte'
	import PieChartComponent from '../components/dataDisplay/AppPieChart.svelte'
	import CheckboxComponent from '../components/selectInputs/AppCheckbox.svelte'
	import ComponentHeader from './ComponentHeader.svelte'

	export let component: AppComponent
	export let selected: boolean

	$: shouldDisplayOverlay = selected && $mode !== 'preview'

	const { staticOutputs, mode, connectingInput } = getContext<AppEditorContext>('AppEditorContext')
</script>

<div class="h-full flex flex-col w-full">
	{#if shouldDisplayOverlay}
		<ComponentHeader {component} {selected} />
	{/if}

	<div
		class={classNames(
			' border overflow-auto cursor-pointer  h-full bg-white',
			shouldDisplayOverlay ? 'border-blue-500' : 'border-white',
			!selected && $mode !== 'preview' && !component.card ? 'border-gray-100' : '',
			$mode !== 'preview' && !$connectingInput.opened ? 'hover:border-blue-500' : '',
			component.card ? 'p-2' : '',
			'relative'
		)}
	>
		{#if component.type === 'displaycomponent'}
			<DisplayComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'barchartcomponent'}
			<BarChartComponent
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'piechartcomponent'}
			<PieChartComponent
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
				bind:componentInput={component.componentInput}
			/>
		{:else if component.type === 'tablecomponent'}
			<TableComponent
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
				bind:componentInput={component.componentInput}
			/>
		{:else if component.type === 'textcomponent'}
			<TextComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'buttoncomponent'}
			<ButtonComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'checkboxcomponent'}
			<CheckboxComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{/if}
	</div>
</div>
