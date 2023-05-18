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
	import Multiselect from '$lib/components/Multiselect.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'multiselectcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')
	let items: string[]

	let outputs = initOutput($worldStore, id, {
		result: [] as string[]
	})

	let resolvedConfig = initConfig(
		components['multiselectcomponent'].initialData.configuration,
		configuration
	)
	// $: outputs && handleOutputs()

	// function handleOutputs() {
	// 	value = outputs.result.peak()
	// }

	let value: string[] | undefined = outputs?.result.peak()

	$: resolvedConfig.items && handleItems()

	function handleItems() {
		if (Array.isArray(resolvedConfig.items)) {
			items = resolvedConfig.items?.map((label) => {
				return typeof label === 'string' ? label : `NOT_STRING`
			})
		}
	}

	$: value ? outputs?.result.set(value) : outputs?.result.set([])

	$: css = concatCustomCss($app.css?.multiselectcomponent, customCss)
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
		class="app-select w-full overflow-auto"
		on:pointerdown={(e) => {
			if (!e.shiftKey) {
				e.stopPropagation()
			}
		}}
	>
		{#if !value || Array.isArray(value)}
			<Multiselect bind:value options={items} placeholder={resolvedConfig.placeholder} />
			<!-- <Select
				--border-radius="0"
				--border-color="#999"
				multiple
				clearable={false}
				closeListOnChange={resolvedConfig.closeListOnChanges}
				on:change={(e) => e.stopPropagation()}
				{items}
				inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
				containerStyles={'border-color: #999; overflow: auto;' +
					SELECT_INPUT_DEFAULT_STYLE.containerStyles +
					css?.input?.style}
				bind:value
				{placeholder}
				on:click={() => {
					if (!$connectingInput.opened) {
						$selectedComponent = [id]
					}
				}}
				on:focus={() => {
					if (!$connectingInput.opened) {
						$selectedComponent = [id]
					}
				}}
				floatingConfig={{
					strategy: 'fixed'
				}}
			/> -->
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
