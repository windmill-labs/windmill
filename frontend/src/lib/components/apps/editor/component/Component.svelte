<script lang="ts" context="module">
	let outTimeout: NodeJS.Timeout | undefined = undefined
</script>

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
		AppMarkdown,
		AppSliderInputs,
		AppFormButton,
		VegaLiteHtml,
		PlotlyHtml,
		PlotlyHtmlV2,
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
	import AppModal from '../../components/layout/AppModal.svelte'
	import AppSchemaForm from '../../components/buttons/AppSchemaForm.svelte'
	import AppStepper from '../../components/layout/AppStepper.svelte'
	import AppSelectTab from '../../components/inputs/AppSelectTab.svelte'
	import AppConditionalWrapper from '../../components/layout/AppConditionalWrapper.svelte'
	import AppSelectStep from '../../components/inputs/AppSelectStep.svelte'
	import AppDownload from '../../components/display/AppDownload.svelte'
	import AppLogsComponent from '../../components/display/AppLogsComponent.svelte'
	import AppFlowStatusComponent from '../../components/display/AppFlowStatusComponent.svelte'
	import AppChartJs from '../../components/display/AppChartJs.svelte'
	import AppChartJsV2 from '../../components/display/AppChartJsV2.svelte'
	import AppQuillEditor from '../../components/inputs/AppQuillEditor.svelte'
	import AppList from '../../components/layout/AppList.svelte'
	import AppJobIdLogComponent from '../../components/display/AppJobIdLogComponent.svelte'
	import AppJobIdFlowStatus from '../../components/display/AppJobIdFlowStatus.svelte'
	import AppCarouselList from '../../components/display/AppCarouselList.svelte'
	import AppAggridTableEe from '../../components/display/table/AppAggridTableEe.svelte'
	import AppCustomComponent from '../../components/display/AppCustomComponent.svelte'
	import AppStatCard from '../../components/display/AppStatCard.svelte'
	import AppMenu from '../../components/display/AppMenu.svelte'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let render: boolean

	const { mode, app, hoverStore, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	const editorContext = getContext<AppEditorContext>('AppEditorContext')
	const movingcomponents = editorContext?.movingcomponents
	$: ismoving =
		movingcomponents != undefined && $mode == 'dnd' && $movingcomponents?.includes(component.id)

	let initializing: boolean | undefined = undefined
	let errorHandledByComponent: boolean = false
	let componentContainerHeight: number = 0

	let inlineEditorOpened: boolean = false

	function mouseOut() {
		outTimeout && clearTimeout(outTimeout)
		outTimeout = setTimeout(() => {
			if ($hoverStore !== undefined) {
				$hoverStore = undefined
			}
		}, 50)
	}
</script>

<!-- svelte-ignore a11y-mouse-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	on:mouseover|stopPropagation={() => {
		outTimeout && clearTimeout(outTimeout)
		if (component.id !== $hoverStore) {
			$hoverStore = component.id
		}
	}}
	on:mouseout|stopPropagation={mouseOut}
	class="h-full flex flex-col w-full component {initializing ? 'overflow-hidden h-0' : ''}"
>
	{#if $mode !== 'preview'}
		<ComponentHeader
			on:mouseover={() => {
				outTimeout && clearTimeout(outTimeout)

				if (component.id !== $hoverStore) {
					$hoverStore = component.id
				}
			}}
			hover={$hoverStore === component.id}
			{component}
			{selected}
			connecting={$connectingInput.opened}
			on:lock
			on:expand
			{locked}
			{inlineEditorOpened}
			hasInlineEditor={component.type === 'textcomponent' &&
				component.componentInput &&
				component.componentInput.type !== 'connected'}
			on:triggerInlineEditor={() => {
				inlineEditorOpened = !inlineEditorOpened
			}}
			{errorHandledByComponent}
		/>
	{/if}

	{#if ismoving}
		<div class="absolute -top-8 w-40">
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
			'h-full outline-1',
			$mode === 'dnd' ? 'bg-surface/40' : '',
			$hoverStore === component.id && $mode !== 'preview'
				? $connectingInput.opened
					? 'outline outline-orange-600'
					: 'outline outline-blue-600'
				: '',
			selected && $mode !== 'preview' ? 'outline outline-indigo-600' : '',
			$mode != 'preview' ? 'cursor-pointer' : '',
			'relative z-auto',
			$app.css?.['app']?.['component']?.class,
			'wm-app-component',
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
				configuration={component.configuration}
				{render}
			/>
		{:else if component.type === 'logcomponent'}
			<AppLogsComponent
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'jobidlogcomponent'}
			<AppJobIdLogComponent
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				configuration={component.configuration}
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'flowstatuscomponent'}
			<AppFlowStatusComponent
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'jobidflowstatuscomponent'}
			<AppJobIdFlowStatus
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				configuration={component.configuration}
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
		{:else if component.type === 'customcomponent'}
			<AppCustomComponent
				customComponent={component.customComponent}
				id={component.id}
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'mardowncomponent'}
			<AppMarkdown
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				configuration={component.configuration}
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
		{:else if component.type === 'plotlycomponentv2'}
			<PlotlyHtmlV2
				id={component.id}
				configuration={component.configuration}
				bind:initializing
				componentInput={component.componentInput}
				datasets={component.datasets}
				xData={component.xData}
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
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'aggridcomponentee'}
			<AppAggridTableEe
				license={component.license}
				id={component.id}
				configuration={component.configuration}
				bind:initializing
				componentInput={component.componentInput}
				customCss={component.customCss}
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
				bind:editorMode={inlineEditorOpened}
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
				bind:errorHandledByComponent
				{render}
			/>
		{:else if component.type === 'downloadcomponent'}
			<AppDownload
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'selectcomponent' || component.type === 'resourceselectcomponent'}
			<AppSelect
				recomputeIds={component.recomputeIds}
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'multiselectcomponent'}
			<AppMultiSelect
				id={component.id}
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
				bind:errorHandledByComponent
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
				bind:errorHandledByComponent
				{render}
			/>
		{:else if component.type === 'checkboxcomponent'}
			<AppCheckbox
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				recomputeIds={component.recomputeIds}
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
		{:else if component.type === 'quillcomponent'}
			<AppQuillEditor id={component.id} configuration={component.configuration} {render} />
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
			/>
		{:else if component.type === 'tabscomponent' && component.tabs}
			<AppTabs
				configuration={component.configuration}
				id={component.id}
				tabs={component.tabs}
				disabledTabs={component.disabledTabs}
				customCss={component.customCss}
				{componentContainerHeight}
				{render}
			/>
		{:else if component.type === 'steppercomponent' && component.tabs}
			<AppStepper
				id={component.id}
				tabs={component.tabs}
				customCss={component.customCss}
				{componentContainerHeight}
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'conditionalwrapper' && component.conditions}
			<AppConditionalWrapper
				id={component.id}
				conditions={component.conditions}
				customCss={component.customCss}
				{componentContainerHeight}
				{render}
			/>
		{:else if component.type === 'containercomponent'}
			<AppContainer
				groupFields={component.groupFields}
				id={component.id}
				customCss={component.customCss}
				{componentContainerHeight}
				{render}
			/>
		{:else if component.type === 'listcomponent'}
			<AppList
				id={component.id}
				customCss={component.customCss}
				configuration={component.configuration}
				componentInput={component.componentInput}
				{render}
				bind:initializing
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
		{:else if component.type === 'modalcomponent'}
			<AppModal
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'schemaformcomponent'}
			<AppSchemaForm
				id={component.id}
				componentInput={component.componentInput}
				configuration={component.configuration}
				customCss={component.customCss}
				{initializing}
				{render}
			/>
		{:else if component.type === 'selecttabcomponent'}
			<AppSelectTab
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'selectstepcomponent'}
			<AppSelectStep
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'chartjscomponent'}
			<AppChartJs
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'chartjscomponentv2'}
			<AppChartJsV2
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				datasets={component.datasets}
				xData={component.xData}
				{render}
			/>
		{:else if component.type === 'carousellistcomponent'}
			<AppCarouselList
				id={component.id}
				configuration={component.configuration}
				componentInput={component.componentInput}
				customCss={component.customCss}
				{componentContainerHeight}
				{render}
				bind:initializing
			/>
		{:else if component.type === 'statcomponent'}
			<AppStatCard
				id={component.id}
				configuration={component.configuration}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'menucomponent'}
			<AppMenu
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				menuItems={component.menuItems}
				{render}
			/>
		{/if}
	</div>
</div>
{#if initializing}
	<!-- svelte-ignore a11y-mouse-events-have-key-events -->
	<!-- svelte-ignore a11y-no-static-element-interactions -->
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
		class="absolute inset-0 center-center flex-col bg- border animate-skeleton"
	/>
{/if}
