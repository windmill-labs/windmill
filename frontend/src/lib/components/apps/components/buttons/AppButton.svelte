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
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Portal from '$lib/components/Portal.svelte'

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
	export let noInitialize = false
	export let replaceCallback: boolean = false
	export let modalId: string | undefined = undefined
	export let onSuccess: (() => void) | undefined = undefined
	export let modalWrapperClass: string | undefined = undefined

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

	if (controls) {
		$componentControl[id] = controls
	}

	let runnableComponent: RunnableComponent

	let confirmedCallback: (() => void) | undefined = undefined

	let beforeIconComponent: any
	let afterIconComponent: any

	$: resolvedConfig.beforeIcon && beforeIconComponent && handleBeforeIcon()
	$: resolvedConfig.afterIcon && afterIconComponent && handleAfterIcon()

	function getIconSize() {
		switch (resolvedConfig.size as 'xs' | 'xs2' | 'xs3' | 'sm' | 'md' | 'lg' | 'xl') {
			case 'xs':
				return 14
			case 'xs2':
				return 12
			case 'xs3':
				return 10
			case 'sm':
				return 16
			case 'md':
				return 20
			case 'lg':
				return 24
			case 'xl':
				return 26
			default:
				return 24
		}
	}

	async function handleBeforeIcon() {
		if (resolvedConfig.beforeIcon) {
			beforeIconComponent = await loadIcon(
				resolvedConfig.beforeIcon,
				beforeIconComponent,
				getIconSize(),
				undefined,
				undefined
			)
		}
	}

	async function handleAfterIcon() {
		if (resolvedConfig.afterIcon) {
			afterIconComponent = await loadIcon(
				resolvedConfig.afterIcon,
				afterIconComponent,
				getIconSize(),
				undefined,
				undefined
			)
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
		const action = async () => {
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
		}

		if (resolvedConfig?.confirmationModal?.selected === 'confirmationModal') {
			confirmedCallback = action
		} else {
			await action()
		}
	}
	let loading = false

	let css = initCss($app.css?.buttoncomponent, customCss)
</script>

{#each Object.entries(components['buttoncomponent'].initialData.configuration) as [key, initialConfig] (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
		{initialConfig}
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
	{noInitialize}
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
	onSuccess={(r) => {
		let inputOutput = { result: r, loading: false }
		if (rowContext && rowInputs) {
			rowInputs.set(id, inputOutput)
		}
		if (iterContext && listInputs) {
			listInputs.set(id, inputOutput)
		}
		onSuccess?.()
	}}
	refreshOnStart={resolvedConfig.triggerOnAppLoad}
	{replaceCallback}
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
					'wm-button',
					`wm-button-${resolvedConfig.color}`
				)}
				variant={isMenuItem ? 'border' : 'contained'}
				style={css?.button?.style}
				wrapperClasses={twMerge(
					css?.container?.class ?? '',
					resolvedConfig.fillContainer ? 'w-full h-full' : '',
					isMenuItem ? 'w-full' : '',
					'wm-button-container',
					`wm-button-container-${resolvedConfig.color}`
				)}
				wrapperStyle={css?.container?.style}
				disabled={resolvedConfig.disabled}
				on:click={handleClick}
				size={resolvedConfig.size}
				color={resolvedConfig.color}
				{loading}
			>
				{#if resolvedConfig.beforeIcon}
					{#key resolvedConfig.beforeIcon}
						<div class="min-w-4" bind:this={beforeIconComponent}></div>
					{/key}
				{/if}
				{#if resolvedConfig.label?.toString() && resolvedConfig.label?.toString()?.length > 0}
					<div>{resolvedConfig.label.toString()}</div>
				{/if}
				{#if resolvedConfig.afterIcon}
					{#key resolvedConfig.afterIcon}
						<div class="min-w-4" bind:this={afterIconComponent}></div>
					{/key}
				{/if}
			</Button>
		{/key}
	</AlignWrapper>
</RunnableWrapper>

{#if resolvedConfig?.confirmationModal?.selected === 'confirmationModal'}
	<Portal name="app-button" target="#app-editor-top-level-drawer">
		<ConfirmationModal
			open={Boolean(confirmedCallback)}
			title={resolvedConfig?.confirmationModal?.configuration?.confirmationModal?.title ?? ''}
			confirmationText={resolvedConfig?.confirmationModal?.configuration?.confirmationModal
				?.confirmationText ?? ''}
			on:canceled={() => {
				confirmedCallback = undefined
			}}
			on:confirmed={() => {
				if (confirmedCallback) {
					confirmedCallback()
				}
				confirmedCallback = undefined
			}}
			{modalId}
			wrapperClass={modalWrapperClass}
		>
			<div class="flex flex-col w-full space-y-4">
				<span>
					{resolvedConfig?.confirmationModal?.configuration?.confirmationModal?.description ?? ''}
				</span>
			</div>
		</ConfirmationModal>
	</Portal>
{/if}
