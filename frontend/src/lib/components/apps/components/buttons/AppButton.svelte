<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { loadIcon } from '../icon'
	import { twMerge } from 'tailwind-merge'
	import { goto } from '$app/navigation'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let recomputeIds: string[] | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let noWFull = false
	export let preclickAction: (() => Promise<void>) | undefined = undefined
	export let customCss: ComponentCustomCSS<'button'> | undefined = undefined

	export const staticOutputs: string[] = ['loading', 'result']

	const { runnableComponents, worldStore, app } = getContext<AppEditorContext>('AppEditorContext')

	let labelValue: string
	let color: ButtonType.Color
	let size: ButtonType.Size
	let runnableComponent: RunnableComponent
	let disabled: boolean | undefined = undefined
	let fillContainer: boolean | undefined = undefined
	let gotoUrl: string | undefined = undefined
	let gotoNewTab: boolean | undefined = undefined

	let isLoading: boolean = false
	let ownClick: boolean = false
	let triggerOnAppLoad = false

	let beforeIcon: undefined | string = undefined
	let afterIcon: undefined | string = undefined

	let beforeIconComponent: any
	let afterIconComponent: any

	$: beforeIcon && handleBeforeIcon()
	$: afterIcon && handleAfterIcon()

	async function handleBeforeIcon() {
		if (beforeIcon) {
			beforeIconComponent = await loadIcon(beforeIcon)
		}
	}

	async function handleAfterIcon() {
		if (afterIcon) {
			afterIconComponent = await loadIcon(afterIcon)
		}
	}

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<Array<any>>
		loading: Output<boolean>
	}

	$: triggerOnAppLoad && runnableComponent?.runComponent()

	$: if (outputs?.loading != undefined) {
		outputs.loading.set(false, true)
	}

	$: outputs?.loading.subscribe({
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
			if (gotoUrl) {
				if (gotoNewTab) {
					window.open(gotoUrl, '_blank')
				} else {
					goto(gotoUrl)
				}
			}
		} else {
			await runnableComponent?.runComponent()
		}

		if (recomputeIds) {
			await Promise.all(recomputeIds.map((id) => $runnableComponents?.[id]?.()))
		}
	}
</script>

<InputValue {id} input={configuration.label} bind:value={labelValue} />
<InputValue {id} input={configuration.goto} bind:value={gotoUrl} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue {id} input={configuration.beforeIcon} bind:value={beforeIcon} />
<InputValue {id} input={configuration.afterIcon} bind:value={afterIcon} />
<InputValue {id} input={configuration.triggerOnAppLoad} bind:value={triggerOnAppLoad} />

<InputValue
	{id}
	input={configuration.disabled}
	bind:value={disabled}
	bind:error={errors.disabled}
/>
<InputValue {id} input={configuration.fillContainer} bind:value={fillContainer} />
<InputValue {id} input={configuration.gotoNewTab} bind:value={gotoNewTab} />

<RunnableWrapper
	flexWrap
	bind:runnableComponent
	{componentInput}
	{id}
	{extraQueryParams}
	autoRefresh={false}
	goto={gotoUrl}
	{gotoNewTab}
>
	<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment}>
		{#if errorsMessage}
			<div class="text-red-500 text-xs">{errorsMessage}</div>
		{/if}
		<Button
			btnClasses={twMerge(
				$app.css?.['buttoncomponent']?.['button']?.class,
				customCss?.button.class,
				fillContainer ? 'w-full h-full' : ''
			)}
			style={[$app.css?.['buttoncomponent']?.['button']?.style, customCss?.button.style].join(';')}
			{disabled}
			on:pointerdown={(e) => {
				e?.stopPropagation()
				window.dispatchEvent(new Event('pointerup'))
			}}
			on:click={handleClick}
			{size}
			{color}
			{loading}
		>
			<span class="truncate inline-flex gap-2 items-center">
				{#if beforeIconComponent}
					<svelte:component this={beforeIconComponent} size={14} />
				{/if}
				<div>{labelValue}</div>
				{#if afterIconComponent}
					<svelte:component this={afterIconComponent} size={14} />
				{/if}
			</span>
		</Button>
	</AlignWrapper>
</RunnableWrapper>
