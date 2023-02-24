<script lang="ts">
	import { getContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import { Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppEditorContext } from '../../types'
	import ComponentHeader from '../ComponentHeader.svelte'
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
		AppContainer
	} from '../../components'
	import type { AppComponent } from './components'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let pointerdown: boolean = false

	const { staticOutputs, mode, connectingInput, app } =
		getContext<AppEditorContext>('AppEditorContext')
	let hover = false
	let initializing: boolean | undefined = undefined
	let componentContainerHeight: number = 0
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
			// Removed in https://github.com/windmill-labs/windmill/pull/1171
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
			component.softWrap ? '' : 'overflow-auto',
			$mode != 'preview' ? 'cursor-pointer' : '',
			'relative z-auto',
			$app.css?.['app']?.['component']?.class
		)}
		style={$app.css?.['app']?.['component']?.style}
		bind:clientHeight={componentContainerHeight}
	>
		{#if component.type === 'displaycomponent'}
			<AppDisplayComponent
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'barchartcomponent'}
			<AppBarChart
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'timeseriescomponent'}
			<AppTimeseries
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'htmlcomponent'}
			<AppHtml
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'vegalitecomponent'}
			<VegaLiteHtml
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'plotlycomponent'}
			<PlotlyHtml
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'scatterchartcomponent'}
			<AppScatterChart
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'piechartcomponent'}
			<AppPieChart
				{...component}
				bind:initializing
				bind:staticOutputs={$staticOutputs[component.id]}
				bind:componentInput={component.componentInput}
			/>
		{:else if component.type === 'tablecomponent'}
			<AppTable
				{...component}
				bind:initializing
				bind:staticOutputs={$staticOutputs[component.id]}
				bind:componentInput={component.componentInput}
				bind:actionButtons={component.actionButtons}
			/>
		{:else if component.type === 'textcomponent'}
			<AppText
				{...component}
				bind:initializing
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'buttoncomponent'}
			<AppButton
				{...component}
				bind:componentInput={component.componentInput}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'selectcomponent'}
			<AppSelect {...component} bind:staticOutputs={$staticOutputs[component.id]} />
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
			<AppCheckbox {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'textinputcomponent'}
			<AppTextInput {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'passwordinputcomponent'}
			<AppTextInput
				inputType="password"
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'dateinputcomponent'}
			<AppDateInput
				inputType="date"
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
			/>
		{:else if component.type === 'numberinputcomponent'}
			<AppNumberInput {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'currencycomponent'}
			<AppCurrencyInput {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'slidercomponent'}
			<AppSliderInputs {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'horizontaldividercomponent'}
			<AppDivider {...component} position="horizontal" />
		{:else if component.type === 'verticaldividercomponent'}
			<AppDivider {...component} position="vertical" />
		{:else if component.type === 'rangecomponent'}
			<AppRangeInput {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'tabscomponent'}
			<AppTabs
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
				{componentContainerHeight}
			/>
		{:else if component.type === 'containercomponent'}
			<AppContainer
				{...component}
				bind:staticOutputs={$staticOutputs[component.id]}
				{componentContainerHeight}
			/>
		{:else if component.type === 'iconcomponent'}
			<AppIcon {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'fileinputcomponent'}
			<AppFileInput {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{:else if component.type === 'imagecomponent'}
			<AppImage {...component} bind:staticOutputs={$staticOutputs[component.id]} />
		{/if}
	</div>
</div>
{#if initializing}
	<div
		out:fade={{ duration: 200 }}
		class="absolute inset-0 center-center flex-col bg-white text-gray-600 border"
	>
		<Loader2 class="animate-spin" size={16} />
		<span class="text-xs mt-1">Loading</span>
	</div>
{/if}
