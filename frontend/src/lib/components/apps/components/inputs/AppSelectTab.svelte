<script lang="ts">
	import { getContext } from 'svelte'

	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { Tab, Tabs } from '$lib/components/common'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'selecttabcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = initConfig(
		components['selecttabcomponent'].initialData.configuration,
		configuration
	)
	const outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	let selected: string = ''
	$: selected === '' && resolvedConfig && setDefaultValue()

	function setDefaultValue() {
		if (resolvedConfig.defaultValue === undefined) {
			selected = resolvedConfig.items[0].value
		} else if (resolvedConfig.defaultValue?.value) {
			selected = resolvedConfig.defaultValue?.value
		}
	}

	function handleSelection(value: string) {
		outputs?.result.set(value)
	}

	function onPointerDown(
		e: PointerEvent & {
			currentTarget: EventTarget & HTMLDivElement
		}
	) {
		if (!e.shiftKey) {
			e.stopPropagation()
		}
	}

	$: selected && handleSelection(selected)
	$: css = concatCustomCss($app.css?.selecttabcomponent, customCss)
</script>

{#each Object.keys(components['selecttabcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {horizontalAlignment} {verticalAlignment}>
	<div class="app-select w-full" style="height: 34px;" on:pointerdown={onPointerDown}>
		<Tabs bind:selected class={css?.tabRow?.class} style={css?.tabRow?.style}>
			{#each resolvedConfig?.items ?? [] as item}
				<Tab
					value={item.value}
					class={css?.allTabs?.class}
					style={css?.allTabs?.style}
					selectedClass={css?.selectedTab?.class}
					selectedStyle={css?.selectedTab?.style}
				>
					<span class="font-semibold text-md">{item.label}</span>
				</Tab>
			{/each}
		</Tabs>
	</div>
</AlignWrapper>
