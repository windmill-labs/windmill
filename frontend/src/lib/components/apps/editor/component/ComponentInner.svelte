<script lang="ts">
	import AppMultiSelect from '../../components/inputs/AppMultiSelect.svelte'
	import AppMultiSelectV2 from '../../components/inputs/AppMultiSelectV2.svelte'
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
	import AppAccordionList from '../../components/display/AppAccordionList.svelte'
	import AppAggridTableEe from '../../components/display/table/AppAggridTableEe.svelte'
	import AppCustomComponent from '../../components/display/AppCustomComponent.svelte'
	import AppStatCard from '../../components/display/AppStatCard.svelte'
	import AppMenu from '../../components/display/AppMenu.svelte'
	import AppDecisionTree from '../../components/layout/AppDecisionTree.svelte'
	import AppAgCharts from '../../components/display/charts/AppAgCharts.svelte'
	import AppDbExplorer from '../../components/display/dbtable/AppDbExplorer.svelte'
	import AppS3FileInput from '../../components/inputs/AppS3FileInput.svelte'
	import AppAlert from '../../components/display/AppAlert.svelte'
	import AppDateSliderInput from '../../components/inputs/AppDateSliderInput.svelte'
	import AppTimeInput from '../../components/inputs/AppTimeInput.svelte'
	import AppDateTimeInput from '../../components/inputs/AppDateTimeInput.svelte'
	import AppAggridInfiniteTable from '../../components/display/table/AppAggridInfiniteTable.svelte'
	import AppAggridInfiniteTableEe from '../../components/display/table/AppAggridInfiniteTableEe.svelte'
	import AppDisplayComponent from '../../components/display/AppDisplayComponent.svelte'
	import AppTimeseries from '../../components/display/AppTimeseries.svelte'
	import AppHtml from '../../components/display/AppHtml.svelte'
	import AppMarkdown from '../../components/display/AppMarkdown.svelte'
	import VegaLiteHtml from '../../components/display/VegaLiteHtml.svelte'
	import PlotlyHtml from '../../components/display/PlotlyHtml.svelte'
	import PlotlyHtmlV2 from '../../components/display/PlotlyHtmlV2.svelte'
	import AppScatterChart from '../../components/display/AppScatterChart.svelte'
	import AppPieChart from '../../components/display/AppPieChart.svelte'
	import AppAggridTable from '../../components/display/table/AppAggridTable.svelte'
	import AppText from '../../components/display/AppText.svelte'
	import AppCodeInputComponent from '../../components/inputs/AppCodeInputComponent.svelte'
	import AppButton from '../../components/buttons/AppButton.svelte'
	import AppForm from '../../components/buttons/AppForm.svelte'
	import AppFormButton from '../../components/buttons/AppFormButton.svelte'
	import AppCheckbox from '../../components/inputs/AppCheckbox.svelte'
	import AppTextInput from '../../components/inputs/AppTextInput.svelte'
	import AppDateInput from '../../components/inputs/AppDateInput.svelte'
	import AppSelect from '../../components/inputs/AppSelect.svelte'
	import AppBarChart from '../../components/display/AppBarChart.svelte'
	import AppDivider from '../../components/layout/AppDivider.svelte'
	import AppRangeInput from '../../components/inputs/AppRangeInput.svelte'
	import AppTabs from '../../components/layout/AppTabs.svelte'
	import AppContainer from '../../components/layout/AppContainer.svelte'
	import AppSplitpanes from '../../components/layout/AppSplitpanes.svelte'
	import AppIcon from '../../components/display/AppIcon.svelte'
	import AppFileInput from '../../components/inputs/AppFileInput.svelte'
	import AppImage from '../../components/display/AppImage.svelte'
	import AppDrawer from '../../components/layout/AppDrawer.svelte'
	import AppMap from '../../components/display/AppMap.svelte'
	import AppPdf from '../../components/display/AppPdf.svelte'
	import AppCurrencyInput from '../../components/inputs/currency/AppCurrencyInput.svelte'
	import AppSliderInputs from '../../components/inputs/AppSliderInputs.svelte'
	import AppNumberInput from '../../components/inputs/AppNumberInput.svelte'
	import AppNavbar from '../../components/display/AppNavbar.svelte'
	import AppDateSelect from '../../components/inputs/AppDateSelect.svelte'
	import AppDisplayComponentByJobId from '../../components/display/AppDisplayComponentByJobId.svelte'
	import AppRecomputeAll from '../../components/display/AppRecomputeAll.svelte'
	import AppUserResource from '../../components/inputs/AppUserResource.svelte'
	import type { AppComponent } from './components'
	import { Button } from '$lib/components/common'
	import { Loader2 } from 'lucide-svelte'

	interface Props {
		component: AppComponent
		render: boolean
		componentContainerHeight: number
		errorHandledByComponent: boolean
		inlineEditorOpened: boolean
		initializing?: boolean | undefined
	}

	let {
		component,
		render,
		componentContainerHeight,
		errorHandledByComponent = $bindable(),
		inlineEditorOpened = $bindable(),
		initializing = $bindable(undefined)
	}: Props = $props()

	// Define the component groups for efficient range checking
	const chunk1Components = new Set([
		'accordionlistcomponent',
		'agchartscomponent',
		'agchartscomponentee',
		'aggridcomponent',
		'aggridcomponentee',
		'aggridinfinitecomponent',
		'aggridinfinitecomponentee',
		'alertcomponent',
		'barchartcomponent',
		'buttoncomponent'
	])
	const chunk2Components = new Set([
		'carousellistcomponent',
		'chartjscomponent',
		'chartjscomponentv2',
		'checkboxcomponent',
		'codeinputcomponent',
		'conditionalwrapper',
		'containercomponent',
		'currencycomponent',
		'customcomponent',
		'dateinputcomponent'
	])
	const chunk3Components = new Set([
		'dateselectcomponent',
		'dateslidercomponent',
		'datetimeinputcomponent',
		'dbexplorercomponent',
		'decisiontreecomponent',
		'displaycomponent',
		'downloadcomponent',
		'drawercomponent',
		'emailinputcomponent',
		'fileinputcomponent'
	])
	const chunk4Components = new Set([
		'flowstatuscomponent',
		'formbuttoncomponent',
		'formcomponent',
		'horizontaldividercomponent',
		'horizontalsplitpanescomponent',
		'htmlcomponent',
		'iconcomponent',
		'imagecomponent',
		'jobiddisplaycomponent',
		'jobidflowstatuscomponent'
	])
	const chunk5Components = new Set([
		'jobidlogcomponent',
		'listcomponent',
		'logcomponent',
		'mapcomponent',
		'mardowncomponent',
		'menucomponent',
		'modalcomponent',
		'multiselectcomponent',
		'multiselectcomponentv2',
		'navbarcomponent'
	])
	const chunk6Components = new Set([
		'numberinputcomponent',
		'passwordinputcomponent',
		'pdfcomponent',
		'piechartcomponent',
		'plotlycomponent',
		'plotlycomponentv2',
		'quillcomponent',
		'rangecomponent',
		'recomputeallcomponent',
		'resourceselectcomponent'
	])
	const chunk7Components = new Set([
		's3fileinputcomponent',
		'scatterchartcomponent',
		'schemaformcomponent',
		'selectcomponent',
		'selectstepcomponent',
		'selecttabcomponent',
		'slidercomponent',
		'statcomponent',
		'steppercomponent'
	])
	const chunk8Components = new Set([
		'tablecomponent',
		'tabscomponent',
		'textareainputcomponent',
		'textcomponent',
		'textinputcomponent',
		'timeinputcomponent',
		'timeseriescomponent',
		'userresourcecomponent',
		'vegalitecomponent',
		'verticaldividercomponent'
	])
	const chunk9Components = new Set(['verticalsplitpanescomponent'])
</script>

<svelte:boundary
	onerror={(e) => {
		console.error('Error displaying component ' + component.id, component, e)
	}}
>
	<!-- Chunk 1: accordionlistcomponent - buttoncomponent -->
	{#if chunk1Components.has(component.type)}
		{#if component.type === 'accordionlistcomponent'}
			<AppAccordionList
				id={component.id}
				componentInput={component.componentInput}
				customCss={component.customCss}
				{componentContainerHeight}
				{render}
				bind:initializing
			/>
		{:else if component.type === 'agchartscomponent'}
			<AppAgCharts
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'agchartscomponentee'}
			<AppAgCharts
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				license={component.license}
				ee={true}
				{render}
			/>
		{:else if component.type === 'aggridcomponent'}
			<AppAggridTable
				id={component.id}
				configuration={component.configuration}
				bind:initializing
				componentInput={component.componentInput}
				customCss={component.customCss}
				actions={component.actions ?? []}
				actionsOrder={component.actionsOrder ?? undefined}
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
				actions={component.actions ?? []}
				actionsOrder={component.actionsOrder ?? undefined}
				{render}
			/>
		{:else if component.type === 'aggridinfinitecomponent'}
			<AppAggridInfiniteTable
				id={component.id}
				configuration={component.configuration}
				bind:initializing
				componentInput={component.componentInput}
				customCss={component.customCss}
				actions={component.actions ?? []}
				{render}
			/>
		{:else if component.type === 'aggridinfinitecomponentee'}
			<AppAggridInfiniteTableEe
				license={component.license}
				id={component.id}
				configuration={component.configuration}
				bind:initializing
				componentInput={component.componentInput}
				customCss={component.customCss}
				actions={component.actions ?? []}
				{render}
			/>
		{:else if component.type === 'alertcomponent'}
			<AppAlert
				id={component.id}
				configuration={component.configuration}
				customCss={component.customCss}
				verticalAlignment={component.verticalAlignment}
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
		{/if}
	{/if}

	<!-- Chunk 2: carousellistcomponent - dateinputcomponent -->
	{#if chunk2Components.has(component.type)}
		{#if component.type === 'carousellistcomponent'}
			<AppCarouselList
				id={component.id}
				configuration={component.configuration}
				componentInput={component.componentInput}
				customCss={component.customCss}
				{componentContainerHeight}
				{render}
				bind:initializing
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
		{:else if component.type === 'checkboxcomponent'}
			<AppCheckbox
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				recomputeIds={component.recomputeIds}
				onToggle={component.onToggle}
				{render}
			/>
		{:else if component.type === 'codeinputcomponent'}
			<AppCodeInputComponent id={component.id} configuration={component.configuration} {render} />
		{:else if component.type === 'conditionalwrapper' && component.conditions}
			<AppConditionalWrapper
				id={component.id}
				conditions={component.conditions}
				customCss={component.customCss}
				onTabChange={component.onTabChange}
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
		{:else if component.type === 'currencycomponent'}
			<AppCurrencyInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'customcomponent'}
			<AppCustomComponent
				customComponent={component.customComponent}
				id={component.id}
				componentInput={component.componentInput}
				{render}
			/>
		{:else if component.type === 'dateinputcomponent'}
			<AppDateInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				inputType="date"
				id={component.id}
				customCss={component.customCss}
				onChange={component.onChange}
				{render}
			/>
		{/if}
	{/if}

	<!-- Chunk 3: dateselectcomponent - fileinputcomponent -->
	{#if chunk3Components.has(component.type)}
		{#if component.type === 'dateselectcomponent'}
			<AppDateSelect
				id={component.id}
				configuration={component.configuration}
				customCss={component.customCss}
				verticalAlignment={component.verticalAlignment}
				{render}
			/>
		{:else if component.type === 'dateslidercomponent'}
			<AppDateSliderInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'datetimeinputcomponent'}
			<AppDateTimeInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				inputType="date"
				id={component.id}
				customCss={component.customCss}
				onChange={component.onChange}
				{render}
			/>
		{:else if component.type === 'dbexplorercomponent'}
			<AppDbExplorer
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				actions={component.actions ?? []}
				bind:initializing
				{render}
			/>
		{:else if component.type === 'decisiontreecomponent' && component.nodes}
			<AppDecisionTree
				id={component.id}
				nodes={component.nodes}
				customCss={component.customCss}
				{componentContainerHeight}
				{render}
			/>
		{:else if component.type === 'displaycomponent'}
			<AppDisplayComponent
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				configuration={component.configuration}
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
		{:else if component.type === 'drawercomponent'}
			<AppDrawer
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				onOpenRecomputeIds={component.onOpenRecomputeIds}
				onCloseRecomputeIds={component.onCloseRecomputeIds}
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
				onChange={component.onChange}
				{render}
			/>
		{:else if component.type === 'fileinputcomponent'}
			<AppFileInput
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				onFileChange={component.onFileChange}
				{render}
			/>
		{/if}
	{/if}

	<!-- Chunk 4: flowstatuscomponent - jobidflowstatuscomponent -->
	{#if chunk4Components.has(component.type)}
		{#if component.type === 'flowstatuscomponent'}
			<AppFlowStatusComponent />
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
		{:else if component.type === 'horizontalsplitpanescomponent'}
			<AppSplitpanes
				id={component.id}
				customCss={component.customCss}
				panes={component.panes}
				{componentContainerHeight}
				horizontal={true}
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
		{:else if component.type === 'iconcomponent'}
			<AppIcon
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
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
		{:else if component.type === 'jobiddisplaycomponent'}
			<AppDisplayComponentByJobId
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				configuration={component.configuration}
				{render}
			/>
		{:else if component.type === 'jobidflowstatuscomponent'}
			<AppJobIdFlowStatus
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				configuration={component.configuration}
				{render}
			/>
		{/if}
	{/if}

	<!-- Chunk 5: jobidlogcomponent - navbarcomponent -->
	{#if chunk5Components.has(component.type)}
		{#if component.type === 'jobidlogcomponent'}
			<AppJobIdLogComponent
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				configuration={component.configuration}
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
		{:else if component.type === 'logcomponent'}
			<AppLogsComponent />
		{:else if component.type === 'mapcomponent'}
			<AppMap
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
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
		{:else if component.type === 'modalcomponent'}
			<AppModal
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				onOpenRecomputeIds={component.onOpenRecomputeIds}
				onCloseRecomputeIds={component.onCloseRecomputeIds}
				{render}
			/>
		{:else if component.type === 'multiselectcomponent'}
			<AppMultiSelect id={component.id} verticalAlignment={component.verticalAlignment} {render} />
		{:else if component.type === 'multiselectcomponentv2'}
			<AppMultiSelectV2
				id={component.id}
				configuration={component.configuration}
				customCss={component.customCss}
				verticalAlignment={component.verticalAlignment}
				{render}
			/>
		{:else if component.type === 'navbarcomponent'}
			<AppNavbar
				id={component.id}
				configuration={component.configuration}
				customCss={component.customCss}
				navbarItems={component.navbarItems}
				{render}
			/>
		{/if}
	{/if}

	<!-- Chunk 6: numberinputcomponent - resourceselectcomponent -->
	{#if chunk6Components.has(component.type)}
		{#if component.type === 'numberinputcomponent'}
			<AppNumberInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				onChange={component.onChange}
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
				onChange={component.onChange}
				{render}
			/>
		{:else if component.type === 'pdfcomponent'}
			<AppPdf
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
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
		{:else if component.type === 'quillcomponent'}
			<AppQuillEditor id={component.id} configuration={component.configuration} {render} />
		{:else if component.type === 'rangecomponent'}
			<AppRangeInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'recomputeallcomponent'}
			<AppRecomputeAll
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				configuration={component.configuration}
				horizontalAlignment={component.horizontalAlignment}
				{render}
			/>
		{:else if component.type === 'resourceselectcomponent'}
			<AppSelect
				recomputeIds={component.recomputeIds}
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				onSelect={component.onSelect}
				{render}
			/>
		{/if}
	{/if}

	<!-- Chunk 7: s3fileinputcomponent - steppercomponent -->
	{#if chunk7Components.has(component.type)}
		{#if component.type === 's3fileinputcomponent'}
			<AppS3FileInput
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				onFileChange={component.onFileChange}
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
		{:else if component.type === 'schemaformcomponent'}
			<AppSchemaForm
				id={component.id}
				componentInput={component.componentInput}
				configuration={component.configuration}
				customCss={component.customCss}
				{initializing}
				{render}
			/>
		{:else if component.type === 'selectcomponent'}
			<AppSelect
				recomputeIds={component.recomputeIds}
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				onSelect={component.onSelect}
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
		{:else if component.type === 'selecttabcomponent'}
			<AppSelectTab
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				horizontalAlignment={component.horizontalAlignment}
				configuration={component.configuration}
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
		{:else if component.type === 'statcomponent'}
			<AppStatCard
				id={component.id}
				configuration={component.configuration}
				customCss={component.customCss}
				{render}
			/>
		{:else if component.type === 'steppercomponent' && component.tabs}
			<AppStepper
				id={component.id}
				tabs={component.tabs}
				customCss={component.customCss}
				{componentContainerHeight}
				componentInput={component.componentInput}
				onNext={component.onNext}
				onPrevious={component.onPrevious}
				{render}
			/>
		{/if}
	{/if}

	<!-- Chunk 8: tablecomponent - verticaldividercomponent -->
	{#if chunk8Components.has(component.type)}
		{#if component.type === 'tablecomponent'}
			{#await import('$lib/components/apps/components/display/table/AppTable.svelte')}
				<Loader2 />
			{:then Module}
				<Module.default
					configuration={component.configuration}
					id={component.id}
					customCss={component.customCss}
					bind:initializing
					componentInput={component.componentInput}
					actionButtons={component.actionButtons}
					{render}
				/>
			{/await}
		{:else if component.type === 'tabscomponent' && component.tabs}
			<AppTabs
				configuration={component.configuration}
				id={component.id}
				tabs={component.tabs}
				disabledTabs={component.disabledTabs}
				onTabChange={component.onTabChange}
				customCss={component.customCss}
				{componentContainerHeight}
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
				onChange={component.onChange}
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
		{:else if component.type === 'textinputcomponent'}
			<AppTextInput
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
				onChange={component.onChange}
				{render}
			/>
		{:else if component.type === 'timeinputcomponent'}
			<AppTimeInput
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				onChange={component.onChange}
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
		{:else if component.type === 'userresourcecomponent'}
			<AppUserResource
				id={component.id}
				verticalAlignment={component.verticalAlignment}
				configuration={component.configuration}
				customCss={component.customCss}
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
		{/if}
	{/if}

	<!-- Chunk 9: verticalsplitpanescomponent -->
	{#if chunk9Components.has(component.type)}
		{#if component.type === 'verticalsplitpanescomponent'}
			<AppSplitpanes
				id={component.id}
				customCss={component.customCss}
				panes={component.panes}
				{componentContainerHeight}
				{render}
			/>
		{/if}
	{/if}

	{#snippet failed(error, reset)}
		<div class="flex flex-col items-center justify-center bg-red-100 p-10 h-full w-full">
			<h3 class="text-red-500 text-2xl font-bold">Rendering of component failed</h3>
			<pre
				class="text-2xs mt-4 w-full font-mono border border-red-500 rounded-md p-2 bg-surface-secondary text-primary overflow-auto"
				>{error}</pre
			>
			<div class="flex mt-4">
				<Button wrapperClasses="border rounded !border-gray-400" color="dark" on:click={reset}
					>reset</Button
				>
			</div>
		</div>
	{/snippet}
</svelte:boundary>
