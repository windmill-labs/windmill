<script lang="ts">
	import { Button } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { loadIcon } from '../icon'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'

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
	export let initializing: boolean | undefined = false
	export let extraKey: string | undefined = undefined

	export let controls: { left: () => boolean; right: () => boolean | string } | undefined =
		undefined

	const { worldStore, app, componentControl, selectedComponent } =
		getContext<AppViewerContext>('AppViewerContext')
	let resolvedConfig = initConfig(
		components['buttoncomponent'].initialData.configuration,
		configuration
	)

	$: initializing = resolvedConfig?.label == undefined

	let outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

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

		if (preclickAction) {
			await preclickAction()
		}

		if (!runnableComponent) {
			runnableWrapper.onSuccess()
		} else {
			await runnableComponent?.runComponent()
		}
	}
	let loading = false
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
	{id}
	{extraQueryParams}
	autoRefresh={false}
	{render}
	{outputs}
	{extraKey}
	refreshOnStart={resolvedConfig.triggerOnAppLoad}
>
	<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment}>
		{#if errorsMessage}
			<div class="text-red-500 text-xs">{errorsMessage}</div>
		{/if}
		<Button
			on:pointerdown={(e) => e.stopPropagation()}
			btnClasses={twMerge(
				$app.css?.['buttoncomponent']?.['button']?.class,
				customCss?.button?.class,
				resolvedConfig.fillContainer ? 'w-full h-full' : ''
			)}
			wrapperClasses={resolvedConfig.fillContainer ? 'w-full h-full' : ''}
			style={[$app.css?.['buttoncomponent']?.['button']?.style, customCss?.button?.style].join(';')}
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
				<div>{resolvedConfig.label}</div>
				{#if resolvedConfig.afterIcon && afterIconComponent}
					<svelte:component this={afterIconComponent} size={14} />
				{/if}
			</span>
		</Button>
	</AlignWrapper>
</RunnableWrapper>
