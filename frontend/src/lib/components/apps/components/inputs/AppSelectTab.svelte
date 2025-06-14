<script lang="ts">
	import { getContext, onDestroy, untrack } from 'svelte'

	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { Tab, Tabs } from '$lib/components/common'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		configuration: RichConfigurations
		horizontalAlignment?: 'left' | 'center' | 'right' | undefined
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		customCss?: ComponentCustomCSS<'selecttabcomponent'> | undefined
		render: boolean
	}

	let {
		id,
		configuration,
		horizontalAlignment = undefined,
		verticalAlignment = undefined,
		customCss = undefined,
		render
	}: Props = $props()

	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = $state(
		initConfig(components['selecttabcomponent'].initialData.configuration, configuration)
	)
	const outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let selected: string = $state('')

	onDestroy(() => {
		listInputs?.remove(id)
	})

	$componentControl[id] = {
		setValue(nvalue: string) {
			selected = nvalue
		},
		setTab(index) {
			selected = resolvedConfig.items?.[index]?.value
		},
		setSelectedIndex(index) {
			if (index >= 0 && index < resolvedConfig.items?.length) {
				selected = resolvedConfig.items?.[index]?.value
			}
		}
	}

	function setDefaultValue() {
		if (resolvedConfig.defaultValue === undefined) {
			selected = resolvedConfig.items?.[0]?.value
		} else if (resolvedConfig.defaultValue) {
			selected = resolvedConfig.defaultValue
		}
	}

	function handleSelection(value: string) {
		outputs?.result.set(value)
		if (iterContext && listInputs) {
			listInputs.set(id, value)
		}
	}

	let css = $state(initCss($app.css?.selecttabcomponent, customCss))
	$effect.pre(() => {
		selected === '' && resolvedConfig.defaultValue && untrack(() => setDefaultValue())
	})
	$effect.pre(() => {
		selected && untrack(() => handleSelection(selected))
	})
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
