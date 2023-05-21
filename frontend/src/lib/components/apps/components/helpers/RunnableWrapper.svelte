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

	type SideEffectAction =
		| {
				selected: 'gotoUrl' | 'none' | 'setTab' | 'sendToast' | 'sendErrorToast' | 'errorOverlay'
				configuration: {
					gotoUrl: { url: string | undefined; newTab: boolean | undefined }
					setTab: {
						setTab: { id: string; index: number }[] | undefined
					}
					sendToast?: {
						message: string | undefined
					}
					sendErrorToast?: {
						message: string | undefined
						appendError: boolean | undefined
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
	export let outputs: { result: Output<any>; loading: Output<boolean> }
	export let extraKey: string | undefined = undefined
	export let refreshOnStart: boolean = false
	export let errorHandledByComponent: boolean = false

	const { staticExporter, noBackend, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	if (noBackend && componentInput?.type == 'runnable') {
		result = componentInput?.['value']
	}
	onMount(() => {
		$staticExporter[id] = () => {
			return result
		}
	})

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

	export function handleSideEffect(success: boolean, errorMessage?: string) {
		const sideEffect = success ? doOnSuccess : doOnError

		if (recomputeIds && success) {
			recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb())
		}
		if (!sideEffect) return

		if (sideEffect.selected == 'none') return

		if (sideEffect.selected == 'setTab') {
			if (Array.isArray(sideEffect.configuration.setTab.setTab)) {
				sideEffect.configuration.setTab?.setTab?.forEach((tab) => {
					if (tab) {
						const { id, index } = tab
						$componentControl[id].setTab?.(index)
					}
				})
			}
		} else if (
			sideEffect.selected == 'gotoUrl' &&
			sideEffect.configuration.gotoUrl.url &&
			sideEffect.configuration.gotoUrl.url != ''
		) {
			if (sideEffect.configuration.gotoUrl.newTab) {
				window.open(sideEffect.configuration.gotoUrl.url, '_blank')
			} else {
				window.location.href = sideEffect.configuration.gotoUrl.url
			}
		} else if (
			sideEffect.selected == 'sendToast' &&
			sideEffect.configuration.sendToast &&
			sideEffect.configuration.sendToast.message &&
			sideEffect.configuration.sendToast.message != ''
		) {
			sendUserToast(sideEffect.configuration.sendToast.message, !success)
		} else if (
			sideEffect.selected == 'sendErrorToast' &&
			sideEffect.configuration.sendErrorToast &&
			sideEffect.configuration.sendErrorToast.message &&
			sideEffect.configuration.sendErrorToast.message != ''
		) {
			sendUserToast(
				sideEffect.configuration.sendErrorToast.message,
				true,
				[],
				sideEffect.configuration.sendErrorToast.appendError ? errorMessage : undefined
			)
		}
	}
</script>

{#if componentInput === undefined}
	<InitializeComponent {id} />
	<slot />
{:else if componentInput.type === 'runnable' && isRunnableDefined(componentInput)}
	<RunnableComponent
		{refreshOnStart}
		{extraKey}
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
		on:done={() => (initializing = false)}
		on:success={() => handleSideEffect(true)}
		on:handleError={(e) => handleSideEffect(false, e.detail)}
		{outputs}
		{errorHandledByComponent}
	>
		<slot />
	</RunnableComponent>
{:else}
	<NonRunnableComponent
		{render}
		bind:result
		{id}
		{componentInput}
		on:done={() => (initializing = false)}
	>
		<slot />
	</NonRunnableComponent>
{/if}
