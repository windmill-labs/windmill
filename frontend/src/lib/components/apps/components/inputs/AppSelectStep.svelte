<script lang="ts">
	import { getContext } from 'svelte'

	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import Stepper from '$lib/components/common/stepper/Stepper.svelte'
	import { twMerge } from 'tailwind-merge'
	import { initCss } from '../../utils'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let render: boolean
	export let customCss: ComponentCustomCSS<'selectstepcomponent'> | undefined = undefined

	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = initConfig(
		components['selectstepcomponent'].initialData.configuration,
		configuration
	)
	const outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	let selected: string = ''
	let selectedIndex: number = 0

	$: resolvedConfig.defaultValue != undefined && setDefaultValue()

	$componentControl[id] = {
		setValue(nvalue: string) {
			selected = nvalue
			selectedIndex = resolvedConfig.items.findIndex((item) => getValue(item) === nvalue)
		},
		setTab(index) {
			let item = resolvedConfig.items?.[index]
			selected = getValue(item)
			selectedIndex = index
		}
	}

	function setDefaultValue() {
		if (resolvedConfig.defaultValue != undefined) {
			selectedIndex = resolvedConfig.items.findIndex(
				(item) => getValue(item) === resolvedConfig.defaultValue
			)
		}
		if (selectedIndex === -1 || resolvedConfig.defaultValue == undefined) {
			selected = getValue(resolvedConfig.items[0])
		} else if (resolvedConfig.defaultValue) {
			selected = resolvedConfig.items[selectedIndex].value
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

	function getValue(item: string | { label: string; value: string }) {
		return typeof item == 'string' ? item : item.value
	}
	function getLabel(item: string | { label: string; value: string }) {
		return typeof item == 'string' ? item : item.label
	}

	let css = initCss(app.val.css?.selectstepcomponent, customCss)
	$: selected && handleSelection(selected)
</script>

{#each Object.keys(components['selectstepcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={app.val.css?.selectstepcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper
	{render}
	{horizontalAlignment}
	{verticalAlignment}
	class={twMerge(css?.container?.class, 'wm-select-step')}
	style={css?.container?.style}
>
	<div class="w-full" on:pointerdown={onPointerDown}>
		<Stepper
			tabs={(resolvedConfig?.items ?? []).map((item) => getLabel(item))}
			hasValidations={false}
			allowStepNavigation={true}
			{selectedIndex}
			on:click={(e) => {
				const index = e.detail.index
				selectedIndex = index
				let item = resolvedConfig?.items[index]
				outputs?.result.set(getValue(item))
			}}
		/>
	</div>
</AlignWrapper>
