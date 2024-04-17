<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import { isScriptByNameDefined, isScriptByPathDefined } from '../../utils'
	import NonRunnableComponent from './NonRunnableComponent.svelte'
	import RunnableComponent from './RunnableComponent.svelte'
	import { sendUserToast } from '$lib/toast'
	import InitializeComponent from './InitializeComponent.svelte'

	export let componentInput: AppInput | undefined
	export let noInitialize = false

	type SideEffectAction =
		| {
				selected:
					| 'gotoUrl'
					| 'none'
					| 'setTab'
					| 'sendToast'
					| 'sendErrorToast'
					| 'errorOverlay'
					| 'openModal'
					| 'closeModal'
					| 'open'
					| 'close'
					| 'clearFiles'
				configuration: {
					gotoUrl: { url: (() => string) | string | undefined; newTab: boolean | undefined }
					setTab: {
						setTab:
							| (() => { id: string; index: number }[])
							| { id: string; index: number }[]
							| undefined
					}
					sendToast?: {
						message: (() => string) | string | undefined
					}
					sendErrorToast?: {
						message: (() => string) | string | undefined
						appendError: boolean | undefined
					}
					openModal?: {
						modalId: string | undefined
					}
					closeModal?: {
						modalId: string | undefined
					}
					open?: {
						id: string | undefined
					}
					close?: {
						id: string | undefined
					}
					clearFiles?: {
						id: string | undefined
					}
				}
		  }
		| undefined

	export let id: string
	export let result: any = undefined
	export let initializing: boolean = true
	export let loading: boolean = false
	export let extraQueryParams: Record<string, any> = {}
	export let autoRefresh: boolean = true
	export let runnableComponent: RunnableComponent | undefined = undefined
	export let forceSchemaDisplay: boolean = false
	export let runnableClass = ''
	export let runnableStyle = ''
	export let doOnSuccess: SideEffectAction = undefined
	export let doOnError: SideEffectAction = undefined

	export let render: boolean
	export let recomputeIds: string[] = []
	export let outputs: {
		result: Output<any>
		loading: Output<boolean>
		jobId?: Output<any> | undefined
	}
	export let extraKey: string | undefined = undefined
	export let refreshOnStart: boolean = false
	export let errorHandledByComponent: boolean = false
	export let hasChildrens: boolean = false
	export let allowConcurentRequests = false
	export let onSuccess: (result: any) => void = () => {}

	export function setArgs(value: any) {
		runnableComponent?.setArgs(value)
	}

	const { staticExporter, noBackend, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	if (noBackend && componentInput?.type == 'runnable') {
		result = componentInput?.['value']
	}

	if (noBackend) {
		initializing = false
	}

	onMount(() => {
		$staticExporter[id] = () => {
			return result
		}
	})

	if (!(initializing && componentInput?.type === 'runnable' && isRunnableDefined(componentInput))) {
		initializing = false
	}

	// We need to make sure that old apps have correct values. Triggerable (button, form, etc) have both autoRefresh and recomputeOnInputChanged set to false
	$: if (!autoRefresh && componentInput?.type === 'runnable' && componentInput.autoRefresh) {
		componentInput.autoRefresh = false
		componentInput.recomputeOnInputChanged = false
	}

	function isRunnableDefined(componentInput) {
		return (
			(isScriptByNameDefined(componentInput) &&
				componentInput.runnable.inlineScript != undefined) ||
			isScriptByPathDefined(componentInput)
		)
	}

	export async function handleSideEffect(success: boolean, errorMessage?: string) {
		const sideEffect = success ? doOnSuccess : doOnError

		if (recomputeIds && success) {
			recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb.map((cb) => cb()))
		}
		if (!sideEffect) return

		if (sideEffect.selected == 'none') return

		switch (sideEffect.selected) {
			case 'setTab':
				let setTab = sideEffect?.configuration.setTab?.setTab
				if (!setTab) return
				if (typeof setTab === 'function') {
					setTab = await setTab()
				}
				if (Array.isArray(setTab)) {
					setTab.forEach((tab) => {
						if (tab) {
							const { id, index } = tab
							$componentControl[id].setTab?.(index)
						}
					})
				}
				break
			case 'gotoUrl':
				let gotoUrl = sideEffect?.configuration?.gotoUrl?.url

				if (!gotoUrl) return
				if (typeof gotoUrl === 'function') {
					gotoUrl = await gotoUrl()
				}
				const newTab = sideEffect?.configuration?.gotoUrl?.newTab

				if (newTab) {
					window.open(gotoUrl, '_blank')
				} else {
					window.location.href = gotoUrl
				}

				break
			case 'sendToast': {
				let message = sideEffect?.configuration?.sendToast?.message

				if (!message) return
				if (typeof message === 'function') {
					message = await message()
				}
				sendUserToast(message, !success)
				break
			}
			case 'sendErrorToast': {
				let message = sideEffect?.configuration?.sendErrorToast?.message
				const appendError = sideEffect?.configuration?.sendErrorToast?.appendError

				if (!message) return
				if (typeof message === 'function') {
					message = await message()
				}
				sendUserToast(message, true, [], appendError ? errorMessage : undefined)
				break
			}
			case 'openModal': {
				const modalId = sideEffect?.configuration?.openModal?.modalId
				if (modalId) {
					$componentControl[modalId].openModal?.()
				}
				break
			}
			case 'closeModal': {
				const modalId = sideEffect?.configuration?.closeModal?.modalId

				if (!modalId) return

				$componentControl[modalId].closeModal?.()
				break
			}
			case 'open': {
				const id = sideEffect?.configuration?.open?.id

				if (!id) return

				$componentControl[id].open?.()
				break
			}
			case 'close': {
				const id = sideEffect?.configuration?.close?.id

				if (!id) return

				$componentControl[id].close?.()
				break
			}
			case 'clearFiles': {
				const id = sideEffect?.configuration?.clearFiles?.id

				if (!id) return

				$componentControl[id].clearFiles?.()
				break
			}
			default:
				break
		}
	}
</script>

{#if componentInput === undefined}
	{#if !noInitialize}
		<InitializeComponent {id} />
	{/if}
	<slot />
{:else if componentInput.type === 'runnable' && isRunnableDefined(componentInput)}
	<RunnableComponent
		{noInitialize}
		{allowConcurentRequests}
		{refreshOnStart}
		{extraKey}
		{hasChildrens}
		bind:loading
		bind:this={runnableComponent}
		fields={componentInput.fields}
		bind:result
		runnable={componentInput.runnable}
		hideRefreshButton={componentInput.hideRefreshButton}
		transformer={componentInput.transformer}
		{autoRefresh}
		recomputableByRefreshButton={componentInput.autoRefresh ?? true}
		bind:recomputeOnInputChanged={componentInput.recomputeOnInputChanged}
		{id}
		{extraQueryParams}
		{forceSchemaDisplay}
		{initializing}
		wrapperClass={runnableClass}
		wrapperStyle={runnableStyle}
		{render}
		on:started
		on:done
		on:doneError
		on:cancel
		on:resultSet={() => (initializing = false)}
		on:success={(e) => {
			onSuccess(e.detail)
			handleSideEffect(true)
		}}
		on:handleError={(e) => handleSideEffect(false, e.detail)}
		{outputs}
		{errorHandledByComponent}
	>
		<slot />
	</RunnableComponent>
{:else}
	<NonRunnableComponent {noInitialize} {hasChildrens} {render} bind:result {id} {componentInput}>
		<slot />
	</NonRunnableComponent>
{/if}
