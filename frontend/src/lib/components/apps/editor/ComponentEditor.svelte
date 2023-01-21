<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import BarChartComponent from '../components/dataDisplay/AppBarChart.svelte'
	import DisplayComponent from '../components/DisplayComponent.svelte'
	import TableComponent from '../components/table/AppTable.svelte'
	import TextComponent from '../components/dataDisplay/AppText.svelte'
	import type { AppComponent, AppEditorContext } from '../types'
	import ButtonComponent from '../components/buttons/AppButton.svelte'
	import PieChartComponent from '../components/dataDisplay/AppPieChart.svelte'
	import SelectComponent from '../components/selectInputs/AppSelect.svelte'
	import CheckboxComponent from '../components/selectInputs/AppCheckbox.svelte'
	import TextInputComponent from '../components/textInputs/AppTextInput.svelte'
	import NumberInputComponent from '../components/numberInputs/AppNumberInput.svelte'
	import DateInputComponent from '../components/dateInputs/AppDateInput.svelte'
	import ComponentHeader from './ComponentHeader.svelte'
	import AppForm from '../components/form/AppForm.svelte'
	import AppScatterChart from '../components/dataDisplay/AppScatterChart.svelte'
	import AppTimeseries from '../components/dataDisplay/AppTimeseries.svelte'
	import AppHtml from '../components/dataDisplay/AppHtml.svelte'
	import AppSliderInputs from '../components/numberInputs/AppSliderInputs.svelte'
	import AppFormButton from '../components/form/AppFormButton.svelte'
	import VegaLiteHtml from '../components/dataDisplay/VegaLiteHtml.svelte'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let pointerdown: boolean = false

	let hover = false
	const { staticOutputs, mode, connectingInput } = getContext<AppEditorContext>('AppEditorContext')
</script>

<div
	on:pointerenter={() => (hover = true)}
	on:pointerleave={() => (hover = false)}
	class="h-full flex flex-col w-full component"
>
	{#if $mode !== 'preview'}
		<ComponentHeader {hover} {pointerdown} {component} {selected} on:delete on:lock {locked} />
	{/if}

	<div
		on:pointerdown={(e) => {
			if ($mode === 'preview') {
				e?.stopPropagation()
			}
		}}
		class={classNames(
			'border h-full bg-white',
			selected && $mode !== 'preview' ? 'border-blue-500' : 'border-white',
			!selected && $mode !== 'preview' && !component.card ? 'border-gray-100' : '',
			$mode !== 'preview' && !$connectingInput.opened ? 'hover:border-blue-500' : '',
			component.softWrap ? '' : 'overflow-auto',
			$mode != 'preview' ? 'cursor-pointer' : '',
			'relative z-auto'
		)}
	>
		{#if component.type === 'displaycomponent'}
			<DisplayComponent
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'barchartcomponent'}
			<BarChartComponent
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'timeseriescomponent'}
			<AppTimeseries
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'htmlcomponent'}
			<AppHtml
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'vegalitecomponent'}
			<VegaLiteHtml
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'scatterchartcomponent'}
			<AppScatterChart
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
				bind:actionButtons={component.actionButtons}
			/>
		{:else if component.type === 'textcomponent'}
			<TextComponent
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'buttoncomponent'}
			<ButtonComponent
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'selectcomponent'}
			<SelectComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'formcomponent'}
			<AppForm
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'formbuttoncomponent'}
			<AppFormButton
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'checkboxcomponent'}
			<CheckboxComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'textinputcomponent'}
			<TextInputComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'passwordinputcomponent'}
			<TextInputComponent
				inputType="password"
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'dateinputcomponent'}
			<DateInputComponent
				inputType="date"
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'numberinputcomponent'}
			<NumberInputComponent {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'slidercomponent'}
			<AppSliderInputs {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{/if}
	</div>
</div>
