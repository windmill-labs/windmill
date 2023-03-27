<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppEditorContext, AppViewerContext } from '../../types'
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

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let render: boolean

	const { mode, app, errorByComponent, hoverStore } =
		getContext<AppViewerContext>('AppViewerContext')

	const editorContext = getContext<AppEditorContext>('AppEditorContext')
	const movingcomponents = editorContext?.movingcomponents
	$: ismoving =
		movingcomponents != undefined && $mode == 'dnd' && $movingcomponents?.includes(component.id)

	let initializing: boolean | undefined = undefined
	let componentContainerHeight: number = 0

	$: componentWithErrors = Object.values($errorByComponent).map((e) => e.componentId)
	$: hasError = componentWithErrors.includes(component.id)
</script>

<!-- svelte-ignore a11y-mouse-events-have-key-events -->
<div
	on:mouseover|stopPropagation={() => {
		if (component.id !== $hoverStore) {
			$hoverStore = component.id
		}
	}}
	on:mouseout|stopPropagation={() => {
		if ($hoverStore !== undefined) {
			$hoverStore = undefined
		}
	}}
	class="h-full flex flex-col w-full component {initializing ? 'overflow-hidden h-0' : ''}"
>
	{#if $mode !== 'preview'}
		<ComponentHeader
			hover={$hoverStore === component.id}
			{component}
			{selected}
			on:delete
			on:lock
			on:expand
			{locked}
		/>
	{/if}

	{#if ismoving}
		<div class="absolute -top-8 w-40 ">
			<button
				class="border p-0.5 text-xs"
				on:click={() => {
					$movingcomponents = undefined
				}}
			>
				Cancel move
			</button>
		</div>
	{/if}
	<div
		class={twMerge(
			'h-full bg-white/40 outline-1',
			$hoverStore === component.id && $mode !== 'preview' ? 'outline outline-blue-600' : '',
			selected && $mode !== 'preview' ? 'outline outline-indigo-600' : '',
			$mode != 'preview' ? 'cursor-pointer' : '',
			'relative z-auto',
			$app.css?.['app']?.['component']?.class,
			ismoving ? 'animate-pulse' : ''
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
				{render}
			/>
		{:else if component.type === 'barchartcomponent'}
			<AppBarChart
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'timeseriescomponent'}
			<AppTimeseries
				id={component.id}
				customCss={component.customCss}
				configuration={component.configuration}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'htmlcomponent'}
			<AppHtml
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'vegalitecomponent'}
			<VegaLiteHtml
				configuration={component.configuration}
				id={component.id}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'plotlycomponent'}
			<PlotlyHtml
				id={component.id}
				configuration={component.configuration}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'scatterchartcomponent'}
			<AppScatterChart
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'piechartcomponent'}
			<AppPieChart
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'tablecomponent'}
			<AppTable
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				actionButtons={component.actionButtons}
				{render}
			/>
		{:else if component.type === 'aggridcomponent'}
			<AppAggridTable
				id={component.id}
				configuration={component.configuration}
				bind:initializing
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
				recomputeIds={component.recomputeIds}
				bind:initializing
				{render}
			/>
		{:else if component.type === 'selectcomponent' || component.type === 'resourceselectcomponent'}
			<AppSelect
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'multiselectcomponent'}
			<AppMultiSelect
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'formcomponent'}
			<AppForm
				id={component.id}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				componentInput={component.componentInput}
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
				{render}
			/>
		{:else if component.type === 'textinputcomponent'}
			<AppTextInput
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'textareainputcomponent'}
			<AppTextInput
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				inputType="textarea"
				appCssKey="textareainputcomponent"
				{render}
			/>
		{:else if component.type === 'emailinputcomponent'}
			<AppTextInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				inputType="email"
				appCssKey="emailinputcomponent"
				id={component.id}
				customCss={component.customCss}
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
				{render}
			/>
		{:else if component.type === 'dateinputcomponent'}
			<AppDateInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				inputType="date"
				id={component.id}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'numberinputcomponent'}
			<AppNumberInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'currencycomponent'}
			<AppCurrencyInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'slidercomponent'}
			<AppSliderInputs
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
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
				{render}
				bind:initializing
			/>
		{:else if component.type === 'tabscomponent' && component.tabs}
			<AppTabs
				configuration={component.configuration}
				id={component.id}
				tabs={component.tabs}
				customCss={component.customCss}
				{componentContainerHeight}
				{render}
			/>
		{:else if component.type === 'containercomponent'}
			<AppContainer
				id={component.id}
				customCss={component.customCss}
				{componentContainerHeight}
				{render}
			/>
		{:else if component.type === 'verticalsplitpanescomponent'}
			<AppSplitpanes
				id={component.id}
				customCss={component.customCss}
				panes={component.panes}
				{componentContainerHeight}
				{render}
			/>
		{:else if component.type === 'horizontalsplitpanescomponent'}
			<AppSplitpanes
				id={component.id}
				customCss={component.customCss}
				panes={component.panes}
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
				{render}
			/>
		{:else if component.type === 'fileinputcomponent'}
			<AppFileInput
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'imagecomponent'}
			<AppImage
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
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
				{render}
			/>
		{:else if component.type === 'pdfcomponent'}
			<AppPdf
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				{render}
			/>
		{/if}
	</div>
</div>
{#if initializing}
	<div class="absolute inset-0 center-center flex-col bg- border animate-skeleton" />
{/if}
