<script lang="ts">
	import { getContext } from 'svelte'
	import ComponentInner from './ComponentInner.svelte'
	import ComponentRendered from './ComponentRendered.svelte'
	import type { AppComponent } from './components'
	import type { AppViewerContext } from '../../types'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let render: boolean
	export let fullHeight: boolean
	export let overlapped: string | undefined = undefined
	export let componentDraggedId: string | undefined = undefined

	const { app } = getContext<AppViewerContext>('AppViewerContext')
	export let moveMode: string | undefined = undefined
	let everRender = render

	$: render && !everRender && (everRender = true)
</script>

<<<<<<< HEAD
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
	class={twMerge(
		'h-full flex flex-col w-full component relative',
		initializing ? 'overflow-hidden h-0' : '',
		hidden && $mode === 'preview' ? 'hidden' : ''
	)}
	data-connection-button
>
	{#if locked && componentActive && $componentActive && moveMode === 'move' && componentDraggedId && componentDraggedId !== component.id && cachedAreOnTheSameSubgrid}
		<div
			class={twMerge('absolute inset-0 bg-locked center-center flex-col z-50', 'bg-locked-hover')}
		>
			<div class="bg-surface p-2 shadow-sm rounded-md flex center-center flex-col gap-2">
				<Anchor size={24} class="text-primary " />
				<div class="text-xs"> Anchored: The component cannot be moved. </div>
			</div>
		</div>
	{:else if moveMode === 'insert' && isContainer(component.type) && componentDraggedId && componentDraggedId !== component.id && cachedComponentDraggedIsNotChild}
		<div
			class={twMerge(
				'absolute inset-0  flex-col rounded-md bg-blue-100 dark:bg-gray-800 bg-opacity-50',
				'outline-dashed outline-offset-2 outline-2 outline-blue-300 dark:outline-blue-700',
				overlapped === component?.id ? 'bg-draggedover dark:bg-draggedover-dark' : ''
			)}
		/>
	{/if}
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
			{fullHeight}
			connecting={$connectingInput.opened}
			on:lock
			on:expand
			on:fillHeight
			{locked}
			{inlineEditorOpened}
			hasInlineEditor={component.type === 'textcomponent' &&
				component.componentInput &&
				component.componentInput.type !== 'connected'}
			on:triggerInlineEditor={() => {
				inlineEditorOpened = !inlineEditorOpened
			}}
			{errorHandledByComponent}
			{componentContainerWidth}
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
					? 'outline outline-[#f8aa4b]'
					: 'outline outline-blue-400'
				: '',
			selected && $mode !== 'preview' ? 'outline outline-blue-600' : '',
			$mode != 'preview' ? 'cursor-pointer' : '',
			'relative z-auto',
			$app.css?.['app']?.['component']?.class,
			'wm-app-component',
			ismoving ? 'animate-pulse' : ''
		)}
		style={$app.css?.['app']?.['component']?.style}
		bind:clientHeight={componentContainerHeight}
		bind:clientWidth={componentContainerWidth}
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
			<AppLogsComponent />
		{:else if component.type === 'jobidlogcomponent'}
			<AppJobIdLogComponent
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				configuration={component.configuration}
				{render}
			/>
		{:else if component.type === 'flowstatuscomponent'}
			<AppFlowStatusComponent />
		{:else if component.type === 'jobidflowstatuscomponent'}
			<AppJobIdFlowStatus
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				configuration={component.configuration}
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
		{:else if component.type === 'agchartscomponent'}
			<AppAgCharts
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				datasets={component.datasets}
				xData={component.xData}
				{render}
			/>
		{:else if component.type === 'agchartscomponentee'}
			<AppAgCharts
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				componentInput={component.componentInput}
				datasets={component.datasets}
				xData={component.xData}
				license={component.license}
				ee={true}
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
		{:else if component.type === 'dbexplorercomponent'}
			<AppDbExplorer
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				actions={component.actions ?? []}
				bind:initializing
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
				onSelect={component.onSelect}
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
		{:else if component.type === 'multiselectcomponent'}
			<AppMultiSelect
				id={component.id}
				configuration={component.configuration}
				customCss={component.customCss}
				verticalAlignment={component.verticalAlignment}
				{render}
			/>
		{:else if component.type === 'multiselectcomponentv2'}
			<AppMultiSelectV2
				id={component.id}
				configuration={component.configuration}
				customCss={component.customCss}
				verticalAlignment={component.verticalAlignment}
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
				onToggle={component.onToggle}
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
		{:else if component.type === 'timeinputcomponent'}
			<AppTimeInput
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
		{:else if component.type === 'dateslidercomponent'}
			<AppDateSliderInput
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
				onTabChange={component.onTabChange}
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
				onNext={component.onNext}
				onPrevious={component.onPrevious}
				{render}
			/>
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
				onFileChange={component.onFileChange}
				{render}
			/>
		{:else if component.type === 's3fileinputcomponent'}
			<AppS3FileInput
				configuration={component.configuration}
				id={component.id}
				customCss={component.customCss}
				onFileChange={component.onFileChange}
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
				onOpenRecomputeIds={component.onOpenRecomputeIds}
				onCloseRecomputeIds={component.onCloseRecomputeIds}
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
				onOpenRecomputeIds={component.onOpenRecomputeIds}
				onCloseRecomputeIds={component.onCloseRecomputeIds}
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
		{:else if component.type === 'accordionlistcomponent'}
			<AppAccordionList
				id={component.id}
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
		{:else if component.type === 'decisiontreecomponent' && component.nodes}
			<AppDecisionTree
				id={component.id}
				nodes={component.nodes}
				customCss={component.customCss}
				{componentContainerHeight}
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
		{:else if component.type === 'navbarcomponent'}
			<AppNavbar
				id={component.id}
				configuration={component.configuration}
				customCss={component.customCss}
				navbarItems={component.navbarItems}
				{render}
			/>
		{:else if component.type === 'dateselectcomponent'}
			<AppDateSelect
				id={component.id}
				configuration={component.configuration}
				customCss={component.customCss}
				verticalAlignment={component.verticalAlignment}
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
		{:else if component.type === 'recomputeallcomponent'}
			<AppRecomputeAll
				id={component.id}
				customCss={component.customCss}
				bind:initializing
				configuration={component.configuration}
				horizontalAlignment={component.horizontalAlignment}
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
		class="absolute inset-0 center-center flex-col [animation-delay:1000ms] dark:bg-frost-900/50 border animate-skeleton"
=======
{#if everRender || $app.eagerRendering}
	<ComponentRendered
		on:expand
		on:lock
		on:fillHeight
		{moveMode}
		{componentDraggedId}
		{overlapped}
		{render}
		{component}
		{selected}
		{locked}
		{fullHeight}
	/>
{:else}
	<ComponentInner
		{component}
		render={false}
		componentContainerHeight={0}
		errorHandledByComponent={false}
		inlineEditorOpened={false}
>>>>>>> main
	/>
{/if}
