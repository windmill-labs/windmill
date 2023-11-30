<script lang="ts">
	import { Button } from '$lib/components/common'
	import { getContext, onDestroy } from 'svelte'
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
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { initCss } from '../../utils'

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
	export let isMenuItem: boolean = false

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
		rowInputs.set(id, inputOutput)
	}

	if (iterContext && listInputs) {
		const inputOutput = { result: outputs.result.peak(), loading: false }
		listInputs.set(id, inputOutput)
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

	onDestroy(() => {
		listInputs?.remove(id)
		rowInputs?.remove(id)
	})

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
			rowInputs.set(id, inputOutput)
		}
		if (iterContext && listInputs) {
			listInputs.set(id, inputOutput)
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
			rowInputs.set(id, { result: outputs.result.peak(), loading: false })
		}
	}
	let loading = false

	let css = initCss($app.css?.buttoncomponent, customCss)
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

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{extraKey}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.buttoncomponent}
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
		{#key css}
			<Button
				on:pointerdown={(e) => e.stopPropagation()}
				btnClasses={twMerge(
					css?.button?.class ?? '',
					isMenuItem ? 'flex items-center justify-start' : '',
					isMenuItem ? '!border-0' : '',
					'wm-button'
				)}
				variant={isMenuItem ? 'border' : 'contained'}
				style={css?.button?.style}
				wrapperClasses={twMerge(
					css?.container?.class ?? '',
					resolvedConfig.fillContainer ? 'w-full h-full' : '',
					isMenuItem ? 'w-full' : '',
					'wm-button-container'
				)}
				wrapperStyle={css?.container?.style}
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
		{/key}
	</AlignWrapper>
</RunnableWrapper>
