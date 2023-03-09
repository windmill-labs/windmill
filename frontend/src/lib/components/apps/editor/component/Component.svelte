<script lang="ts">
	import { getContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import { Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppEditorContext, GridItem } from '../../types'
	import ComponentHeader from '../ComponentHeader.svelte'
	import type { AppComponent } from './components'
	import {
		AppBarChart,
		AppDisplayComponent,
		AppTable,
		AppText,
		AppButton,
		AppPieChart,
		AppSelect,
		AppCheckbox,
		AppTextInput,
		AppNumberInput,
		AppDateInput,
		AppForm,
		AppScatterChart,
		AppTimeseries,
		AppHtml,
		AppSliderInputs,
		AppFormButton,
		VegaLiteHtml,
		PlotlyHtml,
		AppRangeInput,
		AppTabs,
		AppIcon,
		AppCurrencyInput,
		AppDivider,
		AppFileInput,
		AppImage,
		AppContainer,
		AppAggridTable,
		AppDrawer,
		AppMap,
		AppSplitpanes,
		AppPdf
	} from '../../components'
	import AppMultiSelect from '../../components/inputs/AppMultiSelect.svelte'
	import ComponentNavigation from './ComponentNavigation.svelte'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let pointerdown: boolean = false
	export let render: boolean
	export let parentId: string | undefined = undefined
	export let subGrid: GridItem[] | undefined = undefined

	const { staticOutputs, mode, connectingInput, app, errorByComponent } =
		getContext<AppEditorContext>('AppEditorContext')
	let hover = false
	let initializing: boolean | undefined = undefined
	let componentContainerHeight: number = 0

	$: componentWithErrors = Object.values($errorByComponent).map((e) => e.componentId)
	$: hasError = componentWithErrors.includes(component.id)
</script>

<div
	on:pointerenter={() => (hover = true)}
	on:pointerleave={() => (hover = false)}
	class="h-full flex flex-col w-full component"
>
	{#if $mode !== 'preview'}
		<ComponentHeader
			{hover}
			{pointerdown}
			{component}
			{selected}
			on:delete
			on:lock
			on:expand
			{locked}
		/>
	{/if}

	<div
		on:pointerdown={(e) => {
			// Removed in https://github[x].com/windmill-labs/windmill/pull/1171
			// In case of a bug, try stopping propagation on the native event
			// and dispatch a custom event: `e?.stopPropagation(); dispatch('select');`
			// if ($mode === 'preview') {
			// 	e?.stopPropagation()
			// }
		}}
		class={twMerge(
			'h-full bg-white/40',
			selected && $mode !== 'preview' ? 'border border-blue-500' : '',
			!selected && $mode !== 'preview' && !component.card ? 'border-gray-100' : '',
			$mode !== 'preview' && !$connectingInput.opened ? 'hover:border-blue-500' : '',
			component.softWrap || hasError ? '' : 'overflow-auto',
			$mode != 'preview' ? 'cursor-pointer' : '',
			'relative z-auto',
			$app.css?.['app']?.['component']?.class
		)}
		style={$app.css?.['app']?.['component']?.style}
		bind:clientHeight={componentContainerHeight}
	>
		{#if component.type === 'displaycomponent'}
			<AppDisplayComponent
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'barchartcomponent'}
			<AppBarChart
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'timeseriescomponent'}
			<AppTimeseries
				id={component.id}
				customCss={component.customCss}
				configuration={component.configuration}
				bind:initializing
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'htmlcomponent'}
			<AppHtml
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'vegalitecomponent'}
			<VegaLiteHtml
				configuration={component.configuration}
				id={component.id}
				bind:initializing
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'plotlycomponent'}
			<PlotlyHtml
				id={component.id}
				bind:initializing
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'scatterchartcomponent'}
			<AppScatterChart
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'piechartcomponent'}
			<AppPieChart
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				bind:staticOutputs={$staticOutputs[component.id]}
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'tablecomponent'}
			<AppTable
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				bind:staticOutputs={$staticOutputs[component.id]}
				componentInput={component.componentInput}
				bind:actionButtons={component.actionButtons}
				{render}
			/>
		{:else if component.type === 'aggridcomponent'}
			<AppAggridTable
				id={component.id}
				configuration={component.configuration}
				bind:initializing
				bind:staticOutputs={$staticOutputs[component.id]}
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'textcomponent'}
			<AppText
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'buttoncomponent'}
			<AppButton
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				recomputeIds={component.recomputeIds}
				{render}
			/>
		{:else if component.type === 'selectcomponent'}
			<AppSelect
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'multiselectcomponent'}
			<AppMultiSelect
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'formcomponent'}
			<AppForm
				id={component.id}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				recomputeIds={component.recomputeIds}
				{render}
			/>
		{:else if component.type === 'formbuttoncomponent'}
			<AppFormButton
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
				recomputeIds={component.recomputeIds}
				{render}
			/>
		{:else if component.type === 'checkboxcomponent'}
			<AppCheckbox
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'textinputcomponent'}
			<AppTextInput
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'passwordinputcomponent'}
			<AppTextInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				inputType="password"
				appCssKey="passwordinputcomponent"
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'dateinputcomponent'}
			<AppDateInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				inputType="date"
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'numberinputcomponent'}
			<AppNumberInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'currencycomponent'}
			<AppCurrencyInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'slidercomponent'}
			<AppSliderInputs
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'horizontaldividercomponent'}
			<AppDivider
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				position="horizontal"
				{render}
			/>
		{:else if component.type === 'verticaldividercomponent'}
			<AppDivider
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				position="vertical"
				{render}
			/>
		{:else if component.type === 'rangecomponent'}
			<AppRangeInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'tabscomponent'}
			<AppTabs
				configuration={component.configuration}
				id={component.id}
				tabs={component.tabs}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{componentContainerHeight}
				{render}
			/>
		{:else if component.type === 'containercomponent'}
			<AppContainer
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{componentContainerHeight}
				{render}
			/>
		{:else if component.type === 'verticalsplitpanescomponent'}
			<AppSplitpanes
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				panes={component.panes}
				bind:staticOutputs={$staticOutputs[component.id]}
				{componentContainerHeight}
				{render}
			/>
		{:else if component.type === 'horizontalsplitpanescomponent'}
			<AppSplitpanes
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				panes={component.panes}
				bind:staticOutputs={$staticOutputs[component.id]}
				{componentContainerHeight}
				horizontal={true}
				{render}
			/>
		{:else if component.type === 'iconcomponent'}
			<AppIcon
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'fileinputcomponent'}
			<AppFileInput
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'imagecomponent'}
			<AppImage
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'drawercomponent'}
			<AppDrawer
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'mapcomponent'}
			<AppMap
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{:else if component.type === 'pdfcomponent'}
			<AppPdf
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:staticOutputs={$staticOutputs[component.id]}
				{render}
			/>
		{/if}
	</div>
</div>
{#if initializing}
	<div
		out:fade|local={{ duration: 200 }}
		class="absolute inset-0 center-center flex-col bg-white text-gray-600 border"
	>
		<Loader2 class="animate-spin" size={16} />
		<span class="text-xs mt-1">Loading</span>
	</div>
{/if}
