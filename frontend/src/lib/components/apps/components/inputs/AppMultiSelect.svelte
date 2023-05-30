<script lang="ts">
	import { getContext } from 'svelte'
	// import { SELECT_INPUT_DEFAULT_STYLE } from '../../../../defaults'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	// @ts-ignore
	import MultiSelect from 'svelte-multiselect'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'multiselectcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')
	let items: string[]

	const resolvedConfig = initConfig(
		components['multiselectcomponent'].initialData.configuration,
		configuration
	)

	const outputs = initOutput($worldStore, id, {
		result: [] as string[]
	})

	let value: string[] | undefined = outputs?.result.peak()

	$componentControl[id] = {
		setValue(nvalue: string[]) {
			value = nvalue
			outputs?.result.set([...(value ?? [])])
		}
	}

	$: resolvedConfig.items && handleItems()

	function handleItems() {
		if (Array.isArray(resolvedConfig.items)) {
			items = resolvedConfig.items?.map((label) => {
				return typeof label === 'string' ? label : `NOT_STRING`
			})
		}
	}

	$: resolvedConfig.defaultItems && handleDefaultItems()

	function handleDefaultItems() {
		if (Array.isArray(resolvedConfig.defaultItems)) {
			value = resolvedConfig.defaultItems?.map((label) => {
				return typeof label === 'string' ? label : `NOT_STRING`
			})
			outputs?.result.set([...(value ?? [])])
		}
	}

	$: css = concatCustomCss($app.css?.multiselectcomponent, customCss)

	$: outerDiv && css?.multiselect?.style && outerDiv.setAttribute('style', css?.multiselect?.style)
	let outerDiv: HTMLDivElement | undefined = undefined
</script>

{#each Object.keys(components['multiselectcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} hFull>
	<div
		class="app-select w-full h-full"
		on:pointerdown={(e) => {
			if (!e.shiftKey) {
				e.stopPropagation()
			}
		}}
	>
		{#if !value || Array.isArray(value)}
			<MultiSelect
				bind:outerDiv
				outerDivClass={`${resolvedConfig.allowOverflow ? '' : 'h-full'}`}
				ulSelectedClass={`${resolvedConfig.allowOverflow ? '' : 'overflow-auto max-h-full'} `}
				bind:selected={value}
				on:change={() => {
					outputs?.result.set([...(value ?? [])])
				}}
				options={items}
				placeholder={resolvedConfig.placeholder}
				allowUserOptions={resolvedConfig.create}
				on:open={() => {
					$selectedComponent = [id]
				}}
			/>
		{:else}
			Value {value} is not an array
		{/if}
	</div>
</AlignWrapper>

<style global>
	.app-select .value-container {
		padding: 0 !important;
		overflow: auto;
	}
	.svelte-select-list {
		z-index: 1000 !important;
	}
</style>
