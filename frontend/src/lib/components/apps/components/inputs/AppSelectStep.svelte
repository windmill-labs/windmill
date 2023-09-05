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
	import { concatCustomCss } from '../../utils'

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
			selectedIndex = resolvedConfig.items.findIndex((item) => item.value === nvalue)
		},
		setTab(index) {
			selected = resolvedConfig.items?.[index]?.value
			selectedIndex = index
		}
	}

	function setDefaultValue() {
		if (resolvedConfig.defaultValue != undefined) {
			selectedIndex = resolvedConfig.items.findIndex(
				(item) => item.value === resolvedConfig.defaultValue
			)
		}
		if (selectedIndex === -1 || resolvedConfig.defaultValue == undefined) {
			selected = resolvedConfig.items[0].value
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

	$: css = concatCustomCss($app.css?.selectstepcomponent, customCss)
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
			tabs={(resolvedConfig?.items ?? []).map((item) => item.label)}
			hasValidations={false}
			allowStepNavigation={true}
			{selectedIndex}
			on:click={(e) => {
				const index = e.detail.index
				selectedIndex = index
				outputs?.result.set(resolvedConfig?.items[index].value)
			}}
		/>
	</div>
</AlignWrapper>
