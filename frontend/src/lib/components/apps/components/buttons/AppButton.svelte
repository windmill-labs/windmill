<script lang="ts">
	import { Button } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { loadIcon } from '../icon'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { concatCustomCss } from '../../utils'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let recomputeIds: string[] | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let noWFull = false
	export let preclickAction: (() => Promise<void>) | undefined = undefined
	export let customCss: ComponentCustomCSS<'buttoncomponent'> | undefined = undefined
	export let render: boolean
	export let errorHandledByComponent: boolean | undefined = false
	export let extraKey: string | undefined = undefined

	export let controls: { left: () => boolean; right: () => boolean | string } | undefined =
		undefined

	const { worldStore, app, componentControl, selectedComponent } =
		getContext<AppViewerContext>('AppViewerContext')
	const rowContext = getContext<ListContext>('RowWrapperContext')
	const rowInputs: ListInputs | undefined = getContext<ListInputs>('RowInputs')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let resolvedConfig = initConfig(
		components['buttoncomponent'].initialData.configuration,
		configuration
	)

	$: errorHandledByComponent = resolvedConfig?.onError?.selected !== 'errorOverlay'

	let outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	if (rowContext && rowInputs) {
		const inputOutput = { result: outputs.result.peak(), loading: false }
		rowInputs(id, inputOutput)
	}

	if (iterContext && listInputs) {
		const inputOutput = { result: outputs.result.peak(), loading: false }
		listInputs(id, inputOutput)
	}

	if (controls) {
		$componentControl[id] = controls
	}

	let runnableComponent: RunnableComponent

	let beforeIconComponent: any
	let afterIconComponent: any

	$: resolvedConfig.beforeIcon && handleBeforeIcon()
	$: resolvedConfig.afterIcon && handleAfterIcon()

	async function handleBeforeIcon() {
		if (resolvedConfig.beforeIcon) {
			beforeIconComponent = await loadIcon(resolvedConfig.beforeIcon)
		}
	}

	async function handleAfterIcon() {
		if (resolvedConfig.afterIcon) {
			afterIconComponent = await loadIcon(resolvedConfig.afterIcon)
		}
	}

	let errors: Record<string, string> = {}
	$: errorsMessage = Object.values(errors)
		.filter((x) => x != '')
		.join('\n')
	let runnableWrapper: RunnableWrapper

	async function handleClick(event: CustomEvent) {
		event?.stopPropagation()
		event?.preventDefault()

		$selectedComponent = [id]
		const inputOutput = { result: outputs.result.peak(), loading: true }
		if (rowContext && rowInputs) {
			rowInputs(id, inputOutput)
		}
		if (iterContext && listInputs) {
			listInputs(id, inputOutput)
		}
		if (preclickAction) {
			await preclickAction()
		}

		if (!runnableComponent) {
			runnableWrapper?.handleSideEffect(true)
		} else {
			await runnableComponent?.runComponent()
		}
		if (rowContext && rowInputs) {
			rowInputs(id, { result: outputs.result.peak(), loading: false })
		}
	}
	let loading = false

	$: css = concatCustomCss($app.css?.buttoncomponent, customCss)
</script>

{#each Object.keys(components['buttoncomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<!-- gotoNewTab={resolvedConfig.onSuccess.selected == 'goto'} -->
<RunnableWrapper
	bind:this={runnableWrapper}
	{recomputeIds}
	bind:runnableComponent
	bind:loading
	{componentInput}
	doOnSuccess={resolvedConfig.onSuccess}
	doOnError={resolvedConfig.onError}
	{errorHandledByComponent}
	{id}
	{extraQueryParams}
	autoRefresh={false}
	{render}
	{outputs}
	{extraKey}
	refreshOnStart={resolvedConfig.triggerOnAppLoad}
>
	<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment} class="wm-button-wrapper">
		{#if errorsMessage}
			<div class="text-red-500 text-xs">{errorsMessage}</div>
		{/if}
		<Button
			on:pointerdown={(e) => e.stopPropagation()}
			btnClasses={twMerge(css?.button?.class, 'wm-button')}
			wrapperClasses={twMerge(
				css?.container?.class,
				resolvedConfig.fillContainer ? 'w-full h-full' : '',
				'wm-button-container'
			)}
			wrapperStyle={css?.container?.style}
			style={css?.button?.style}
			disabled={resolvedConfig.disabled}
			on:click={handleClick}
			size={resolvedConfig.size}
			color={resolvedConfig.color}
			{loading}
		>
			<span class="truncate inline-flex gap-2 items-center">
				{#if resolvedConfig.beforeIcon && beforeIconComponent}
					<svelte:component this={beforeIconComponent} size={14} />
				{/if}
				{#if resolvedConfig.label?.toString() && resolvedConfig.label?.toString()?.length > 0}
					<div>{resolvedConfig.label.toString()}</div>
				{/if}
				{#if resolvedConfig.afterIcon && afterIconComponent}
					<svelte:component this={afterIconComponent} size={14} />
				{/if}
			</span>
		</Button>
	</AlignWrapper>
</RunnableWrapper>
