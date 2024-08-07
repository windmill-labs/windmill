<script lang="ts">
	import { getContext, onDestroy } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'userresourcecomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['userresourcecomponent'].initialData.configuration,
		configuration
	)

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	let value: string | undefined = outputs.result.peak()?.replace('$res:', '')

	value && assignValue(outputs.result.peak())

	onDestroy(() => {
		listInputs?.remove(id)
	})

	$componentControl[id] = {
		setValue(nvalue: string) {
			value = nvalue
		},
		askNewResource() {
			resourcePicker?.askNewResource()
		}
	}

	function assignValue(value: string) {
		let nval
		if (!value || value === '') {
			nval = undefined
		} else {
			nval = '$res:' + value.replace('$res:', '')
		}
		outputs?.result.set(nval)
		if (iterContext && listInputs) {
			listInputs.set(id, nval)
		}
	}

	let css = initCss($app.css?.['userresourcecomponent'], customCss)

	let resourcePicker: ResourcePicker | undefined = undefined
</script>

{#each Object.keys(components['userresourcecomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.textinputcomponent}
	/>
{/each}

<InitializeComponent {id} />
{#if render}
	<AlignWrapper {render} {verticalAlignment}>
		<div class="relative w-full">
			<ResourcePicker
				expressOAuthSetup={resolvedConfig.expressOauthSetup}
				bind:this={resourcePicker}
				{value}
				on:change={(e) => {
					assignValue(e.detail)
				}}
				disabled={resolvedConfig.disabled}
				resourceType={resolvedConfig.resourceType}
			/>
		</div>
	</AlignWrapper>
{/if}
