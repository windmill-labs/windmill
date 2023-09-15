<script lang="ts">
	import { getContext } from 'svelte'

	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { Tab, Tabs } from '$lib/components/common'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'selecttabcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = initConfig(
		components['selecttabcomponent'].initialData.configuration,
		configuration
	)
	const outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	let selected: string = ''
	$: selected === '' && resolvedConfig && setDefaultValue()

	$componentControl[id] = {
		setValue(nvalue: string) {
			selected = nvalue
		},
		setTab(index) {
			selected = resolvedConfig.items?.[index]?.value
		}
	}

	function setDefaultValue() {
		if (resolvedConfig.defaultValue === undefined) {
			selected = resolvedConfig.items[0].value
		} else if (resolvedConfig.defaultValue) {
			selected = resolvedConfig.defaultValue
		}
	}

	function handleSelection(value: string) {
		outputs?.result.set(value)
	}

	$: selected && handleSelection(selected)
	let css = initCss($app.css?.selecttabcomponent, customCss)
</script>

{#each Object.keys(components['selecttabcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.selecttabcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {horizontalAlignment} {verticalAlignment}>
	<div class="w-full">
		<Tabs
			bind:selected
			class={twMerge(css?.tabRow?.class, 'wm-select-tab-row')}
			style={css?.tabRow?.style}
		>
			{#each resolvedConfig?.items ?? [] as item}
				<Tab
					value={item.value}
					class={twMerge(css?.allTabs?.class, 'wm-select-tab')}
					style={css?.allTabs?.style}
					selectedClass={twMerge(css?.selectedTab?.class, 'wm-select-tab-selected')}
					selectedStyle={css?.selectedTab?.style}
					size={resolvedConfig?.tabSize}
				>
					<span class="font-semibold text-md">{item.label}</span>
				</Tab>
			{/each}
		</Tabs>
	</div>
</AlignWrapper>
