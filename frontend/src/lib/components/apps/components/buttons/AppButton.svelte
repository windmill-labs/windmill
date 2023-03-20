<script lang="ts">
	import { Button } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { loadIcon } from '../icon'
	import { twMerge } from 'tailwind-merge'
	import { goto } from '$app/navigation'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components, configurationKeys } from '../../editor/component'

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
	export let initializing: boolean | undefined = true

	export let controls: { left: () => boolean; right: () => boolean | string } | undefined =
		undefined

	const { worldStore, app, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	if (controls) {
		$componentControl[id] = controls
	}

	let runnableComponent: RunnableComponent

	let isLoading: boolean = false
	let ownClick: boolean = false

	let resolvedConfig = initConfig(components['buttoncomponent'].initialData.configuration)

	$: initializing = resolvedConfig?.label == undefined

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

	$: resolvedConfig?.triggerOnAppLoad && runnableComponent?.runComponent()

	$: if (outputs?.loading != undefined) {
		outputs.loading.set(false, true)
	}

	$: outputs?.loading.subscribe({
		id: 'loading-' + id,
		next: (value) => {
			isLoading = value
			if (ownClick && !value) {
				ownClick = false
			}
		}
	})

	$: loading = isLoading && ownClick
	let errors: Record<string, string> = {}
	$: errorsMessage = Object.values(errors)
		.filter((x) => x != '')
		.join('\n')

	async function handleClick(event: CustomEvent) {
		event?.stopPropagation()
		event?.preventDefault()

		if (preclickAction) {
			await preclickAction()
		}

		ownClick = true

		if (!runnableComponent) {
			if (resolvedConfig.goto) {
				if (resolvedConfig.gotoNewTab) {
					window.open(resolvedConfig.goto, '_blank')
				} else {
					goto(resolvedConfig.goto)
				}
			}
		} else {
			await runnableComponent?.runComponent()
		}
	}
</script>

{#each configurationKeys['buttoncomponent'] as key (key)}
	<InputValue {key} {id} input={configuration[key]} bind:value={resolvedConfig[key]} />
{/each}

<RunnableWrapper
	{recomputeIds}
	bind:runnableComponent
	{componentInput}
	{id}
	{extraQueryParams}
	autoRefresh={false}
	goto={resolvedConfig.goto}
	gotoNewTab={resolvedConfig.gotoNewTab}
	setTab={resolvedConfig.setTab}
	{render}
	{outputs}
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
