<script lang="ts">
	import { getContext, onDestroy, untrack } from 'svelte'
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
	import { twMerge } from 'tailwind-merge'
	import LightweightResourcePicker from '$lib/components/LightweightResourcePicker.svelte'

	interface Props {
		id: string
		configuration: RichConfigurations
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		customCss?: ComponentCustomCSS<'userresourcecomponent'> | undefined
		render: boolean
	}

	let {
		id,
		configuration,
		verticalAlignment = undefined,
		customCss = undefined,
		render
	}: Props = $props()

	const { app, worldStore, componentControl, isEditor, mode } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = $state(
		initConfig(components['userresourcecomponent'].initialData.configuration, configuration)
	)

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	let css = $state(initCss($app.css?.['userresourcecomponent'], customCss))

	let classInput = $derived(
		twMerge(
			'windmillapp w-full px-2',
			css?.input?.class ?? '',
			'wm-input',
			'wm-user-resource-select'
		)
	)

	function getDefaultValue(): string | undefined {
		if (resolvedConfig.defaultValue && typeof resolvedConfig.defaultValue === 'string') {
			const nval = resolvedConfig.defaultValue as string
			return nval.replace('$res:', '')
		}
		return undefined
	}

	let value: string | undefined = $state(
		outputs.result.peak()?.replace('$res:', '') ?? getDefaultValue()
	)

	$effect(() => {
		value
		untrack(() => assignValue(value))
	})

	let lastDefaultValue = $state(getDefaultValue())
	$effect(() => {
		// when in dnd mode, we react to the default value being changed for better UX
		if (isEditor && $mode === 'dnd') {
			const currentDefaultValue = getDefaultValue() // reactive
			if (lastDefaultValue !== currentDefaultValue) {
				value = currentDefaultValue
				lastDefaultValue = currentDefaultValue
			}
		}
	})

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

	function assignValue(value: string | undefined) {
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

	let resourcePicker: LightweightResourcePicker | undefined = $state(undefined)
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
		<div
			class="relative w-full {classInput}',
"
		>
			<LightweightResourcePicker
				expressOAuthSetup={resolvedConfig.expressOauthSetup}
				bind:this={resourcePicker}
				bind:value
				disabled={resolvedConfig.disabled}
				resourceType={resolvedConfig.resourceType}
			/>
		</div>
	</AlignWrapper>
{/if}
